//! Translation of s2protocol events into Rerun primitives.
//!
//! The entity layout is kept small on purpose, there is at most one entity per player per kind,
//! units are not given their own entities, instead they are batched as instances:
//! - `units/{player}`: Points3D snapshot of all the alive units, sampled every
//!   [`UNIT_SNAPSHOT_LOOPS`] and only if any of the player units changed.
//! - `deaths/{player}`: Points3D of the units that died in the last [`DEATH_VISIBLE_LOOPS`].
//! - `commands/{player}`: Arrows3D from units to their command targets, issued in the last
//!   [`COMMAND_VISIBLE_LOOPS`].
//! - `cameras/{player}`: Boxes3D of the player camera.
//! - `events/{player}`: TextLog with births, deaths, upgrades, abilities and chat.
//! - `stats/{stat}`: Scalars, one series per player.
//! - `unit_types/{player}/{unit}`: Scalar with the alive count of each unit type.
//! - `decisions/{player}`: Scalar with the accumulated score of the unit choices, see
//!   [`decisions::score_unit_choice`].
//!
//! A blueprint is sent so that the map takes most of the viewer, see
//! [`blueprint::send_blueprint`].
pub mod unit_colors;
use s2protocol::state::unit_props::*;

pub mod blueprint;
pub mod decisions;
pub mod game_events;
pub mod players;
pub mod terrain;
pub mod viewer;

pub use players::{ReplayPlayers, NEUTRAL_PLAYER};
use terrain::Terrain;

use decisions::{
    base_unit_name, format_unit_counts, has_cost, is_morph_death, score_unit_choice, unit_supply,
};

use rerun::{entity_path, RecordingStream};
use s2protocol::game_events::VersionedBalanceUnits;
use s2protocol::{tracker_events::*, SC2ReplayState, SC2Unit, UnitChangeHint};
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use swarmy_common::{clean_player_name_clan_escapes, SwarmyError};

/// Unit snapshots are taken at most once per this many game loops (~1 second at Faster speed).
pub const UNIT_SNAPSHOT_LOOPS: i64 = 22;

/// Deaths are shown for this many game loops before being cleared.
pub const DEATH_VISIBLE_LOOPS: i64 = 112;

/// The radius of the points drawn where units died.
pub const DEATH_RADIUS: f32 = 0.75;

/// Command arrows and camera boxes are drawn this high above the ground.
pub const OVERLAY_HEIGHT: f32 = 0.5;

/// Command arrows are shown for this many game loops before being cleared.
pub const COMMAND_VISIBLE_LOOPS: i64 = 45;

/// The player stats as named by `PlayerStats::as_prop_name_value_vec`.
pub const PLAYER_STATS: [&str; 39] = [
    "minerals/current",
    "vespene/current",
    "minerals/collection_rate",
    "vespene/collection_rate",
    "workers_active_count",
    "minerals/used_in_progress_army",
    "minerals/used_in_progress_economy",
    "minerals/used_in_progress_technology",
    "vespene/used_in_progress_army",
    "vespene/used_in_progress_economy",
    "vespene/used_in_progress_technology",
    "minerals/used_current_army",
    "minerals/used_current_economy",
    "minerals/used_current_technology",
    "vespene/used_current_army",
    "vespene/used_current_economy",
    "vespene/used_current_technology",
    "minerals/lost_army",
    "minerals/lost_economy",
    "minerals/lost_technology",
    "vespene/lost_army",
    "vespene/lost_economy",
    "vespene/lost_technology",
    "minerals/killed_army",
    "minerals/killed_economy",
    "minerals/killed_technology",
    "vespene/killed_army",
    "vespene/killed_economy",
    "vespene/killed_technology",
    "food/used",
    "food/made",
    "minerals/used_active_forces",
    "vespene/used_active_forces",
    "minerals/friendly_fire_army",
    "minerals/friendly_fire_economy",
    "minerals/friendly_fire_technology",
    "vespene/friendly_fire_army",
    "vespene/friendly_fire_economy",
    "vespene/friendly_fire_technology",
];

/// The command arrows of a player for a game loop.
/// The overlays added in the current game loop for a player.
struct PendingOverlays<T> {
    color: [u8; 4],
    items: Vec<T>,
}

/// A command arrow, its origin and the vector to its target.
type CommandArrow = ([f32; 3], [f32; 3]);

/// The position of a dead unit and what killed it, shown on hover.
type DeathMark = ([f32; 3], String);

/// The overlays of a player that are still visible. The entity holds a single batch, each
/// batch replaces the previous one, so every batch must include all the visible overlays.
struct RecentOverlays<T> {
    color: [u8; 4],
    /// The overlays with the game loop they were added in, oldest first.
    entries: VecDeque<(i64, T)>,
}

fn set_stream_time(recording_stream: &RecordingStream, game_loop: i64) {
    recording_stream.set_time_sequence("game_loop", game_loop);
    recording_stream.set_duration_secs(
        "game_time",
        game_loop as f64 / viewer::GAME_LOOPS_PER_SECOND,
    );
}

/// Logs the overlays that changed: at the game loops in which some of them expired, up to
/// `current_loop`, and then at `current_loop` with the pending ones added.
fn flush_overlays<T>(
    recording_stream: &RecordingStream,
    current_loop: i64,
    visible_loops: i64,
    recent: &mut BTreeMap<String, RecentOverlays<T>>,
    pending: BTreeMap<String, PendingOverlays<T>>,
    log: impl Fn(&RecordingStream, &str, &RecentOverlays<T>) -> Result<(), SwarmyError>,
) -> Result<(), SwarmyError> {
    for (player, overlays) in recent.iter_mut() {
        while let Some(expiry) = overlays
            .entries
            .front()
            .map(|(added, _)| added + visible_loops)
            .filter(|expiry| *expiry <= current_loop)
        {
            while overlays
                .entries
                .front()
                .is_some_and(|(added, _)| added + visible_loops <= expiry)
            {
                overlays.entries.pop_front();
            }
            set_stream_time(recording_stream, expiry);
            log(recording_stream, player, overlays)?;
        }
    }
    if !pending.is_empty() {
        set_stream_time(recording_stream, current_loop);
    }
    for (player, PendingOverlays { color, items }) in pending {
        let overlays = recent
            .entry(player.clone())
            .or_insert_with(|| RecentOverlays {
                color,
                entries: VecDeque::new(),
            });
        overlays
            .entries
            .extend(items.into_iter().map(|item| (current_loop, item)));
        log(recording_stream, &player, overlays)?;
    }
    recent.retain(|_, overlays| !overlays.entries.is_empty());
    Ok(())
}

fn log_deaths(
    recording_stream: &RecordingStream,
    player: &str,
    deaths: &RecentOverlays<DeathMark>,
) -> Result<(), SwarmyError> {
    let path = entity_path!("deaths", player);
    if deaths.entries.is_empty() {
        recording_stream.log(path, &rerun::Clear::flat())?;
    } else {
        recording_stream.log(
            path,
            &rerun::Points3D::new(deaths.entries.iter().map(|(_, (position, _))| *position))
                .with_colors([deaths.color])
                .with_radii([DEATH_RADIUS])
                .with_labels(deaths.entries.iter().map(|(_, (_, label))| label.as_str()))
                .with_show_labels(false),
        )?;
    }
    Ok(())
}

fn log_commands(
    recording_stream: &RecordingStream,
    player: &str,
    commands: &RecentOverlays<CommandArrow>,
) -> Result<(), SwarmyError> {
    let path = entity_path!("commands", player);
    if commands.entries.is_empty() {
        recording_stream.log(path, &rerun::Clear::flat())?;
    } else {
        recording_stream.log(
            path,
            &rerun::Arrows3D::from_vectors(commands.entries.iter().map(|(_, (_, vector))| *vector))
                .with_origins(commands.entries.iter().map(|(_, (origin, _))| *origin))
                .with_colors([commands.color]),
        )?;
    }
    Ok(())
}

/// Keeps track of the data pending to be logged for the current game loop.
pub struct ReplayRerunLogger {
    recording_stream: RecordingStream,
    /// The unit costs, strengths and weaknesses, possibly from a close protocol version.
    balance_units: &'static VersionedBalanceUnits,
    /// The game loop the pending data belongs to.
    current_loop: i64,
    /// The last game loop in which the units were snapshotted.
    last_snapshot_loop: Option<i64>,
    /// The players whose units changed since the last snapshot.
    dirty_players: BTreeSet<String>,
    players: ReplayPlayers,
    /// The map terrain, if the map cache is available, units are drawn on top of it.
    terrain: Option<Terrain>,
    /// The unit names to annotation class ids.
    unit_classes: HashMap<String, u16>,
    /// The death positions of the current game loop per player.
    deaths: BTreeMap<String, PendingOverlays<DeathMark>>,
    /// The visible death positions per player.
    recent_deaths: BTreeMap<String, RecentOverlays<DeathMark>>,
    /// The command arrows of the current game loop per player.
    commands: BTreeMap<String, PendingOverlays<CommandArrow>>,
    /// The visible command arrows per player.
    recent_commands: BTreeMap<String, RecentOverlays<CommandArrow>>,
    /// The latest value of each stat, indexed by player_id - 1.
    stats: BTreeMap<String, Vec<f64>>,
    /// The stats that changed in the current game loop.
    dirty_stats: BTreeSet<String>,
    /// The last logged count of each unit type, per player.
    unit_type_counts: HashMap<String, BTreeMap<String, usize>>,
    /// The base unit name of the alive units whose choice has been scored, by tag index.
    scored_units: HashMap<u32, String>,
    /// The accumulated choice score per player.
    decision_totals: BTreeMap<String, f64>,
}

impl ReplayRerunLogger {
    pub fn new(
        recording_stream: RecordingStream,
        state: &SC2ReplayState,
        balance_units: &'static VersionedBalanceUnits,
        terrain: Option<Terrain>,
    ) -> Result<Self, SwarmyError> {
        let players = ReplayPlayers::from_state(state);
        blueprint::send_blueprint(&recording_stream, &players.names)?;
        recording_stream.log_static("/", &rerun::ViewCoordinates::RIGHT_HAND_Z_UP())?;
        if let Some(terrain) = &terrain {
            recording_stream.log_static("map/terrain", &terrain.mesh())?;
        }
        Ok(Self {
            recording_stream,
            balance_units,
            current_loop: 0,
            last_snapshot_loop: None,
            dirty_players: BTreeSet::new(),
            players,
            terrain,
            unit_classes: HashMap::new(),
            deaths: BTreeMap::new(),
            recent_deaths: BTreeMap::new(),
            commands: BTreeMap::new(),
            recent_commands: BTreeMap::new(),
            stats: BTreeMap::new(),
            dirty_stats: BTreeSet::new(),
            unit_type_counts: HashMap::new(),
            scored_units: HashMap::new(),
            decision_totals: BTreeMap::new(),
        })
    }

    fn set_time(&self, game_loop: i64) {
        set_stream_time(&self.recording_stream, game_loop);
    }

    /// Returns the height of the top of the ground at the map position, 0 without terrain.
    /// Points are drawn as spheres, so they must be lifted by their radius on top of it.
    fn ground_z(&self, x: f32, y: f32) -> f32 {
        self.terrain
            .as_ref()
            .map(|terrain| terrain.z_at(x, y))
            .unwrap_or(0.)
    }

    /// Logs a text line on the player events.
    fn log_event(&self, player: &str, text: String, level: &str) -> Result<(), SwarmyError> {
        self.recording_stream.log(
            entity_path!("events", player),
            &rerun::TextLog::new(text).with_level(level),
        )?;
        Ok(())
    }

    /// Flushes the pending data whenever the game loop changes.
    pub fn advance_to(
        &mut self,
        game_loop: i64,
        state: &SC2ReplayState,
    ) -> Result<(), SwarmyError> {
        if game_loop != self.current_loop {
            self.flush(state, false)?;
            self.current_loop = game_loop;
            self.set_time(game_loop);
        }
        Ok(())
    }

    /// Flushes everything that is pending, to be called once the replay is exhausted.
    pub fn finish(&mut self, state: &SC2ReplayState) -> Result<(), SwarmyError> {
        self.flush(state, true)?;
        // Hides the overlays still visible at the end, at the game loops they expire in.
        self.flush_overlays(i64::MAX)
    }

    fn flush_overlays(&mut self, current_loop: i64) -> Result<(), SwarmyError> {
        flush_overlays(
            &self.recording_stream,
            current_loop,
            DEATH_VISIBLE_LOOPS,
            &mut self.recent_deaths,
            std::mem::take(&mut self.deaths),
            log_deaths,
        )?;
        flush_overlays(
            &self.recording_stream,
            current_loop,
            COMMAND_VISIBLE_LOOPS,
            &mut self.recent_commands,
            std::mem::take(&mut self.commands),
            log_commands,
        )
    }

    fn flush(&mut self, state: &SC2ReplayState, force_snapshot: bool) -> Result<(), SwarmyError> {
        // Expired overlays are logged at their own game loop, which is in the past by now.
        self.flush_overlays(self.current_loop)?;
        self.set_time(self.current_loop);

        for stat in std::mem::take(&mut self.dirty_stats) {
            self.recording_stream.log(
                format!("stats/{stat}"),
                &rerun::Scalars::new(self.stats[&stat].clone()),
            )?;
        }

        if !self.dirty_players.is_empty()
            && (force_snapshot
                || self
                    .last_snapshot_loop
                    .is_none_or(|last| self.current_loop - last >= UNIT_SNAPSHOT_LOOPS))
        {
            self.snapshot_units(state)?;
        }
        Ok(())
    }

    /// Logs the alive units of the players that changed since the last snapshot.
    fn snapshot_units(&mut self, state: &SC2ReplayState) -> Result<(), SwarmyError> {
        let mut player_units: BTreeMap<String, Vec<&SC2Unit>> = self
            .dirty_players
            .iter()
            .map(|player| (player.clone(), vec![]))
            .collect();
        let mut new_classes = false;
        for unit in state.units.values() {
            if let Some(units) = player_units.get_mut(&self.players.unit_player(unit)) {
                if !self.unit_classes.contains_key(&unit.name) {
                    let class_id = self.unit_classes.len() as u16;
                    self.unit_classes.insert(unit.name.clone(), class_id);
                    new_classes = true;
                }
                units.push(unit);
            }
        }
        if new_classes {
            // The unit names are shown on hover through the class ids.
            let classes: Vec<(u16, &str)> = self
                .unit_classes
                .iter()
                .map(|(name, class_id)| (*class_id, name.as_str()))
                .collect();
            self.recording_stream
                .log_static("units", &rerun::AnnotationContext::new(classes))?;
        }
        for (player, mut units) in player_units {
            if player != NEUTRAL_PLAYER {
                self.log_unit_type_counts(&player, &units)?;
            }
            let path = entity_path!("units", player);
            if units.is_empty() {
                self.recording_stream.log(path, &rerun::Clear::flat())?;
                continue;
            }
            // Keep the instance order stable between snapshots.
            units.sort_by_key(|unit| unit.tag_index);
            // Neutral units keep their per unit type colors, i.e. minerals, geysers.
            let player_color = self.players.color(&player);
            self.recording_stream.log(
                path,
                &rerun::Points3D::new(units.iter().map(|unit| {
                    let (x, y) = (unit.pos.x(), unit.pos.y());
                    (x, y, self.ground_z(x, y) + unit.radius)
                }))
                .with_colors(units.iter().map(|unit| player_color.unwrap_or(unit.color)))
                .with_radii(units.iter().map(|unit| unit.radius))
                .with_class_ids(units.iter().map(|unit| self.unit_classes[&unit.name]))
                // The hover shows the label, the instance index means nothing to the user.
                .with_labels(
                    units
                        .iter()
                        .map(|unit| format!("{} (tag {})", unit.name, unit.tag_index)),
                )
                .with_show_labels(false),
            )?;
        }
        self.dirty_players.clear();
        self.last_snapshot_loop = Some(self.current_loop);
        Ok(())
    }

    /// Logs the count of each unit type that changed since the last snapshot.
    fn log_unit_type_counts(
        &mut self,
        player: &str,
        units: &[&SC2Unit],
    ) -> Result<(), SwarmyError> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for unit in units {
            let name = base_unit_name(&unit.name);
            // Structures, larva, eggs, etc have no supply cost.
            if unit_supply(self.balance_units, name).is_some() {
                *counts.entry(name.to_string()).or_default() += 1;
            }
        }
        let previous = self
            .unit_type_counts
            .insert(player.to_string(), counts.clone())
            .unwrap_or_default();
        let gone = previous
            .keys()
            .filter(|name| !counts.contains_key(*name))
            .map(|name| (name.clone(), 0));
        for (name, count) in counts.clone().into_iter().chain(gone) {
            if previous.get(&name) != Some(&count) {
                self.recording_stream.log(
                    entity_path!("unit_types", player, name),
                    &rerun::Scalars::new([count as f64]),
                )?;
            }
        }
        Ok(())
    }

    /// Scores the unit choice against the enemy army, see [`score_unit_choice`].
    fn register_unit_choice(
        &mut self,
        change_hint: &UnitChangeHint,
        state: &SC2ReplayState,
    ) -> Result<(), SwarmyError> {
        let UnitChangeHint::Registered { unit, .. } = change_hint else {
            return Ok(());
        };
        let player = self.players.unit_player(unit);
        if player == NEUTRAL_PLAYER {
            return Ok(());
        }
        let name = base_unit_name(&unit.name).to_string();
        // Burrowing, sieging, etc register the unit again under a variant name.
        if self.scored_units.get(&unit.tag_index) == Some(&name) {
            return Ok(());
        }
        self.scored_units.insert(unit.tag_index, name.clone());
        let enemy_units = state.units.values().filter(|enemy| {
            self.players
                .is_enemy(&player, &self.players.unit_player(enemy))
        });
        let Some(choice) = score_unit_choice(self.balance_units, &name, enemy_units) else {
            return Ok(());
        };
        if !self.decision_totals.contains_key(&player) {
            let mut series = rerun::SeriesLines::new().with_names([player.clone()]);
            if let Some(color) = self.players.color(&player) {
                series = series.with_colors([color]);
            }
            self.recording_stream
                .log_static(entity_path!("decisions", player), &series)?;
        }
        let total = self.decision_totals.entry(player.clone()).or_default();
        *total += choice.score;
        let total = *total;
        self.recording_stream.log(
            entity_path!("decisions", player),
            &rerun::Scalars::new([total]),
        )?;
        let (verdict, level) = if choice.is_good() {
            ("Good", rerun::TextLogLevel::INFO)
        } else if choice.is_bad() {
            ("Bad", rerun::TextLogLevel::WARN)
        } else {
            ("Even", rerun::TextLogLevel::DEBUG)
        };
        let mut text = format!("{verdict} choice {name} ({:+.2})", choice.score);
        if !choice.strong_vs.is_empty() {
            text.push_str(&format!(
                " strong vs {}",
                format_unit_counts(&choice.strong_vs)
            ));
        }
        if !choice.weak_vs.is_empty() {
            text.push_str(&format!(" weak vs {}", format_unit_counts(&choice.weak_vs)));
        }
        self.log_event(&player, text, level)
    }

    /// Marks the players whose units are in the change hint as needing a new snapshot.
    fn mark_dirty(&mut self, change_hint: &UnitChangeHint) {
        let units: Vec<&SC2Unit> = match change_hint {
            UnitChangeHint::Registered { unit, .. } => vec![unit],
            UnitChangeHint::Unregistered { killed, .. } => vec![killed],
            UnitChangeHint::Positions(units)
            | UnitChangeHint::TargetPoints(units)
            | UnitChangeHint::TargetUnits { units, .. } => units.iter().collect(),
            UnitChangeHint::Abilities { .. }
            | UnitChangeHint::Selection(_)
            | UnitChangeHint::None => vec![],
        };
        for unit in units {
            self.dirty_players.insert(self.players.unit_player(unit));
        }
    }

    fn register_unit(
        &mut self,
        verb: &str,
        change_hint: &UnitChangeHint,
    ) -> Result<(), SwarmyError> {
        if let UnitChangeHint::Registered { unit, creator } = change_hint {
            let player = self.players.unit_player(unit);
            if player != NEUTRAL_PLAYER {
                let text = match creator {
                    Some(creator) => format!("{verb} {} (from {})", unit.name, creator.name),
                    None => format!("{verb} {}", unit.name),
                };
                self.log_event(&player, text, rerun::TextLogLevel::DEBUG)?;
            }
        }
        Ok(())
    }

    fn register_unit_died(&mut self, change_hint: &UnitChangeHint) -> Result<(), SwarmyError> {
        if let UnitChangeHint::Unregistered { killer, killed } = change_hint {
            let player = self.players.unit_player(killed);
            if player == NEUTRAL_PLAYER {
                // Mined out minerals, destroyed rocks, etc.
                return Ok(());
            }
            // A Larva dies when it morphs into a unit, Broodlings, Adept shades, etc are
            // temporary, only the units that cost resources are losses.
            if !has_cost(self.balance_units, base_unit_name(&killed.name)) {
                return Ok(());
            }
            // A Drone morphing into a structure or templars merging are not losses.
            if is_morph_death(base_unit_name(&killed.name), killer.is_some()) {
                return Ok(());
            }
            let (x, y) = (killed.pos.x(), killed.pos.y());
            let z = self.ground_z(x, y) + DEATH_RADIUS;
            let text = match killer {
                Some(killer) => format!(
                    "Lost {} to {} ({})",
                    killed.name,
                    killer.name,
                    self.players.unit_player(killer)
                ),
                None => format!("Lost {}", killed.name),
            };
            self.deaths
                .entry(player.clone())
                .or_insert_with(|| PendingOverlays {
                    color: FREYA_RED,
                    items: vec![],
                })
                .items
                .push(([x, y, z], text.clone()));
            self.log_event(&player, text, rerun::TextLogLevel::INFO)?;
        }
        Ok(())
    }

    fn register_player_stats(
        &mut self,
        player_stats: &PlayerStatsEvent,
    ) -> Result<(), SwarmyError> {
        let player_idx = (player_stats.player_id as usize).saturating_sub(1);
        for (stat, value) in player_stats.stats.as_prop_name_value_vec() {
            if !self.stats.contains_key(&stat) {
                let num_players = self.players.names.len().max(player_idx + 1);
                let names: Vec<String> = (0..num_players)
                    .map(|idx| self.players.tracker_player_name(idx as u8 + 1))
                    .collect();
                let colors: Vec<[u8; 4]> = (0..num_players)
                    .map(|idx| {
                        self.players
                            .colors
                            .get(idx)
                            .copied()
                            .unwrap_or(user_color(idx as i64 + 1))
                    })
                    .collect();
                self.recording_stream.log_static(
                    format!("stats/{stat}"),
                    &rerun::SeriesLines::new()
                        .with_names(names.clone())
                        .with_colors(colors),
                )?;
                self.stats.insert(stat.clone(), vec![0.; names.len()]);
            }
            let values = self.stats.get_mut(&stat).expect("stat was just inserted");
            if values.len() <= player_idx {
                values.resize(player_idx + 1, 0.);
            }
            values[player_idx] = value as f64;
            self.dirty_stats.insert(stat);
        }
        Ok(())
    }

    /// Registers the tracker events to Rerun.
    pub fn add_tracker_event(
        &mut self,
        evt: &ReplayTrackerEvent,
        change_hint: &UnitChangeHint,
        state: &SC2ReplayState,
    ) -> Result<(), SwarmyError> {
        self.mark_dirty(change_hint);
        match change_hint {
            UnitChangeHint::Registered { .. } => self.register_unit_choice(change_hint, state)?,
            UnitChangeHint::Unregistered { killed, .. } => {
                self.scored_units.remove(&killed.tag_index);
            }
            _ => {}
        }
        match evt {
            ReplayTrackerEvent::UnitInit(_) => self.register_unit("Started", change_hint)?,
            ReplayTrackerEvent::UnitBorn(_) => self.register_unit("Born", change_hint)?,
            ReplayTrackerEvent::UnitTypeChange(_) => {
                self.register_unit("Morphed into", change_hint)?
            }
            ReplayTrackerEvent::UnitDied(_) => self.register_unit_died(change_hint)?,
            ReplayTrackerEvent::PlayerStats(player_stats) => {
                self.register_player_stats(player_stats)?
            }
            // Sprays, rewards, etc are registered as upgrades at the start of the game.
            ReplayTrackerEvent::Upgrade(upgrade) if self.current_loop > 0 => {
                let player = match &upgrade.player_name {
                    Some(name) => clean_player_name_clan_escapes(name),
                    None => self.players.tracker_player_name(upgrade.player_id),
                };
                self.log_event(
                    &player,
                    format!("Upgrade {} ({})", upgrade.upgrade_type_name, upgrade.count),
                    rerun::TextLogLevel::INFO,
                )?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn pending(items: &[u32]) -> BTreeMap<String, PendingOverlays<u32>> {
        BTreeMap::from([(
            "A".to_string(),
            PendingOverlays {
                color: FREYA_RED,
                items: items.to_vec(),
            },
        )])
    }

    #[test]
    fn it_keeps_the_overlays_visible_for_their_window() {
        let stream = RecordingStream::disabled();
        let mut recent = BTreeMap::new();
        let logged = RefCell::new(vec![]);
        let log = |_: &RecordingStream, _: &str, overlays: &RecentOverlays<u32>| {
            let items: Vec<u32> = overlays.entries.iter().map(|(_, item)| *item).collect();
            logged.borrow_mut().push(items);
            Ok(())
        };
        flush_overlays(&stream, 10, 100, &mut recent, pending(&[1]), log).unwrap();
        flush_overlays(&stream, 20, 100, &mut recent, pending(&[2, 3]), log).unwrap();
        // The first overlay expires at 110, the others at 120.
        flush_overlays(&stream, 115, 100, &mut recent, BTreeMap::new(), log).unwrap();
        flush_overlays(&stream, i64::MAX, 100, &mut recent, BTreeMap::new(), log).unwrap();
        assert_eq!(
            logged.into_inner(),
            vec![vec![1], vec![1, 2, 3], vec![2, 3], vec![]]
        );
        assert!(recent.is_empty());
    }
}
