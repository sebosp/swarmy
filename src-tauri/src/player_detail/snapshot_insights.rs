//! The analysis of each game for the Player view, computed from the files of the snapshot of its
//! day, not from the replay: the units registered (`unit_init`, `unit_born`), their deaths
//! (`unit_died`), the stats of the players every 10 seconds (`stats`), the upgrades, the commands
//! (the cmd files and `game_actions`) and the players (`details`).
//!
//! The values follow the definitions of the analysis of a replay (`replay_detail::data`), with the
//! same helpers: the order of the events of a loop and the first scout (here the first command
//! aimed near the enemy start, right clicks included) may differ slightly.

use crate::data::*;
use crate::replay_detail::data::{
    idle_seconds, is_cosmetic_upgrade, is_tech_structure, BENCHMARK_MINUTES, CASTERS,
    EXPANSION_TOWNHALLS, FLOATING_THRESHOLD, NOT_SCOUTS, OPENING_STRUCTURES, PRODUCTION_STRUCTURES,
    PROXY_DISTANCE, PROXY_MAX_SECONDS, PROXY_STRUCTURES, SCOUT_DISTANCE, SNAPSHOT_SECONDS,
    TOWNHALLS,
};
use crate::replay_rerun::decisions::{
    army_supply, base_unit_name, closest_balance_units, has_cost, is_morph_death, unit_cost,
};
use crate::replay_rerun::viewer::GAME_LOOPS_PER_SECOND;
use polars::prelude::*;
use s2protocol::game_events::VersionedBalanceUnits;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use swarmy_common::*;

/// The players change their clan tags, they are identified by the name without it.
pub fn name_without_clan(player_name: &str) -> &str {
    player_name.rsplit("<sp/>").next().unwrap_or(player_name)
}

fn loop_seconds(game_loop: i64) -> u32 {
    (game_loop.max(0) as f64 / GAME_LOOPS_PER_SECOND) as u32
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn sorted_counts(counts: BTreeMap<String, usize>) -> Vec<UnitCount> {
    let mut counts: Vec<UnitCount> = counts
        .into_iter()
        .map(|(unit, count)| UnitCount { unit, count })
        .collect();
    counts.sort_by(|a, b| b.count.cmp(&a.count).then(a.unit.cmp(&b.unit)));
    counts
}

/// Reads the columns of a file of the day.
fn read_day_file(day: &Path, file: &str, columns: &[&str]) -> Result<DataFrame, SwarmyError> {
    let path = day.join(file);
    if !path.is_file() {
        return Err(SwarmyError::Other(format!(
            "{} is missing, the snapshot was written by an older s2protocol, Rebuild it",
            path.display()
        )));
    }
    Ok(LazyFrame::scan_ipc(
        PlRefPath::new(path.to_string_lossy().as_ref()),
        Default::default(),
        UnifiedScanArgs {
            hive_options: HiveOptions::new_disabled(),
            ..Default::default()
        },
    )?
    .select(columns.iter().map(|name| col(*name)).collect::<Vec<_>>())
    .collect()?)
}

fn opt_str_values(df: &DataFrame, name: &str) -> Result<Vec<Option<String>>, SwarmyError> {
    Ok(df
        .column(name)?
        .str()?
        .iter()
        .map(|value| value.map(str::to_string))
        .collect())
}

fn opt_u32_values(df: &DataFrame, name: &str) -> Result<Vec<Option<u32>>, SwarmyError> {
    Ok(df.column(name)?.u32()?.iter().collect())
}

fn i32_values(df: &DataFrame, name: &str) -> Result<Vec<i32>, SwarmyError> {
    Ok(df
        .column(name)?
        .i32()?
        .iter()
        .map(Option::unwrap_or_default)
        .collect())
}

/// A player of a game.
#[derive(Debug, Clone, Default)]
struct Player {
    /// The name without the clan tag.
    name: String,
    race: String,
    result: String,
    team_id: u8,
    rating: Option<i32>,
    /// The player id of the tracker events, the `control_player_id` of its units.
    tracker_id: Option<u8>,
}

/// What happened in a game, in the order of the loops.
#[derive(Debug, Clone)]
enum TrackerItem {
    /// A unit started, born, finished or changed type.
    Registered {
        /// A structure started (`unit_init`), it is not a caster until it is finished.
        init: bool,
        tag: u32,
        name: String,
        owner: Option<u8>,
        x: f32,
        y: f32,
        creator_tag: Option<u32>,
        creator_name: Option<String>,
    },
    Died {
        tag: u32,
        name: String,
        killer_player: Option<u8>,
        killer_name: Option<String>,
    },
    Stats {
        player: u8,
        stats: GameStats,
    },
    Upgrade {
        player: u8,
        name: String,
    },
}

#[derive(Debug, Clone, Default)]
struct GameStats {
    food_used: i32,
    food_made: i32,
    minerals_current: i32,
    vespene_current: i32,
    minerals_rate: i32,
    vespene_rate: i32,
    workers: i32,
    army_value: i32,
}

/// A command of a player.
#[derive(Debug, Clone, Default)]
struct Command {
    game_loop: i64,
    ability: String,
    unit_name: String,
    /// The target point of the commands aimed at a point, in map coordinates.
    target: Option<(f32, f32)>,
}

/// The rows of a game.
#[derive(Debug, Default)]
struct GameRows {
    protocol_version: u32,
    map_title: String,
    details_rows: usize,
    players: Vec<Player>,
    tracker: Vec<(i64, usize, TrackerItem)>,
    /// By player name without the clan tag.
    commands: HashMap<String, Vec<Command>>,
    /// The commands of each minute, by player name without the clan tag.
    commands_per_minute: HashMap<String, BTreeMap<u32, u32>>,
    /// The commands of the game, and the ones with an ability name (the balance data of the
    /// patch names them).
    command_count: usize,
    named_command_count: usize,
}

/// What is counted for a player while the events of the game are read.
#[derive(Debug, Default)]
struct Accumulator {
    start: Option<(f32, f32)>,
    build_order: Vec<String>,
    produced: BTreeMap<String, usize>,
    /// The production structures by tag: their type and when they finished their last unit.
    production_finished: HashMap<u32, (String, u32)>,
    /// By structure type: the structures that produced, and their idle seconds.
    production_idle: BTreeMap<String, (usize, u32)>,
    /// The intervals in which the supply was full, see `replay_detail::data::idle_seconds`.
    supply_full: Vec<(u32, u32)>,
    proxy: bool,
    first_upgrade: Option<String>,
    killed_by: BTreeMap<String, usize>,
    lost: u32,
    /// The units lost by type, to compare with the analysis of the replay in the tests.
    #[cfg(test)]
    lost_units: BTreeMap<String, usize>,
    killed: u32,
    mining_seconds_lost: f32,
    first_kill_seconds: Option<u32>,
    bases_lost: usize,
    army_at_10: Vec<UnitCount>,
    bases_at_10: u32,
    benchmarks: Vec<Benchmark>,
    last_stats_seconds: Option<u32>,
    income_per_minute: i32,
    army_value: i32,
    supply_blocked_seconds: u32,
    floating_seconds: u32,
    unspent_samples: Vec<i32>,
    was_behind: bool,
    /// The seconds the casters (Queens, Nexuses, Orbital Commands, Hatcheries...) were alive and
    /// finished until 10:00, by type.
    caster_seconds: BTreeMap<&'static str, f32>,
}

/// Reads the rows of the games of a day, by game.
fn read_day(day: &Path) -> Result<BTreeMap<u64, GameRows>, SwarmyError> {
    let mut games: BTreeMap<u64, GameRows> = BTreeMap::new();
    let details = read_day_file(
        day,
        DETAILS_IPC,
        &[
            "ext_fs_id",
            "player_name",
            "player_race",
            "player_result",
            "player_team_id",
            "player_observe",
            "player_scaled_rating",
            "title",
            "ext_protocol_version",
        ],
    )?;
    let ratings: Vec<Option<i32>> = details
        .column("player_scaled_rating")?
        .i32()?
        .iter()
        .collect();
    let observe = u8_values(&details, "player_observe")?;
    let teams = u8_values(&details, "player_team_id")?;
    let versions = u32_values(&details, "ext_protocol_version")?;
    let titles = str_values(&details, "title")?;
    let races = str_values(&details, "player_race")?;
    let results = str_values(&details, "player_result")?;
    for (idx, (ext_fs_id, name)) in u64_values(&details, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&details, "player_name")?)
        .enumerate()
    {
        let game = games.entry(ext_fs_id).or_default();
        game.protocol_version = versions[idx];
        game.map_title = titles[idx].clone();
        // The rows of a game are in the order of the players of the replay: the tracker id of a
        // player is its position, from 1. `game_players` tells it for the humans, the computers
        // are not in it.
        game.details_rows += 1;
        if observe[idx].unwrap_or_default() != 0 {
            continue;
        }
        game.players.push(Player {
            name: name_without_clan(&name).to_string(),
            race: normalize_race(&races[idx]),
            result: results[idx].clone(),
            team_id: teams[idx].unwrap_or_default(),
            rating: ratings[idx],
            tracker_id: u8::try_from(game.details_rows).ok(),
        });
    }
    let players = read_day_file(
        day,
        GAME_PLAYERS_IPC,
        &["ext_fs_id", "player_name", "player_id"],
    )?;
    for ((ext_fs_id, name), id) in u64_values(&players, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&players, "player_name")?)
        .zip(u8_values(&players, "player_id")?)
    {
        let name = name_without_clan(&name);
        if let Some(player) = games
            .get_mut(&ext_fs_id)
            .and_then(|game| game.players.iter_mut().find(|player| player.name == name))
        {
            player.tracker_id = id;
        }
    }
    // The order of the files of the events of a loop: the units, their deaths, the stats.
    let mut push = |ext_fs_id: u64, game_loop: i64, order: usize, item: TrackerItem| {
        if let Some(game) = games.get_mut(&ext_fs_id) {
            game.tracker.push((game_loop, order, item));
        }
    };
    let unit_columns = [
        "ext_fs_id",
        "ext_replay_loop",
        "unit_tag_index",
        "unit_type_name",
        "control_player_id",
        "x",
        "y",
        "creator_unit_tag_index",
        "creator_unit_type_name",
    ];
    for (file, init) in [(UNIT_INIT_IPC, true), (UNIT_BORN_IPC, false)] {
        let rows = read_day_file(day, file, &unit_columns)?;
        let ids = u64_values(&rows, "ext_fs_id")?;
        let loops = i64_values(&rows, "ext_replay_loop")?;
        let tags = u32_values(&rows, "unit_tag_index")?;
        let names = str_values(&rows, "unit_type_name")?;
        let owners = u8_values(&rows, "control_player_id")?;
        let xs = f32_values(&rows, "x")?;
        let ys = f32_values(&rows, "y")?;
        let creator_tags = opt_u32_values(&rows, "creator_unit_tag_index")?;
        let creator_names = opt_str_values(&rows, "creator_unit_type_name")?;
        for idx in 0..rows.height() {
            push(
                ids[idx],
                loops[idx],
                0,
                TrackerItem::Registered {
                    init,
                    tag: tags[idx],
                    name: names[idx].clone(),
                    owner: owners[idx],
                    x: xs[idx],
                    y: ys[idx],
                    creator_tag: creator_tags[idx],
                    creator_name: creator_names[idx].clone(),
                },
            );
        }
    }
    let died = read_day_file(
        day,
        UNIT_DIED_IPC,
        &[
            "ext_fs_id",
            "ext_replay_loop",
            "unit_tag_index",
            "unit_died_name",
            "killer_player_id",
            "unit_killer_name",
        ],
    )?;
    {
        let ids = u64_values(&died, "ext_fs_id")?;
        let loops = i64_values(&died, "ext_replay_loop")?;
        let tags = u32_values(&died, "unit_tag_index")?;
        let names = str_values(&died, "unit_died_name")?;
        let killers = u8_values(&died, "killer_player_id")?;
        // A death without a killer has an empty name.
        let killer_names: Vec<Option<String>> = opt_str_values(&died, "unit_killer_name")?
            .into_iter()
            .map(|name| name.filter(|name| !name.is_empty()))
            .collect();
        for idx in 0..died.height() {
            push(
                ids[idx],
                loops[idx],
                1,
                TrackerItem::Died {
                    tag: tags[idx],
                    name: names[idx].clone(),
                    killer_player: killers[idx],
                    killer_name: killer_names[idx].clone(),
                },
            );
        }
    }
    let stats = read_day_file(
        day,
        STATS_IPC,
        &[
            "ext_fs_id",
            "ext_replay_loop",
            "player_id",
            "food_used",
            "food_made",
            "minerals_current",
            "vespene_current",
            "minerals_collection_rate",
            "vespene_collection_rate",
            "workers_active_count",
            "minerals_used_current_army",
            "vespene_used_current_army",
        ],
    )?;
    {
        let ids = u64_values(&stats, "ext_fs_id")?;
        let loops = i64_values(&stats, "ext_replay_loop")?;
        let players = u8_values(&stats, "player_id")?;
        let values = |name: &str| i32_values(&stats, name);
        let (food_used, food_made) = (values("food_used")?, values("food_made")?);
        let (minerals, vespene) = (values("minerals_current")?, values("vespene_current")?);
        let minerals_rate = values("minerals_collection_rate")?;
        let vespene_rate = values("vespene_collection_rate")?;
        let workers = values("workers_active_count")?;
        let army_minerals = values("minerals_used_current_army")?;
        let army_vespene = values("vespene_used_current_army")?;
        for idx in 0..stats.height() {
            let Some(player) = players[idx] else { continue };
            push(
                ids[idx],
                loops[idx],
                2,
                TrackerItem::Stats {
                    player,
                    stats: GameStats {
                        food_used: food_used[idx],
                        food_made: food_made[idx],
                        minerals_current: minerals[idx],
                        vespene_current: vespene[idx],
                        minerals_rate: minerals_rate[idx],
                        vespene_rate: vespene_rate[idx],
                        workers: workers[idx],
                        army_value: army_minerals[idx] + army_vespene[idx],
                    },
                },
            );
        }
    }
    let upgrades = read_day_file(
        day,
        UPGRADES_IPC,
        &["ext_fs_id", "ext_replay_loop", "player_id", "name"],
    )?;
    {
        let ids = u64_values(&upgrades, "ext_fs_id")?;
        let loops = i64_values(&upgrades, "ext_replay_loop")?;
        let players = u8_values(&upgrades, "player_id")?;
        let names = str_values(&upgrades, "name")?;
        for idx in 0..upgrades.height() {
            let Some(player) = players[idx] else { continue };
            push(
                ids[idx],
                loops[idx],
                3,
                TrackerItem::Upgrade {
                    player,
                    name: names[idx].clone(),
                },
            );
        }
    }
    for game in games.values_mut() {
        game.tracker
            .sort_by_key(|(game_loop, order, _)| (*game_loop, *order));
    }
    // The commands, the points in 1/4096 of a map cell.
    for (file, has_point) in [
        (CMD_TARGET_POINT_IPC, true),
        (CMD_TARGET_UNIT_IPC, false),
        (CMD_NO_TARGET_IPC, false),
    ] {
        let mut columns = vec![
            "ext_fs_id",
            "ext_replay_loop",
            "player_name",
            "ability",
            "unit_name",
        ];
        if has_point {
            columns.extend(["target_point_x", "target_point_y"]);
        }
        let rows = read_day_file(day, file, &columns)?;
        let ids = u64_values(&rows, "ext_fs_id")?;
        let loops = i64_values(&rows, "ext_replay_loop")?;
        let names = str_values(&rows, "player_name")?;
        let abilities = str_values(&rows, "ability")?;
        let units = str_values(&rows, "unit_name")?;
        let points = if has_point {
            Some((
                i64_values(&rows, "target_point_x")?,
                i64_values(&rows, "target_point_y")?,
            ))
        } else {
            None
        };
        for idx in 0..rows.height() {
            let Some(game) = games.get_mut(&ids[idx]) else {
                continue;
            };
            game.command_count += 1;
            game.named_command_count += (!abilities[idx].is_empty()) as usize;
            game.commands
                .entry(name_without_clan(&names[idx]).to_string())
                .or_default()
                .push(Command {
                    game_loop: loops[idx],
                    ability: abilities[idx].clone(),
                    unit_name: units[idx].clone(),
                    target: points
                        .as_ref()
                        .map(|(xs, ys)| (xs[idx] as f32 / 4096., ys[idx] as f32 / 4096.)),
                });
        }
    }
    for commands in games
        .values_mut()
        .flat_map(|game| game.commands.values_mut())
    {
        commands.sort_by_key(|command| command.game_loop);
    }
    let actions = read_day_file(
        day,
        GAME_ACTIONS_IPC,
        &["ext_fs_id", "player_name", "minute", "commands"],
    )?;
    for (((ext_fs_id, name), minute), commands) in u64_values(&actions, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&actions, "player_name")?)
        .zip(u32_values(&actions, "minute")?)
        .zip(u32_values(&actions, "commands")?)
    {
        if let Some(game) = games.get_mut(&ext_fs_id) {
            *game
                .commands_per_minute
                .entry(name_without_clan(&name).to_string())
                .or_default()
                .entry(minute)
                .or_default() += commands;
        }
    }
    Ok(games)
}

/// The army units and townhalls alive, by owner.
fn take_snapshot(
    alive: &HashMap<u32, (u8, String)>,
    accumulators: &mut HashMap<u8, Accumulator>,
    balance: &VersionedBalanceUnits,
) {
    let mut army: HashMap<u8, BTreeMap<String, usize>> = HashMap::new();
    for (owner, name) in alive.values() {
        let Some(acc) = accumulators.get_mut(owner) else {
            continue;
        };
        if TOWNHALLS.contains(&name.as_str()) {
            acc.bases_at_10 += 1;
        } else if army_supply(balance, name).is_some() {
            *army
                .entry(*owner)
                .or_default()
                .entry(name.clone())
                .or_default() += 1;
        }
    }
    for (owner, counts) in army {
        if let Some(acc) = accumulators.get_mut(&owner) {
            acc.army_at_10 = sorted_counts(counts);
        }
    }
}

#[cfg(test)]
thread_local! {
    /// The units lost by each player of the last game computed, for the tests.
    static LOST_UNITS: std::cell::RefCell<HashMap<String, BTreeMap<String, usize>>> =
        Default::default();
}

/// The insights of the players of a game.
fn game_insights(game: &GameRows) -> Result<Vec<(String, PlayerGameInsight)>, SwarmyError> {
    let balance = closest_balance_units(game.protocol_version)?;
    let team_of: HashMap<u8, u8> = game
        .players
        .iter()
        .filter_map(|player| Some((player.tracker_id?, player.team_id)))
        .collect();
    let is_enemy = |a: u8, b: u8| match (team_of.get(&a), team_of.get(&b)) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    };
    let mut accumulators: HashMap<u8, Accumulator> = team_of
        .keys()
        .map(|id| (*id, Accumulator::default()))
        .collect();
    // The units alive by tag: their owner and type, and the type registered for the build order.
    let mut alive: HashMap<u32, (u8, String)> = HashMap::new();
    let mut owner_of: HashMap<u32, u8> = HashMap::new();
    let mut registered: HashMap<u32, String> = HashMap::new();
    // The structures started and not finished yet.
    let mut constructing: std::collections::HashSet<u32> = Default::default();
    let mut snapshot_taken = false;
    let mut last_loop = 0;
    for (game_loop, _, item) in &game.tracker {
        let seconds = loop_seconds(*game_loop);
        last_loop = last_loop.max(*game_loop);
        if !snapshot_taken && seconds >= SNAPSHOT_SECONDS {
            snapshot_taken = true;
            take_snapshot(&alive, &mut accumulators, balance);
        }
        match item {
            TrackerItem::Registered {
                init,
                tag,
                name,
                owner,
                x,
                y,
                creator_tag,
                creator_name,
            } => {
                let name = base_unit_name(name).to_string();
                let Some(owner) = owner.filter(|owner| team_of.contains_key(owner)) else {
                    continue;
                };
                alive.insert(*tag, (owner, name.clone()));
                owner_of.insert(*tag, owner);
                if *init {
                    constructing.insert(*tag);
                } else {
                    constructing.remove(tag);
                }
                if registered.get(tag) == Some(&name) {
                    continue;
                }
                registered.insert(*tag, name.clone());
                let Some(acc) = accumulators.get_mut(&owner) else {
                    continue;
                };
                if *game_loop == 0 && EXPANSION_TOWNHALLS.contains(&name.as_str()) {
                    acc.start = Some((*x, *y));
                }
                // Larva, eggs, etc have no cost.
                if !has_cost(balance, &name) {
                    continue;
                }
                *acc.produced.entry(name.clone()).or_default() += 1;
                // The units at the start of the game are not part of the build order.
                if *game_loop == 0 {
                    continue;
                }
                if let (Some(creator_tag), Some(creator_name)) = (creator_tag, creator_name) {
                    let structure = base_unit_name(creator_name).to_string();
                    if PRODUCTION_STRUCTURES.contains(&structure.as_str()) {
                        let started = seconds.saturating_sub(unit_cost(balance, &name).time as u32);
                        let previous = acc
                            .production_finished
                            .insert(*creator_tag, (structure.clone(), seconds));
                        let idle = acc.production_idle.entry(structure).or_default();
                        match previous {
                            // Reactors produce two units at once, the overlap is not idle time.
                            Some((_, finished)) => {
                                idle.1 += idle_seconds(finished, started, &acc.supply_full)
                            }
                            None => idle.0 += 1,
                        }
                    }
                }
                if !acc.proxy
                    && seconds <= PROXY_MAX_SECONDS
                    && PROXY_STRUCTURES.contains(&name.as_str())
                    && acc
                        .start
                        .is_some_and(|start| distance(start, (*x, *y)) > PROXY_DISTANCE)
                {
                    acc.proxy = true;
                }
                acc.build_order.push(name);
            }
            TrackerItem::Died {
                tag,
                name,
                killer_player,
                killer_name,
            } => {
                registered.remove(tag);
                alive.remove(tag);
                constructing.remove(tag);
                let Some(owner) = owner_of.get(tag).copied() else {
                    continue;
                };
                let name = base_unit_name(name).to_string();
                if !has_cost(balance, &name) || is_morph_death(&name, killer_name.is_some()) {
                    continue;
                }
                let cost = unit_cost(balance, &name);
                let value = cost.minerals + cost.vespene;
                let Some(owner_acc) = accumulators.get_mut(&owner) else {
                    continue;
                };
                owner_acc.production_finished.remove(tag);
                owner_acc.lost += value;
                #[cfg(test)]
                {
                    *owner_acc.lost_units.entry(name.clone()).or_default() += 1;
                }
                if owner_acc.income_per_minute > 0 {
                    owner_acc.mining_seconds_lost +=
                        value as f32 * 60. / owner_acc.income_per_minute as f32;
                }
                if TOWNHALLS.contains(&name.as_str()) {
                    owner_acc.bases_lost += 1;
                }
                if let (Some(killer), Some(killer_name)) = (killer_player, killer_name) {
                    if is_enemy(owner, *killer) {
                        *owner_acc
                            .killed_by
                            .entry(base_unit_name(killer_name).to_string())
                            .or_default() += 1;
                        if let Some(killer_acc) = accumulators.get_mut(killer) {
                            killer_acc.killed += value;
                            killer_acc.first_kill_seconds.get_or_insert(seconds);
                        }
                    }
                }
            }
            TrackerItem::Stats { player, stats } => {
                let Some(acc) = accumulators.get_mut(player) else {
                    continue;
                };
                let interval = acc
                    .last_stats_seconds
                    .map(|last| seconds.saturating_sub(last))
                    .unwrap_or_default();
                acc.last_stats_seconds = Some(seconds);
                // The casters alive since the previous stats of the player, until 10:00.
                if seconds <= SNAPSHOT_SECONDS {
                    for (tag, (owner, name)) in &alive {
                        if owner != player || constructing.contains(tag) {
                            continue;
                        }
                        if let Some(caster) = CASTERS.iter().find(|caster| **caster == name) {
                            *acc.caster_seconds.entry(caster).or_default() += interval as f32;
                        }
                    }
                }
                // At 200 or blocked below it, the production cannot start.
                if stats.food_made > 0 && stats.food_used >= stats.food_made && interval > 0 {
                    acc.supply_full.push((seconds - interval, seconds));
                }
                acc.income_per_minute = stats.minerals_rate + stats.vespene_rate;
                acc.army_value = stats.army_value;
                if stats.food_made > 0
                    && stats.food_used >= stats.food_made
                    && stats.food_made < 200
                {
                    acc.supply_blocked_seconds += interval;
                }
                let unspent = stats.minerals_current + stats.vespene_current;
                // The first minute is spent saving for the first structures.
                if seconds >= 60 {
                    acc.unspent_samples.push(unspent);
                    if unspent > FLOATING_THRESHOLD {
                        acc.floating_seconds += interval;
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
                            workers: stats.workers,
                            supply: stats.food_used,
                            minerals_rate: stats.minerals_rate,
                            vespene_rate: stats.vespene_rate,
                            army_value: stats.army_value,
                        });
                    }
                }
                if seconds >= 300 {
                    let army_value = stats.army_value;
                    let enemy_army = accumulators
                        .iter()
                        .filter(|(other, _)| is_enemy(*player, **other))
                        .map(|(_, other)| other.army_value)
                        .max()
                        .unwrap_or_default();
                    if enemy_army >= 1000 && (army_value as f32) < enemy_army as f32 * 0.7 {
                        if let Some(acc) = accumulators.get_mut(player) {
                            acc.was_behind = true;
                        }
                    }
                }
            }
            TrackerItem::Upgrade { player, name } => {
                if *game_loop > 0 && !is_cosmetic_upgrade(name) {
                    if let Some(acc) = accumulators.get_mut(player) {
                        acc.first_upgrade.get_or_insert_with(|| name.clone());
                    }
                }
            }
        }
    }
    // The games that end before 10:00 have their army and bases counted at the end.
    if !snapshot_taken {
        take_snapshot(&alive, &mut accumulators, balance);
    }
    let starts: HashMap<u8, (f32, f32)> = accumulators
        .iter()
        .filter_map(|(id, acc)| Some((*id, acc.start?)))
        .collect();
    let bases_lost: HashMap<u8, usize> = accumulators
        .iter()
        .map(|(id, acc)| (*id, acc.bases_lost))
        .collect();
    let mut insights = vec![];
    for player in &game.players {
        let Some(id) = player.tracker_id else {
            continue;
        };
        let Some(acc) = accumulators.remove(&id) else {
            continue;
        };
        #[cfg(test)]
        LOST_UNITS.with(|lost| {
            lost.borrow_mut()
                .insert(player.name.clone(), acc.lost_units.clone())
        });
        let opponents: Vec<&Player> = game
            .players
            .iter()
            .filter(|other| other.team_id != player.team_id)
            .collect();
        let matchup = match opponents.as_slice() {
            [opponent] => matchup_label(&player.race, &opponent.race),
            _ => "Team game".to_string(),
        };
        let base_trade = acc.bases_lost >= 2
            && opponents.iter().any(|opponent| {
                opponent
                    .tracker_id
                    .and_then(|id| bases_lost.get(&id))
                    .is_some_and(|lost| *lost >= 2)
            });
        let commands = game.commands.get(&player.name);
        let count = |prefix: &str| {
            commands.map_or(0, |commands| {
                commands
                    .iter()
                    .filter(|command| command.ability.starts_with(prefix))
                    .count()
            }) as f32
        };
        let enemy_starts: Vec<(f32, f32)> = starts
            .iter()
            .filter(|(other, _)| is_enemy(id, **other))
            .map(|(_, start)| *start)
            .collect();
        let first_scout = commands.and_then(|commands| {
            commands.iter().find(|command| {
                !NOT_SCOUTS.contains(&base_unit_name(&command.unit_name))
                    && command.target.is_some_and(|target| {
                        enemy_starts
                            .iter()
                            .any(|start| distance(*start, target) < SCOUT_DISTANCE)
                    })
            })
        });
        // The uses until 10:00, compared with the energy of the casters when the commands of the
        // patch have names, as the analysis of a replay does.
        let early = |prefix: &str| {
            commands.map_or(0, |commands| {
                commands
                    .iter()
                    .filter(|command| {
                        loop_seconds(command.game_loop) <= SNAPSHOT_SECONDS
                            && command.ability.starts_with(prefix)
                    })
                    .count()
            }) as u32
        };
        let energy = if game.named_command_count * 2 >= game.command_count {
            crate::replay_detail::lessons::ability_efficiency(
                &acc.caster_seconds,
                &MacroSummary {
                    injects: early("SpawnLarva"),
                    chrono_boosts: early("ChronoBoost"),
                    mules: early("CalldownMULE"),
                    ..Default::default()
                },
            )
        } else {
            vec![]
        };
        let per_minute = game.commands_per_minute.get(&player.name);
        let commands_per_minute = per_minute
            .and_then(|minutes| {
                let last = minutes
                    .iter()
                    .filter(|(_, commands)| **commands > 0)
                    .map(|(minute, _)| *minute)
                    .max()?;
                Some(minutes.values().sum::<u32>() as f32 / (last + 1) as f32)
            })
            .unwrap_or_default();
        let mut opening: Vec<String> = vec![];
        for name in &acc.build_order {
            if is_tech_structure(name, balance) && !opening.contains(name) {
                opening.push(name.clone());
                if opening.len() == OPENING_STRUCTURES {
                    break;
                }
            }
        }
        insights.push((
            player.name.clone(),
            PlayerGameInsight {
                won: player.result == "Win",
                matchup,
                map_title: game.map_title.clone(),
                duration_seconds: loop_seconds(last_loop),
                opening,
                first_army_unit: acc
                    .build_order
                    .iter()
                    .find(|name| army_supply(balance, name).is_some())
                    .cloned(),
                favourite_army_unit: acc
                    .produced
                    .iter()
                    .filter(|(unit, _)| army_supply(balance, unit).is_some())
                    .max_by_key(|(_, count)| **count)
                    .map(|(unit, _)| unit.clone()),
                benchmarks: acc.benchmarks,
                production_idle: acc
                    .production_idle
                    .into_iter()
                    .map(|(structure, (structures, idle_seconds))| ProductionIdle {
                        structure,
                        structures,
                        idle_seconds,
                    })
                    .collect(),
                first_scout_unit: first_scout.map(|command| command.unit_name.clone()),
                first_scout_seconds: first_scout.map(|command| loop_seconds(command.game_loop)),
                first_upgrade: acc.first_upgrade,
                killed_by: sorted_counts(acc.killed_by),
                army_at_10: acc.army_at_10,
                was_behind: acc.was_behind,
                proxy: acc.proxy,
                base_trade,
                rating: player.rating,
                supply_blocked_seconds: acc.supply_blocked_seconds as f32,
                floating_seconds: acc.floating_seconds as f32,
                avg_unspent: match acc.unspent_samples.len() {
                    0 => 0.,
                    samples => {
                        (acc.unspent_samples.iter().sum::<i32>() / samples as i32) as u32 as f32
                    }
                },
                injects: count("SpawnLarva"),
                chrono_boosts: count("ChronoBoost"),
                mules: count("CalldownMULE"),
                commands_per_minute,
                resources_lost: acc.lost as f32,
                resources_killed: acc.killed as f32,
                mining_seconds_lost: acc.mining_seconds_lost.round(),
                bases_at_10: acc.bases_at_10 as f32,
                first_kill_seconds: acc.first_kill_seconds,
                energy,
            },
        ));
    }
    Ok(insights)
}

/// The insights of every player of every game of the day: `(ext_fs_id, name, insight)`, the name
/// without the clan tag. A game that cannot be computed is left out with a warning.
pub fn day_insights(day: &Path) -> Result<Vec<(u64, String, PlayerGameInsight)>, SwarmyError> {
    let games = read_day(day)?;
    let mut rows = vec![];
    for (ext_fs_id, game) in &games {
        match game_insights(game) {
            Ok(insights) => rows.extend(
                insights
                    .into_iter()
                    .map(|(name, insight)| (*ext_fs_id, name, insight)),
            ),
            Err(err) => {
                tracing::warn!("Unable to compute the insights of game {ext_fs_id}: {err:?}")
            }
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay_detail::data::analyze_replay_file;

    /// The insight of the analysis of a replay, as the Player view computed it before.
    fn from_replay(detail: &ReplayDetail, summary: &ReplayPlayerSummary) -> PlayerGameInsight {
        let opponents: Vec<&ReplayPlayerSummary> = detail
            .players
            .iter()
            .filter(|other| other.player.team_id != summary.player.team_id)
            .collect();
        let macros = &summary.macro_summary;
        let minutes = &macros.commands_per_minute;
        PlayerGameInsight {
            won: summary.player.result == "Win",
            matchup: match opponents.as_slice() {
                [opponent] => matchup_label(&summary.player.race, &opponent.player.race),
                _ => "Team game".to_string(),
            },
            map_title: detail.map_title.clone(),
            duration_seconds: detail.duration_seconds,
            opening: summary.opening.clone(),
            first_army_unit: summary.first_army_unit.clone(),
            favourite_army_unit: summary.favourite_army_unit.clone(),
            benchmarks: summary.benchmarks.clone(),
            production_idle: macros.production_idle.clone(),
            first_scout_unit: summary.first_scout.as_ref().map(|scout| scout.text.clone()),
            first_scout_seconds: summary.first_scout.as_ref().map(|scout| scout.seconds),
            first_upgrade: summary.first_upgrade.clone(),
            killed_by: summary.killed_by.clone(),
            army_at_10: summary.army_at_10.clone(),
            was_behind: summary.was_behind,
            proxy: summary.proxy.is_some(),
            base_trade: summary.bases_lost.len() >= 2
                && opponents
                    .iter()
                    .any(|opponent| opponent.bases_lost.len() >= 2),
            rating: summary.rating,
            supply_blocked_seconds: macros.supply_blocked_seconds as f32,
            floating_seconds: macros.floating_seconds as f32,
            avg_unspent: macros.avg_unspent as f32,
            injects: macros.injects as f32,
            chrono_boosts: macros.chrono_boosts as f32,
            mules: macros.mules as f32,
            commands_per_minute: minutes.iter().sum::<u32>() as f32 / minutes.len().max(1) as f32,
            resources_lost: summary.resources.lost.total() as f32,
            resources_killed: summary.resources.killed.total() as f32,
            mining_seconds_lost: summary.resources.mining_seconds_lost as f32,
            bases_at_10: summary.bases_at_10 as f32,
            first_kill_seconds: summary.first_kill_seconds,
            energy: summary.ability_efficiency.clone(),
        }
    }

    /// Compares the insights of the snapshot with the ones of the analysis of the replays of the
    /// first days of `SWARMY_TEST_SNAPSHOT_IPCS` (an `ipcs` directory written by s2protocol 5.1),
    /// `SWARMY_TEST_GAMES` games at most (200).
    #[test]
    #[ignore]
    fn it_matches_the_analysis_of_the_replays() {
        let ipcs = std::env::var("SWARMY_TEST_SNAPSHOT_IPCS").expect("SWARMY_TEST_SNAPSHOT_IPCS");
        let max_games: usize = std::env::var("SWARMY_TEST_GAMES")
            .ok()
            .and_then(|games| games.parse().ok())
            .unwrap_or(200);
        let days = s2protocol::arrow_store::partitioned::list_partitions(Path::new(&ipcs)).unwrap();
        let mut same: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
        let mut examples: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
        let mut games = 0;
        let started = std::time::Instant::now();
        let mut snapshot_seconds = 0.;
        // Every tenth day, from the latest: the games of several players and kinds.
        for day in days.iter().rev().step_by(10) {
            if games >= max_games {
                break;
            }
            let day_started = std::time::Instant::now();
            let day_games = read_day(day).unwrap();
            snapshot_seconds += day_started.elapsed().as_secs_f64();
            let details =
                read_day_file(day, DETAILS_IPC, &["ext_fs_id", "ext_fs_file_name"]).unwrap();
            let files: HashMap<u64, String> = u64_values(&details, "ext_fs_id")
                .unwrap()
                .into_iter()
                .zip(str_values(&details, "ext_fs_file_name").unwrap())
                .collect();
            for (id, game) in &day_games {
                if games >= max_games {
                    break;
                }
                let id = *id;
                let players = game_insights(game).unwrap();
                let lost_units = LOST_UNITS.with(|lost| lost.borrow().clone());
                let Ok(detail) = analyze_replay_file(&files[&id]) else {
                    continue;
                };
                games += 1;
                for (name, snapshot) in players {
                    let Some(summary) = detail
                        .players
                        .iter()
                        .find(|summary| name_without_clan(&summary.player.name) == name)
                    else {
                        continue;
                    };
                    let replay = from_replay(&detail, summary);
                    let human = !summary.player.is_computer;
                    // The unit types lost in different numbers.
                    let snapshot_lost = lost_units.get(&name).cloned().unwrap_or_default();
                    let replay_lost: BTreeMap<String, usize> = summary
                        .units_lost
                        .iter()
                        .map(|unit| (unit.unit.clone(), unit.count))
                        .collect();
                    let lost_diff: Vec<String> = snapshot_lost
                        .keys()
                        .chain(replay_lost.keys())
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .filter(|unit| snapshot_lost.get(*unit) != replay_lost.get(*unit))
                        .map(|unit| {
                            format!(
                                "{unit} {}/{}",
                                snapshot_lost.get(unit).unwrap_or(&0),
                                replay_lost.get(unit).unwrap_or(&0)
                            )
                        })
                        .collect();
                    let close =
                        |a: f32, b: f32| (a - b).abs() <= 0.02 * a.abs().max(b.abs()).max(1.);
                    let fields: Vec<(&str, bool, String)> = vec![
                        ("won", snapshot.won == replay.won, String::new()),
                        (
                            "matchup",
                            snapshot.matchup == replay.matchup,
                            format!("{} / {}", snapshot.matchup, replay.matchup),
                        ),
                        (
                            "duration",
                            snapshot.duration_seconds.abs_diff(replay.duration_seconds) <= 1,
                            format!(
                                "{} / {}",
                                snapshot.duration_seconds, replay.duration_seconds
                            ),
                        ),
                        (
                            "opening",
                            snapshot.opening == replay.opening,
                            format!("{:?} / {:?}", snapshot.opening, replay.opening),
                        ),
                        (
                            "first_army_unit",
                            snapshot.first_army_unit == replay.first_army_unit,
                            format!(
                                "{:?} / {:?}",
                                snapshot.first_army_unit, replay.first_army_unit
                            ),
                        ),
                        (
                            "favourite_army_unit",
                            snapshot.favourite_army_unit == replay.favourite_army_unit,
                            format!(
                                "{:?} / {:?}",
                                snapshot.favourite_army_unit, replay.favourite_army_unit
                            ),
                        ),
                        (
                            "benchmarks",
                            snapshot.benchmarks == replay.benchmarks,
                            format!(
                                "{:?} / {:?}",
                                snapshot.benchmarks.first(),
                                replay.benchmarks.first()
                            ),
                        ),
                        (
                            "production_idle",
                            snapshot.production_idle == replay.production_idle,
                            format!(
                                "{:?} / {:?}",
                                snapshot.production_idle, replay.production_idle
                            ),
                        ),
                        (
                            "first_scout",
                            snapshot.first_scout_unit == replay.first_scout_unit,
                            format!(
                                "{:?} {:?} / {:?} {:?}",
                                snapshot.first_scout_unit,
                                snapshot.first_scout_seconds,
                                replay.first_scout_unit,
                                replay.first_scout_seconds
                            ),
                        ),
                        (
                            "first_upgrade",
                            snapshot.first_upgrade == replay.first_upgrade,
                            format!("{:?} / {:?}", snapshot.first_upgrade, replay.first_upgrade),
                        ),
                        (
                            "killed_by",
                            snapshot.killed_by == replay.killed_by,
                            format!(
                                "{:?} / {:?}",
                                snapshot.killed_by.first(),
                                replay.killed_by.first()
                            ),
                        ),
                        (
                            "army_at_10",
                            snapshot.army_at_10 == replay.army_at_10,
                            format!("{:?} / {:?}", snapshot.army_at_10, replay.army_at_10),
                        ),
                        (
                            "was_behind",
                            snapshot.was_behind == replay.was_behind,
                            String::new(),
                        ),
                        ("proxy", snapshot.proxy == replay.proxy, String::new()),
                        (
                            "base_trade",
                            snapshot.base_trade == replay.base_trade,
                            String::new(),
                        ),
                        (
                            "rating",
                            snapshot.rating == replay.rating,
                            format!("{:?} / {:?}", snapshot.rating, replay.rating),
                        ),
                        (
                            "supply_blocked",
                            close(
                                snapshot.supply_blocked_seconds,
                                replay.supply_blocked_seconds,
                            ),
                            format!(
                                "{} / {}",
                                snapshot.supply_blocked_seconds, replay.supply_blocked_seconds
                            ),
                        ),
                        (
                            "floating",
                            close(snapshot.floating_seconds, replay.floating_seconds),
                            format!(
                                "{} / {}",
                                snapshot.floating_seconds, replay.floating_seconds
                            ),
                        ),
                        (
                            "avg_unspent",
                            close(snapshot.avg_unspent, replay.avg_unspent),
                            format!("{} / {}", snapshot.avg_unspent, replay.avg_unspent),
                        ),
                        (
                            "injects",
                            snapshot.injects == replay.injects,
                            format!("{} / {}", snapshot.injects, replay.injects),
                        ),
                        (
                            "chrono_boosts",
                            snapshot.chrono_boosts == replay.chrono_boosts,
                            format!("{} / {}", snapshot.chrono_boosts, replay.chrono_boosts),
                        ),
                        (
                            "mules",
                            snapshot.mules == replay.mules,
                            format!("{} / {}", snapshot.mules, replay.mules),
                        ),
                        (
                            "commands_per_minute",
                            close(snapshot.commands_per_minute, replay.commands_per_minute),
                            format!(
                                "{} / {}",
                                snapshot.commands_per_minute, replay.commands_per_minute
                            ),
                        ),
                        (
                            "resources_lost",
                            close(snapshot.resources_lost, replay.resources_lost),
                            format!(
                                "{} / {} {lost_diff:?}",
                                snapshot.resources_lost, replay.resources_lost
                            ),
                        ),
                        (
                            "resources_killed",
                            close(snapshot.resources_killed, replay.resources_killed),
                            format!(
                                "{} / {}",
                                snapshot.resources_killed, replay.resources_killed
                            ),
                        ),
                        (
                            "mining_seconds_lost",
                            close(snapshot.mining_seconds_lost, replay.mining_seconds_lost),
                            format!(
                                "{} / {}",
                                snapshot.mining_seconds_lost, replay.mining_seconds_lost
                            ),
                        ),
                        (
                            "bases_at_10",
                            snapshot.bases_at_10 == replay.bases_at_10,
                            format!("{} / {}", snapshot.bases_at_10, replay.bases_at_10),
                        ),
                        (
                            "first_kill",
                            snapshot
                                .first_kill_seconds
                                .zip(replay.first_kill_seconds)
                                .is_none_or(|(a, b)| a.abs_diff(b) <= 1)
                                && snapshot.first_kill_seconds.is_some()
                                    == replay.first_kill_seconds.is_some(),
                            format!(
                                "{:?} / {:?}",
                                snapshot.first_kill_seconds, replay.first_kill_seconds
                            ),
                        ),
                    ];
                    let energy_close = snapshot.energy.len() == replay.energy.len()
                        && snapshot.energy.iter().zip(&replay.energy).all(|(a, b)| {
                            a.ability == b.ability
                                && a.uses == b.uses
                                && (a.possible - b.possible).abs() <= 0.05 * b.possible.max(1.)
                        });
                    let mut fields = fields;
                    fields.push((
                        "energy",
                        energy_close,
                        format!(
                            "{:?} / {:?}",
                            snapshot
                                .energy
                                .iter()
                                .map(|e| (e.uses, e.possible as u32))
                                .collect::<Vec<_>>(),
                            replay
                                .energy
                                .iter()
                                .map(|e| (e.uses, e.possible as u32))
                                .collect::<Vec<_>>()
                        ),
                    ));
                    for (field, equal, example) in fields {
                        let field: &'static str = if human {
                            field
                        } else {
                            Box::leak(format!("{field} (computers)").into_boxed_str())
                        };
                        let entry = same.entry(field).or_default();
                        entry.1 += 1;
                        if equal {
                            entry.0 += 1;
                        } else if examples.get(field).is_none_or(|list| list.len() < 3) {
                            examples
                                .entry(field)
                                .or_default()
                                .push(format!("game {id} {name}: {example}"));
                        }
                    }
                }
            }
        }
        println!(
            "{games} games, the snapshot in {snapshot_seconds:.1}s, all in {:.1}s",
            started.elapsed().as_secs_f64()
        );
        for (field, (equal, total)) in &same {
            println!(
                "{field:22} {equal:>5} / {total:<5} {:5.1}%",
                100. * *equal as f64 / *total as f64
            );
            for example in examples.get(field).into_iter().flatten() {
                println!("    snapshot / replay: {example}");
            }
        }
    }
}
