use crate::UnitEffectiveness;
use serde::{Deserialize, Serialize};

/// What the player was paying attention to, from the units selected and commanded.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum FocusKind {
    /// Army units, attacking, moving, scouting, casting.
    Army,
    /// Townhalls, production structures, larva, injects, chrono boosts, MULEs, structures built.
    Macro,
    /// Workers mining, repairing.
    Economy,
    /// Townhalls built.
    Expand,
    /// Research structures and research.
    Upgrades,
}

impl FocusKind {
    pub const ALL: [FocusKind; 5] = [
        FocusKind::Army,
        FocusKind::Macro,
        FocusKind::Economy,
        FocusKind::Expand,
        FocusKind::Upgrades,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            FocusKind::Army => "Army",
            FocusKind::Macro => "Macro",
            FocusKind::Economy => "Economy",
            FocusKind::Expand => "Expand",
            FocusKind::Upgrades => "Upgrades",
        }
    }
}

/// The selections and commands of the player by [`FocusKind`].
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct PlayerFocus {
    /// The actions of each minute, indexed by [`FocusKind::index`].
    pub per_minute: Vec<[u32; 5]>,
    /// The actions of the whole game, indexed by [`FocusKind::index`].
    pub totals: [u32; 5],
    /// The longest time commanding the army without any macro, expand or upgrade action.
    pub longest_army_only: Option<TimeRange>,
}

/// A period of the game, in game seconds.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub start: u32,
    pub end: u32,
}

/// The replay to analyse, by its id in the snapshot.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReplayDetailQuery {
    pub ext_fs_id: u64,
}

/// The notes of the user about a replay, kept in the personal database by the sha256 of the replay.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReplayNotes {
    pub sha256: String,
    /// From 1 to 5 stars, none when not rated.
    pub rating: Option<u8>,
    pub note: String,
    pub tags: Vec<String>,
}

/// Which replay to go to from the current one, by the date the games were played.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ReplayStep {
    /// The newest replay.
    #[default]
    Latest,
    Previous,
    Next,
}

/// The replay before or after the current one, of the locked map and players.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AdjacentReplayQuery {
    /// The current replay, none for the newest one.
    pub ext_fs_id: Option<u64>,
    pub step: ReplayStep,
    /// The replays of this map only.
    pub map_title: Option<String>,
    /// The replays of all these players only, as stored in the replay.
    pub player_names: Vec<String>,
}

/// A player of the replay.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplayPlayer {
    /// The player name as stored in the replay, with the clan tag escaped.
    pub name: String,
    pub race: String,
    /// Win, Loss, Tie or Undecided.
    pub result: String,
    pub team_id: u8,
    /// A computer player, i.e. Cheater 3 (Insane).
    #[serde(default)]
    pub is_computer: bool,
    /// RGBA
    pub color: [u8; 4],
}

/// Something that happened at a moment of the game.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct TimedEvent {
    pub seconds: u32,
    /// The player the event belongs to.
    pub player: String,
    pub text: String,
}

/// A unit, structure or upgrade started by a player.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct BuildOrderEntry {
    pub seconds: u32,
    /// The supply used at the last stats update before the entry.
    pub supply: i32,
    pub name: String,
    pub is_worker: bool,
    /// Upgrades are not units, they are not in the balance data.
    pub is_upgrade: bool,
    /// The average time of the upgrade, or of the expansion with the same townhall and number,
    /// in the games of the same patch. None for other entries or with too few games.
    #[serde(default)]
    pub patch_average: Option<PatchAverage>,
}

/// The average time of an upgrade or an expansion in the games of the same patch of the snapshot.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct PatchAverage {
    pub seconds: u32,
    /// The upgrades or expansions averaged, one per player and game.
    pub samples: usize,
}

impl PatchAverage {
    /// How the time compares to the average, i.e. `5s faster than average`.
    pub fn compare(&self, seconds: u32) -> String {
        let delta = seconds as i64 - self.seconds as i64;
        match delta {
            0 => "same as average".to_string(),
            delta if delta < 0 => format!("{}s faster than average", -delta),
            delta => format!("{delta}s slower than average"),
        }
    }
}

/// How many units of a type were killed or lost.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct UnitCount {
    pub unit: String,
    pub count: usize,
}

/// A unit produced while the enemy army counters it, see swarmy's decisions module.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct UnitDecision {
    pub seconds: u32,
    pub unit: String,
    /// From -1 (countered) to 1 (counters the enemy army).
    pub score: f64,
    pub strong_vs: String,
    pub weak_vs: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct Resources {
    pub minerals: u32,
    pub vespene: u32,
}

impl Resources {
    pub fn total(&self) -> u32 {
        self.minerals + self.vespene
    }

    pub fn add(&mut self, other: Resources) {
        self.minerals += other.minerals;
        self.vespene += other.vespene;
    }
}

/// The resources a player lost and killed, from the balance data unit costs.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ResourceSummary {
    pub lost: Resources,
    pub killed: Resources,
    /// The seconds the player needed to mine the lost resources, at its income when they were lost.
    pub mining_seconds_lost: u32,
    /// The build time of the units and structures lost.
    pub build_seconds_lost: u32,
}

/// The idle time of the production structures of a type, between one unit and the next.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProductionIdle {
    pub structure: String,
    /// The structures of this type that produced at least one unit.
    pub structures: usize,
    pub idle_seconds: u32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct MacroSummary {
    /// The time with all the supply used, below the 200 cap.
    pub supply_blocked_seconds: u32,
    /// The time with more than `FLOATING_THRESHOLD` minerals and vespene unspent.
    pub floating_seconds: u32,
    pub avg_unspent: u32,
    pub production_idle: Vec<ProductionIdle>,
    pub injects: u32,
    pub chrono_boosts: u32,
    pub mules: u32,
    /// The minerals and vespene collected per minute, on average.
    #[serde(default)]
    pub avg_income: u32,
    /// The commands issued in each minute of the game.
    pub commands_per_minute: Vec<u32>,
}

/// The economy of a player at a minute of the game.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct Benchmark {
    pub minute: u32,
    pub workers: i32,
    pub supply: i32,
    /// Per minute.
    pub minerals_rate: i32,
    /// Per minute.
    pub vespene_rate: i32,
    /// The minerals and vespene of the current army.
    pub army_value: i32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplayPlayerSummary {
    pub player: ReplayPlayer,
    /// The ladder rating stored in the replay, if any.
    pub rating: Option<i32>,
    pub build_order: Vec<BuildOrderEntry>,
    /// The first tech structures, i.e. Barracks, Factory, Starport.
    pub opening: Vec<String>,
    /// The first army unit produced.
    pub first_army_unit: Option<String>,
    /// The first upgrade researched.
    pub first_upgrade: Option<String>,
    /// The most produced army unit.
    pub favourite_army_unit: Option<String>,
    /// The townhalls started after the initial one.
    pub expansions: Vec<BuildOrderEntry>,
    /// The townhalls this player lost.
    pub bases_lost: Vec<TimedEvent>,
    /// The enemy units killed by this player, by type, most killed first.
    pub units_killed: Vec<UnitCount>,
    /// The units this player lost, by type, most lost first.
    pub units_lost: Vec<UnitCount>,
    /// The enemy units that killed the units of this player, most kills first.
    pub killed_by: Vec<UnitCount>,
    /// The units of this player that killed enemy units, most kills first.
    pub units: Vec<UnitEffectiveness>,
    pub resources: ResourceSummary,
    pub macro_summary: MacroSummary,
    pub focus: PlayerFocus,
    pub benchmarks: Vec<Benchmark>,
    /// The army units alive at 10:00, or at the end of shorter games.
    pub army_at_10: Vec<UnitCount>,
    /// The townhalls alive at 10:00, or at the end of shorter games.
    pub bases_at_10: usize,
    /// The first enemy unit or structure killed.
    pub first_kill_seconds: Option<u32>,
    /// The first command targeting the enemy start location, the text is the unit sent.
    pub first_scout: Option<TimedEvent>,
    /// A production structure built far from the start location early in the game.
    pub proxy: Option<TimedEvent>,
    /// The army value was below 70% of the enemy one at some point after 5:00.
    pub was_behind: bool,
    pub good_decisions: usize,
    pub even_decisions: usize,
    pub bad_decisions: Vec<UnitDecision>,
    /// The abilities with energy or a cooldown used, and the common ones of the race not used, in
    /// the order of [`crate::TRACKED_ABILITIES`].
    #[serde(default)]
    pub abilities: Vec<crate::AbilityUse>,
    /// The injects, chrono boosts and MULEs compared to what the casters could have done.
    #[serde(default)]
    pub ability_efficiency: Vec<AbilityEfficiency>,
    #[serde(default)]
    pub control: ControlSummary,
    /// What cost the player the most in the game, the most costly first, at most
    /// [`MAX_LESSONS`].
    #[serde(default)]
    pub lessons: Vec<Lesson>,
}

/// The lessons of a player in a game.
pub const MAX_LESSONS: usize = 3;

/// Something that cost a player resources in a game, i.e. being supply blocked or idle
/// production, with an estimate of the resources it cost.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct Lesson {
    pub title: String,
    pub detail: String,
    /// The minerals and vespene it cost, estimated, to compare the lessons.
    pub cost: u32,
    /// When it happened, if it happened at a moment.
    pub seconds: Option<u32>,
}

/// The uses of an ability with energy by the casters of a player, compared to the uses their
/// energy allowed: the energy regenerates at 0.7875 per second.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct AbilityEfficiency {
    /// i.e. Inject Larva
    pub ability: String,
    /// i.e. Queen
    pub caster: String,
    pub uses: u32,
    /// The minutes the casters were alive, added.
    pub caster_minutes: f32,
    /// The uses that the energy of the casters allowed.
    pub possible: f32,
}

/// The control groups and the camera of a player.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ControlSummary {
    /// The control groups the player assigned units to, 0 to 9.
    pub groups: Vec<u8>,
    /// Assigning units to a control group, setting or adding.
    pub assigns: u32,
    /// Selecting a control group.
    pub recalls_per_minute: f32,
    /// Moving the camera, with the minimap, the screen edges or a camera hotkey.
    pub camera_moves_per_minute: f32,
}

/// What a player lost in an engagement.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct EngagementLoss {
    pub player: String,
    pub resources: Resources,
    pub supply: f32,
    /// i.e. `12xZergling,2xQueen`
    pub units: String,
    /// The seconds the player needed to mine the lost resources.
    pub mining_seconds: u32,
}

/// Units dying close in time, from the first to the last death.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct Engagement {
    pub start_seconds: u32,
    pub end_seconds: u32,
    pub losses: Vec<EngagementLoss>,
}

/// The chance of the first team to win at a moment of the game, from the economy and the armies
/// of the teams. The players' mistakes are not in it, it is who the game favors at that moment.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct WinChancePoint {
    pub seconds: u32,
    /// From 0 to 1, the chance of the second team is `1 - chance`.
    pub chance: f32,
    /// What favors the first team (positive) or the second one (negative), in log-odds: the army
    /// value, the income, the workers and the resources killed compared to the lost ones.
    pub army: f32,
    pub income: f32,
    pub workers: f32,
    pub trades: f32,
}

/// The win chance over a game of two teams.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct WinChances {
    /// The players of the first and the second team, as in [`ReplayPlayer::name`].
    pub first_team: Vec<String>,
    pub second_team: Vec<String>,
    pub points: Vec<WinChancePoint>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ReplayDetail {
    pub ext_fs_id: u64,
    /// The sha256 of the replay file, the same in every snapshot, the notes are kept by it.
    pub sha256: String,
    pub map_title: String,
    pub replay_date: String,
    pub replay_location: String,
    pub protocol_version: u32,
    pub duration_seconds: u32,
    /// The game second at which `army_at_10` and `bases_at_10` were counted, 600 or the end of
    /// shorter games.
    pub snapshot_seconds: u32,
    pub winners: Vec<String>,
    pub players: Vec<ReplayPlayerSummary>,
    pub engagements: Vec<Engagement>,
    /// The chat messages, the player is the sender.
    pub messages: Vec<TimedEvent>,
    /// None for games that are not between two teams.
    #[serde(default)]
    pub win_chances: Option<WinChances>,
}
