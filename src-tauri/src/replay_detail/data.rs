//! Analysis of a single replay file.

use super::focus::{classify_action, FocusAccumulator};
use crate::data::*;
use crate::replay_rerun::decisions::{
    army_supply, balance_units, base_unit_name, closest_balance_units, format_unit_counts,
    has_cost, is_morph_death, score_unit_choice, unit_cost, UnitCost, WORKERS,
};
use crate::replay_rerun::viewer::GAME_LOOPS_PER_SECOND;
use crate::replay_rerun::{ReplayPlayers, NEUTRAL_PLAYER};
use polars::prelude::*;
use s2protocol::basic_replay_data::SC2ReplayBasicData;
use s2protocol::game_events::{ReplayGameEvent, VersionedBalanceUnits};
use s2protocol::message_events::ReplayMessageEvent;
use s2protocol::state::SC2UnitCmdData;
use s2protocol::tracker_events::ReplayTrackerEvent;
use s2protocol::{SC2EventIterator, SC2EventType, SC2Unit, UnitChangeHint};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

/// The townhalls that count as an expansion when started.
pub const EXPANSION_TOWNHALLS: [&str; 3] = ["Hatchery", "CommandCenter", "Nexus"];

/// The townhalls, and their upgrades, that count as a base.
pub const TOWNHALLS: [&str; 9] = [
    "Hatchery",
    "Lair",
    "Hive",
    "CommandCenter",
    "CommandCenterFlying",
    "OrbitalCommand",
    "OrbitalCommandFlying",
    "PlanetaryFortress",
    "Nexus",
];

/// The structures whose idle time between units is measured.
pub const PRODUCTION_STRUCTURES: [&str; 10] = [
    "Barracks",
    "Factory",
    "Starport",
    "Gateway",
    "RoboticsFacility",
    "Stargate",
    "CommandCenter",
    "OrbitalCommand",
    "PlanetaryFortress",
    "Nexus",
];

/// The structures that, built far from the start location, make a proxy. Townhalls are not
/// included, the natural expansion is far from the start location too.
pub const PROXY_STRUCTURES: [&str; 7] = [
    "Barracks",
    "Factory",
    "Starport",
    "Gateway",
    "RoboticsFacility",
    "Stargate",
    "SpawningPool",
];

/// A proxy is built this far, in map units, from the start location.
pub const PROXY_DISTANCE: f32 = 50.;

/// Proxies are only looked for until this game second.
pub const PROXY_MAX_SECONDS: u32 = 240;

/// A command this close to the enemy start location is scouting it.
pub const SCOUT_DISTANCE: f32 = 30.;

/// Unspent minerals and vespene above this are floating.
pub const FLOATING_THRESHOLD: i32 = 1000;

/// Deaths separated by less than this are part of the same engagement.
pub const ENGAGEMENT_GAP_SECONDS: u32 = 12;

/// Continuous fights are split in engagements of at most this long.
pub const MAX_ENGAGEMENT_SECONDS: u32 = 60;

/// Engagements in which less than this was lost are skirmishes, not shown.
pub const MIN_ENGAGEMENT_RESOURCES: u32 = 400;

/// The minutes at which the economy is compared.
pub const BENCHMARK_MINUTES: [u32; 4] = [4, 6, 8, 10];

/// The army and bases are counted at this game second.
pub const SNAPSHOT_SECONDS: u32 = 600;

fn loop_seconds(game_loop: i64) -> u32 {
    (game_loop.max(0) as f64 / GAME_LOOPS_PER_SECOND) as u32
}

/// The prefixes of the sprays, rewards, etc, they are registered as upgrades but are not part of
/// the build order.
const COSMETIC_UPGRADE_PREFIXES: [&str; 3] = ["Spray", "Reward", "GameHeart"];

/// Sprays, rewards, etc are registered as upgrades but are not part of the build order.
pub fn is_cosmetic_upgrade(name: &str) -> bool {
    COSMETIC_UPGRADE_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// [`is_cosmetic_upgrade`] of the column.
pub fn cosmetic_upgrade_expr(column: &str) -> Expr {
    COSMETIC_UPGRADE_PREFIXES
        .iter()
        .map(|prefix| col(column).str().starts_with(lit(*prefix)))
        .reduce(Expr::or)
        .expect("there are cosmetic upgrade prefixes")
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Counts sorted by the most units first.
fn sorted_counts(counts: BTreeMap<String, usize>) -> Vec<UnitCount> {
    let mut counts: Vec<UnitCount> = counts
        .into_iter()
        .map(|(unit, count)| UnitCount { unit, count })
        .collect();
    counts.sort_by(|a, b| b.count.cmp(&a.count).then(a.unit.cmp(&b.unit)));
    counts
}

/// The units that killed at least one enemy unit, most kills first.
pub fn unit_effectiveness(
    produced: BTreeMap<String, usize>,
    kills: &BTreeMap<String, (usize, u32)>,
    balance: &VersionedBalanceUnits,
) -> Vec<UnitEffectiveness> {
    let mut units: Vec<UnitEffectiveness> = produced
        .into_iter()
        .filter_map(|(unit, produced)| {
            let (kills, value_killed) = *kills.get(&unit)?;
            let cost = unit_cost(balance, &unit);
            Some(UnitEffectiveness {
                unit_cost: cost.minerals + cost.vespene,
                unit,
                produced,
                kills,
                value_killed,
            })
        })
        .collect();
    units.sort_by(|a, b| b.kills.cmp(&a.kills).then(a.unit.cmp(&b.unit)));
    units
}

fn resources(cost: &UnitCost) -> Resources {
    Resources {
        minerals: cost.minerals,
        vespene: cost.vespene,
    }
}

/// A unit or structure that died, for the engagements.
struct Death {
    seconds: u32,
    owner: String,
    name: String,
    cost: UnitCost,
    mining_seconds: f32,
}

#[derive(Default)]
struct PlayerAccumulator {
    supply: i32,
    supply_made: i32,
    /// Minerals and vespene per minute at the last stats update.
    income_per_minute: i32,
    army_value: i32,
    last_stats_seconds: Option<u32>,
    unspent_samples: Vec<i32>,
    start: Option<(f32, f32)>,
    build_order: Vec<BuildOrderEntry>,
    expansions: Vec<BuildOrderEntry>,
    killed: BTreeMap<String, usize>,
    lost: BTreeMap<String, usize>,
    killed_by: BTreeMap<String, usize>,
    bases_lost: Vec<TimedEvent>,
    /// The units, including the initial ones, by type.
    produced: BTreeMap<String, usize>,
    /// The enemy units killed by each type of unit of the player, and their value.
    kills_by_unit: BTreeMap<String, (usize, u32)>,
    resources: ResourceSummary,
    /// The mining seconds of a single unit can be a fraction of a second.
    mining_seconds_lost: f32,
    macro_summary: MacroSummary,
    focus: FocusAccumulator,
    /// The last time each production structure, by tag, finished a unit.
    production_finished: HashMap<u32, (String, u32)>,
    production_idle: BTreeMap<String, (usize, u32)>,
    benchmarks: Vec<Benchmark>,
    army_at_10: Vec<UnitCount>,
    bases_at_10: usize,
    first_kill_seconds: Option<u32>,
    first_scout: Option<TimedEvent>,
    proxy: Option<TimedEvent>,
    was_behind: bool,
    first_upgrade: Option<String>,
    good_decisions: usize,
    even_decisions: usize,
    bad_decisions: Vec<UnitDecision>,
    /// The commands of the abilities with energy or a cooldown, by [`TrackedAbility::ability`].
    abilities: BTreeMap<&'static str, u32>,
}

/// The analysis of a replay file does not change, it is kept for the Player view which analyses
/// all the games of a player.
fn analysis_cache() -> &'static Mutex<HashMap<String, Arc<ReplayDetail>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<ReplayDetail>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Analyses the replay file, or returns the analysis of a previous request.
pub fn analyze_replay_cached(replay_location: &str) -> Result<Arc<ReplayDetail>, SwarmyError> {
    if let Some(detail) = analysis_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(replay_location).cloned())
    {
        return Ok(detail);
    }
    let detail = Arc::new(analyze_replay_file(replay_location)?);
    if let Ok(mut cache) = analysis_cache().lock() {
        cache.insert(replay_location.to_string(), detail.clone());
    }
    Ok(detail)
}

/// Looks up the replay file of the snapshot id and analyses it.
pub fn try_query_replay_detail(
    replay_path: String,
    ext_fs_id: u64,
) -> Result<ReplayDetail, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    let details = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .filter(col("ext_fs_id").eq(lit(ext_fs_id)))
        .select([
            col("ext_fs_file_name"),
            col("ext_fs_sha256"),
            col("ext_datetime")
                .dt()
                .to_string("%Y-%m-%d %H:%M")
                .alias("replay_date"),
        ])
        .limit(1)
        .collect()?;
    let replay_location = details
        .column("ext_fs_file_name")?
        .str()?
        .get(0)
        .ok_or_else(|| SwarmyError::Other(format!("Replay {ext_fs_id} not in the snapshot")))?
        .to_string();
    let replay_date = str_values(&details, "replay_date")?.remove(0);
    let sha256 = str_values(&details, "ext_fs_sha256")?.remove(0);
    let mut detail = (*analyze_replay_cached(&replay_location)?).clone();
    detail.ext_fs_id = ext_fs_id;
    detail.sha256 = sha256;
    detail.replay_date = replay_date;
    // Without the averages the replay is still shown.
    match super::patch_timings::patch_timings_cached(&ipc_path, detail.protocol_version) {
        Ok(timings) => super::patch_timings::annotate(&mut detail, &timings),
        Err(err) => tracing::warn!("No patch timings for {}: {err:?}", detail.protocol_version),
    }
    match crate::ability_usage::ability_usage_cached(&ipc_path) {
        Ok(usage) => crate::ability_usage::annotate(&mut detail, &usage),
        Err(err) => tracing::warn!("No ability usage: {err:?}"),
    }
    Ok(detail)
}

/// Collects the build orders, kills, engagements, macro, decisions and messages of the replay.
pub fn analyze_replay_file(replay_location: &str) -> Result<ReplayDetail, SwarmyError> {
    let source = PathBuf::from(replay_location);
    let basic_data = SC2ReplayBasicData::new(&source, 0)?;
    let protocol_version = basic_data.init_data.version;
    let balance = closest_balance_units(protocol_version)?;
    let mut event_iterator = SC2EventIterator::new(&basic_data, balance_units()?)?;
    let players = ReplayPlayers::from_state(&event_iterator.sc2_state);
    let link_name = |player: &str| players.raw_name(player).unwrap_or(player).to_string();

    let mut accumulators: HashMap<String, PlayerAccumulator> = HashMap::new();
    let mut deaths: Vec<Death> = vec![];
    // The base unit name of the alive units, morphs such as burrowing are not new units.
    let mut registered_units: HashMap<u32, String> = HashMap::new();
    let mut snapshot_taken = false;
    let mut last_loop = 0;
    while let Some(item) = event_iterator.next() {
        let state = &event_iterator.sc2_state;
        let (game_loop, tracker_event) = match &item.event_type {
            SC2EventType::Tracker {
                tracker_loop,
                event,
            } => (*tracker_loop, Some(event)),
            SC2EventType::Game {
                game_loop,
                player_name,
                event,
                ..
            } => {
                let player = match player_name {
                    Some(name) => clean_player_name_clan_escapes(name),
                    None => continue,
                };
                let seconds = loop_seconds(*game_loop);
                if let ReplayGameEvent::Cmd(cmd) = event {
                    let acc = accumulators.entry(player.clone()).or_default();
                    let minute = (seconds / 60) as usize;
                    let per_minute = &mut acc.macro_summary.commands_per_minute;
                    if per_minute.len() <= minute {
                        per_minute.resize(minute + 1, 0);
                    }
                    per_minute[minute] += 1;
                    // s2protocol names the ability in the command of the change hint, the
                    // command of the event only has its ids.
                    let command = match &item.change_hint {
                        UnitChangeHint::Abilities { event, .. } => event,
                        _ => cmd,
                    };
                    let ability = command
                        .m_abil
                        .as_ref()
                        .map(|abil| abil.ability.as_str())
                        .unwrap_or_default();
                    if ability.starts_with("SpawnLarva") {
                        acc.macro_summary.injects += 1;
                    } else if ability.starts_with("ChronoBoost") {
                        acc.macro_summary.chrono_boosts += 1;
                    } else if ability.starts_with("CalldownMULE") {
                        acc.macro_summary.mules += 1;
                    }
                    if let Some(tracked) = tracked_ability(ability) {
                        *acc.abilities.entry(tracked.ability).or_default() += 1;
                    }
                }
                let focus = match &item.change_hint {
                    UnitChangeHint::Abilities { units, event, .. } => {
                        let ability = event
                            .m_abil
                            .as_ref()
                            .map(|abil| abil.ability.as_str())
                            .unwrap_or_default();
                        classify_action(units, ability, balance)
                    }
                    UnitChangeHint::Selection(units) => classify_action(units, "", balance),
                    _ => None,
                };
                if let Some(kind) = focus {
                    accumulators
                        .entry(player.clone())
                        .or_default()
                        .focus
                        .add(seconds, kind);
                }
                register_scout(
                    &players,
                    &mut accumulators,
                    &player,
                    seconds,
                    &item.change_hint,
                );
                continue;
            }
        };
        let Some(event) = tracker_event else {
            continue;
        };
        last_loop = game_loop;
        let seconds = loop_seconds(game_loop);
        if !snapshot_taken && seconds >= SNAPSHOT_SECONDS {
            snapshot_taken = true;
            take_snapshot(&players, &mut accumulators, &state.units, balance);
        }
        match (event, &item.change_hint) {
            (ReplayTrackerEvent::PlayerStats(stats), _) => {
                let player = players.tracker_player_name(stats.player_id);
                let stats = &stats.stats;
                let acc = accumulators.entry(player.clone()).or_default();
                let interval = acc
                    .last_stats_seconds
                    .map(|last| seconds.saturating_sub(last))
                    .unwrap_or_default();
                acc.last_stats_seconds = Some(seconds);
                acc.supply = stats.food_used;
                acc.supply_made = stats.food_made;
                acc.income_per_minute =
                    stats.minerals_collection_rate + stats.vespene_collection_rate;
                acc.army_value = stats.minerals_used_current_army + stats.vespene_used_current_army;
                if stats.food_made > 0
                    && stats.food_used >= stats.food_made
                    && stats.food_made < 200
                {
                    acc.macro_summary.supply_blocked_seconds += interval;
                }
                let unspent = stats.minerals_current + stats.vespene_current;
                // The first minute is spent saving for the first structures.
                if seconds >= 60 {
                    acc.unspent_samples.push(unspent);
                    if unspent > FLOATING_THRESHOLD {
                        acc.macro_summary.floating_seconds += interval;
                    }
                }
                for minute in BENCHMARK_MINUTES {
                    if seconds >= minute * 60
                        && !acc
                            .benchmarks
                            .iter()
                            .any(|benchmark| benchmark.minute == minute)
                    {
                        acc.benchmarks.push(Benchmark {
                            minute,
                            workers: stats.workers_active_count,
                            supply: stats.food_used,
                            minerals_rate: stats.minerals_collection_rate,
                            vespene_rate: stats.vespene_collection_rate,
                            army_value: acc.army_value,
                        });
                    }
                }
                if seconds >= 300 {
                    let army_value = acc.army_value;
                    let enemy_army = accumulators
                        .iter()
                        .filter(|(other, _)| players.is_enemy(&player, other))
                        .map(|(_, other)| other.army_value)
                        .max()
                        .unwrap_or_default();
                    if enemy_army >= 1000 && (army_value as f32) < enemy_army as f32 * 0.7 {
                        accumulators.entry(player).or_default().was_behind = true;
                    }
                }
            }
            (_, UnitChangeHint::Registered { unit, creator }) => {
                let player = players.unit_player(unit);
                let name = base_unit_name(&unit.name).to_string();
                if player == NEUTRAL_PLAYER || registered_units.get(&unit.tag_index) == Some(&name)
                {
                    continue;
                }
                registered_units.insert(unit.tag_index, name.clone());
                let acc = accumulators.entry(player.clone()).or_default();
                if game_loop == 0 && EXPANSION_TOWNHALLS.contains(&name.as_str()) {
                    acc.start = Some((unit.pos.x(), unit.pos.y()));
                }
                // Larva, eggs, etc have no cost.
                if !has_cost(balance, &name) {
                    continue;
                }
                *acc.produced.entry(name.clone()).or_default() += 1;
                // The units at the start of the game are not part of the build order.
                if game_loop == 0 {
                    continue;
                }
                if let Some(creator) = creator {
                    register_production(acc, creator, &name, seconds, balance);
                }
                if acc.proxy.is_none()
                    && seconds <= PROXY_MAX_SECONDS
                    && PROXY_STRUCTURES.contains(&name.as_str())
                {
                    if let Some(start) = acc.start {
                        let away = distance(start, (unit.pos.x(), unit.pos.y()));
                        if away > PROXY_DISTANCE {
                            acc.proxy = Some(TimedEvent {
                                seconds,
                                player: link_name(&player),
                                text: format!("{name} {away:.0} away from the start location"),
                            });
                        }
                    }
                }
                let enemy_units = state
                    .units
                    .values()
                    .filter(|enemy| players.is_enemy(&player, &players.unit_player(enemy)));
                let choice = score_unit_choice(balance, &name, enemy_units);
                let acc = accumulators.entry(player).or_default();
                let entry = BuildOrderEntry {
                    seconds,
                    supply: acc.supply,
                    name: name.clone(),
                    is_worker: WORKERS.contains(&name.as_str()),
                    is_upgrade: false,
                    patch_average: None,
                };
                if EXPANSION_TOWNHALLS.contains(&name.as_str()) {
                    acc.expansions.push(entry.clone());
                }
                acc.build_order.push(entry);
                match choice {
                    Some(choice) if choice.is_bad() => acc.bad_decisions.push(UnitDecision {
                        seconds,
                        unit: name,
                        score: choice.score,
                        strong_vs: format_unit_counts(&choice.strong_vs),
                        weak_vs: format_unit_counts(&choice.weak_vs),
                    }),
                    Some(choice) if choice.is_good() => acc.good_decisions += 1,
                    Some(_) => acc.even_decisions += 1,
                    None => {}
                }
            }
            (ReplayTrackerEvent::UnitDied(_), UnitChangeHint::Unregistered { killer, killed }) => {
                registered_units.remove(&killed.tag_index);
                let owner = players.unit_player(killed);
                let name = base_unit_name(&killed.name).to_string();
                if owner == NEUTRAL_PLAYER
                    || !has_cost(balance, &name)
                    || is_morph_death(&name, killer.is_some())
                {
                    continue;
                }
                let cost = unit_cost(balance, &name);
                let owner_acc = accumulators.entry(owner.clone()).or_default();
                owner_acc.production_finished.remove(&killed.tag_index);
                *owner_acc.lost.entry(name.clone()).or_default() += 1;
                owner_acc.resources.lost.add(resources(&cost));
                owner_acc.resources.build_seconds_lost += cost.time as u32;
                // The income is per minute.
                let mining_seconds = if owner_acc.income_per_minute > 0 {
                    (cost.minerals + cost.vespene) as f32 * 60. / owner_acc.income_per_minute as f32
                } else {
                    0.
                };
                owner_acc.mining_seconds_lost += mining_seconds;
                deaths.push(Death {
                    seconds,
                    owner: owner.clone(),
                    name: name.clone(),
                    cost,
                    mining_seconds,
                });
                let killer_player = killer.as_ref().map(|killer| players.unit_player(killer));
                if let (Some(killer), Some(killer_player)) = (killer, &killer_player) {
                    if players.is_enemy(&owner, killer_player) {
                        let killer_name = base_unit_name(&killer.name).to_string();
                        *accumulators
                            .entry(owner.clone())
                            .or_default()
                            .killed_by
                            .entry(killer_name.clone())
                            .or_default() += 1;
                        let acc = accumulators.entry(killer_player.clone()).or_default();
                        *acc.killed.entry(name.clone()).or_default() += 1;
                        acc.resources.killed.add(resources(&cost));
                        let unit_kills = acc.kills_by_unit.entry(killer_name).or_default();
                        unit_kills.0 += 1;
                        unit_kills.1 += cost.minerals + cost.vespene;
                        acc.first_kill_seconds.get_or_insert(seconds);
                    }
                }
                if TOWNHALLS.contains(&name.as_str()) {
                    let text = match (killer, &killer_player) {
                        (Some(killer), Some(killer_player)) => {
                            format!("{name} killed by {killer_player}'s {}", killer.name)
                        }
                        _ => format!("{name} destroyed"),
                    };
                    accumulators
                        .entry(owner.clone())
                        .or_default()
                        .bases_lost
                        .push(TimedEvent {
                            seconds,
                            player: link_name(&owner),
                            text,
                        });
                }
            }
            (ReplayTrackerEvent::Upgrade(upgrade), _)
                if game_loop > 0 && !is_cosmetic_upgrade(&upgrade.upgrade_type_name) =>
            {
                let player = match &upgrade.player_name {
                    Some(name) => clean_player_name_clan_escapes(name),
                    None => players.tracker_player_name(upgrade.player_id),
                };
                let acc = accumulators.entry(player).or_default();
                acc.first_upgrade
                    .get_or_insert_with(|| upgrade.upgrade_type_name.clone());
                let supply = acc.supply;
                acc.build_order.push(BuildOrderEntry {
                    seconds,
                    supply,
                    name: upgrade.upgrade_type_name.clone(),
                    is_worker: false,
                    is_upgrade: true,
                    patch_average: None,
                });
            }
            _ => {}
        }
    }
    // The games that end before 10:00 have their army and bases counted at the end.
    let snapshot_seconds = if snapshot_taken {
        SNAPSHOT_SECONDS
    } else {
        take_snapshot(
            &players,
            &mut accumulators,
            &event_iterator.sc2_state.units,
            balance,
        );
        loop_seconds(last_loop)
    };

    let messages = read_chat_messages(&source, &event_iterator, &players)?;
    let engagements = engagements(deaths, &link_name);
    let ratings: HashMap<String, i32> = basic_data
        .init_data
        .sync_lobby_state
        .user_initial_data
        .iter()
        .filter_map(|user| Some((user.name.clone(), user.scaled_rating?)))
        .collect();
    let player_list = &basic_data.details.player_list;
    let summaries = player_list
        .iter()
        .map(|details| {
            let name = clean_player_name_clan_escapes(&details.name);
            let acc = accumulators.remove(&name).unwrap_or_default();
            let color = &details.color;
            let name_without_clan = details.name.rsplit("<sp/>").next().unwrap_or_default();
            let mut macro_summary = acc.macro_summary;
            macro_summary.avg_unspent = match acc.unspent_samples.len() {
                0 => 0,
                samples => (acc.unspent_samples.iter().sum::<i32>() / samples as i32) as u32,
            };
            macro_summary.production_idle = acc
                .production_idle
                .into_iter()
                .map(|(structure, (structures, idle_seconds))| ProductionIdle {
                    structure,
                    structures,
                    idle_seconds,
                })
                .collect();
            ReplayPlayerSummary {
                player: ReplayPlayer {
                    name: details.name.clone(),
                    race: normalize_race(&details.race),
                    result: details.result.clone(),
                    team_id: details.team_id,
                    color: [color.r, color.g, color.b, color.a],
                },
                rating: ratings.get(name_without_clan).copied(),
                opening: opening(&acc.build_order, balance),
                first_army_unit: acc
                    .build_order
                    .iter()
                    .find(|entry| army_supply(balance, &entry.name).is_some())
                    .map(|entry| entry.name.clone()),
                first_upgrade: acc.first_upgrade,
                favourite_army_unit: acc
                    .produced
                    .iter()
                    .filter(|(unit, _)| army_supply(balance, unit).is_some())
                    .max_by_key(|(_, count)| **count)
                    .map(|(unit, _)| unit.clone()),
                build_order: acc.build_order,
                expansions: acc.expansions,
                bases_lost: acc.bases_lost,
                units_killed: sorted_counts(acc.killed),
                units_lost: sorted_counts(acc.lost),
                killed_by: sorted_counts(acc.killed_by),
                units: unit_effectiveness(acc.produced, &acc.kills_by_unit, balance),
                resources: ResourceSummary {
                    mining_seconds_lost: acc.mining_seconds_lost.round() as u32,
                    ..acc.resources
                },
                macro_summary,
                focus: acc.focus.summary(loop_seconds(last_loop)),
                benchmarks: acc.benchmarks,
                army_at_10: acc.army_at_10,
                bases_at_10: acc.bases_at_10,
                first_kill_seconds: acc.first_kill_seconds,
                first_scout: acc.first_scout,
                proxy: acc.proxy,
                was_behind: acc.was_behind,
                good_decisions: acc.good_decisions,
                even_decisions: acc.even_decisions,
                bad_decisions: acc.bad_decisions,
                // Compared to the games of the patch once the snapshot is known.
                abilities: acc
                    .abilities
                    .into_iter()
                    .map(|(ability, uses)| AbilityUse {
                        ability: ability.to_string(),
                        uses,
                        ..Default::default()
                    })
                    .collect(),
            }
        })
        .collect();
    Ok(ReplayDetail {
        map_title: basic_data.details.title.clone(),
        replay_location: replay_location.to_string(),
        protocol_version,
        duration_seconds: loop_seconds(last_loop),
        snapshot_seconds,
        winners: player_list
            .iter()
            .filter(|player| player.result == "Win")
            .map(|player| player.name.clone())
            .collect(),
        players: summaries,
        engagements,
        messages,
        ..Default::default()
    })
}

/// The first tech structures, supply, gas, townhalls and defenses are not tech.
fn opening(build_order: &[BuildOrderEntry], balance: &VersionedBalanceUnits) -> Vec<String> {
    const NOT_TECH: [&str; 15] = [
        "SupplyDepot",
        "Pylon",
        "Refinery",
        "Assimilator",
        "Extractor",
        "Hatchery",
        "CommandCenter",
        "Nexus",
        "Bunker",
        "MissileTurret",
        "PhotonCannon",
        "ShieldBattery",
        "SpineCrawler",
        "SporeCrawler",
        "CreepTumor",
    ];
    let mut opening: Vec<String> = vec![];
    for entry in build_order {
        let is_structure = balance.get(&entry.name).is_some_and(|unit| {
            unit.attributes
                .inner
                .iter()
                .any(|attribute| attribute.kind.as_deref() == Some("Structure"))
        });
        if is_structure
            && !NOT_TECH.contains(&entry.name.as_str())
            && !opening.contains(&entry.name)
        {
            opening.push(entry.name.clone());
            if opening.len() == 3 {
                break;
            }
        }
    }
    opening
}

/// Adds the idle time of the production structure since it finished its previous unit.
fn register_production(
    acc: &mut PlayerAccumulator,
    creator: &SC2Unit,
    unit_name: &str,
    seconds: u32,
    balance: &VersionedBalanceUnits,
) {
    let structure = base_unit_name(&creator.name).to_string();
    if !PRODUCTION_STRUCTURES.contains(&structure.as_str()) {
        return;
    }
    let started = seconds.saturating_sub(unit_cost(balance, unit_name).time as u32);
    let previous = acc
        .production_finished
        .insert(creator.tag_index, (structure.clone(), seconds));
    let idle = acc.production_idle.entry(structure).or_default();
    match previous {
        // Reactors produce two units at once, the overlap is not idle time.
        Some((_, finished)) => idle.1 += started.saturating_sub(finished),
        None => idle.0 += 1,
    }
}

/// The first command of a player targeting the enemy start location.
fn register_scout(
    players: &ReplayPlayers,
    accumulators: &mut HashMap<String, PlayerAccumulator>,
    player: &str,
    seconds: u32,
    change_hint: &UnitChangeHint,
) {
    let UnitChangeHint::TargetPoints(units) = change_hint else {
        return;
    };
    if accumulators
        .get(player)
        .is_some_and(|acc| acc.first_scout.is_some())
    {
        return;
    }
    let enemy_starts: Vec<(f32, f32)> = accumulators
        .iter()
        .filter(|(other, _)| players.is_enemy(player, other))
        .filter_map(|(_, other)| other.start)
        .collect();
    for unit in units {
        let SC2UnitCmdData::TargetPoint(target) = &unit.cmd.data else {
            continue;
        };
        // s2protocol negates the y of game event coordinates.
        let target = (target.x(), -target.y());
        if enemy_starts
            .iter()
            .any(|start| distance(*start, target) < SCOUT_DISTANCE)
        {
            accumulators
                .entry(player.to_string())
                .or_default()
                .first_scout = Some(TimedEvent {
                seconds,
                player: players.raw_name(player).unwrap_or(player).to_string(),
                text: unit.name.clone(),
            });
            return;
        }
    }
}

/// The army units and townhalls alive at `SNAPSHOT_SECONDS`, or at the end of shorter games.
fn take_snapshot(
    players: &ReplayPlayers,
    accumulators: &mut HashMap<String, PlayerAccumulator>,
    units: &HashMap<u32, SC2Unit>,
    balance: &VersionedBalanceUnits,
) {
    let mut army: HashMap<String, BTreeMap<String, usize>> = HashMap::new();
    for unit in units.values() {
        let player = players.unit_player(unit);
        if player == NEUTRAL_PLAYER {
            continue;
        }
        let name = base_unit_name(&unit.name);
        if TOWNHALLS.contains(&name) {
            accumulators.entry(player.clone()).or_default().bases_at_10 += 1;
        } else if army_supply(balance, name).is_some() {
            *army
                .entry(player)
                .or_default()
                .entry(name.to_string())
                .or_default() += 1;
        }
    }
    for (player, counts) in army {
        accumulators.entry(player).or_default().army_at_10 = sorted_counts(counts);
    }
}

/// Groups the deaths close in time into engagements.
fn engagements(deaths: Vec<Death>, link_name: &dyn Fn(&str) -> String) -> Vec<Engagement> {
    let mut groups: Vec<Vec<Death>> = vec![];
    for death in deaths {
        match groups.last_mut() {
            Some(group)
                if death.seconds
                    <= group.last().map(|last| last.seconds).unwrap_or_default()
                        + ENGAGEMENT_GAP_SECONDS
                    && death.seconds
                        <= group.first().map(|first| first.seconds).unwrap_or_default()
                            + MAX_ENGAGEMENT_SECONDS =>
            {
                group.push(death)
            }
            _ => groups.push(vec![death]),
        }
    }
    groups
        .into_iter()
        .filter_map(|group| {
            let total: u32 = group
                .iter()
                .map(|death| death.cost.minerals + death.cost.vespene)
                .sum();
            if total < MIN_ENGAGEMENT_RESOURCES {
                return None;
            }
            let start_seconds = group.first()?.seconds;
            let end_seconds = group.last()?.seconds;
            let mut losses: BTreeMap<String, (EngagementLoss, BTreeMap<String, usize>, f32)> =
                BTreeMap::new();
            for death in group {
                let (loss, units, mining_seconds) =
                    losses.entry(death.owner.clone()).or_insert_with(|| {
                        (
                            EngagementLoss {
                                player: link_name(&death.owner),
                                ..Default::default()
                            },
                            BTreeMap::new(),
                            0.,
                        )
                    });
                loss.resources.add(resources(&death.cost));
                loss.supply += death.cost.supply;
                *mining_seconds += death.mining_seconds;
                *units.entry(death.name).or_default() += 1;
            }
            Some(Engagement {
                start_seconds,
                end_seconds,
                losses: losses
                    .into_values()
                    .map(|(mut loss, units, mining_seconds)| {
                        loss.units = format_unit_counts(&units);
                        loss.mining_seconds = mining_seconds.round() as u32;
                        loss
                    })
                    .collect(),
            })
        })
        .collect()
}

/// The chat messages are stored apart from the tracker and game events.
fn read_chat_messages(
    source: &PathBuf,
    event_iterator: &SC2EventIterator,
    players: &ReplayPlayers,
) -> Result<Vec<TimedEvent>, SwarmyError> {
    let file_contents = s2protocol::read_file(source)?;
    let (_, mpq) = s2protocol::parser::parse(&file_contents)
        .map_err(|e| SwarmyError::Other(format!("Unable to parse {}: {e:?}", source.display())))?;
    let file_name = source.to_string_lossy();
    let message_events = s2protocol::read_message_events(&file_name, &mpq, &file_contents)?;
    let mut game_loop = 0;
    let mut messages = vec![];
    for message in message_events {
        // The message events store the loops since the previous message.
        game_loop += message.delta;
        let ReplayMessageEvent::EChat(chat) = message.event;
        let sender = event_iterator
            .sc2_state
            .user_state
            .get(&message.user_id)
            .map(|user| user.player_lobby_details.player_details.name.clone())
            .unwrap_or_else(|| {
                players
                    .raw_names
                    .get(message.user_id as usize)
                    .cloned()
                    .unwrap_or_else(|| format!("User {}", message.user_id))
            });
        messages.push(TimedEvent {
            seconds: loop_seconds(game_loop),
            player: sender,
            text: chat.m_string,
        });
    }
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn death(seconds: u32, owner: &str, minerals: u32) -> Death {
        Death {
            seconds,
            owner: owner.to_string(),
            name: "Marine".to_string(),
            cost: UnitCost {
                minerals,
                vespene: 0,
                supply: 1.,
                time: 18.,
            },
            mining_seconds: 10.,
        }
    }

    #[test]
    fn it_groups_deaths_into_engagements() {
        let deaths = vec![
            death(100, "A", 300),
            death(105, "B", 200),
            // Too far from the previous death, and too cheap to be an engagement.
            death(200, "A", 50),
        ];
        let engagements = engagements(deaths, &|player: &str| player.to_string());
        assert_eq!(engagements.len(), 1);
        assert_eq!(
            (engagements[0].start_seconds, engagements[0].end_seconds),
            (100, 105)
        );
        assert_eq!(engagements[0].losses.len(), 2);
        assert_eq!(engagements[0].losses[0].resources.minerals, 300);
        assert_eq!(engagements[0].losses[0].units, "1xMarine");
    }
}
