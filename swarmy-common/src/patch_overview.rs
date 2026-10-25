use serde::{Deserialize, Serialize};

/// A unit or structure available in the balance data.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchUnitName {
    pub name: String,
    /// Terran, Zerg, Protoss or Other.
    pub race: String,
}

/// The patches and units in the s2protocol balance data.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchUnitList {
    /// The protocol versions, oldest first.
    pub versions: Vec<u32>,
    /// The units that cost resources in any patch, by race and name.
    pub units: Vec<PatchUnitName>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchUnitQuery {
    pub unit_name: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchChangesQuery {
    pub version: u32,
}

/// Whether more of the property makes the unit more expensive or better.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum PatchRowKind {
    /// i.e. minerals, build time, cooldown, requirements: more costs more.
    Cost,
    /// i.e. life, armor, range: more is better.
    Benefit,
    /// i.e. attributes, strong vs.
    #[default]
    Neutral,
}

/// How a value changed from the previous patch.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum PatchChange {
    #[default]
    Same,
    /// A number went up, or items were added to a list.
    Increased,
    /// A number went down, or items were removed from a list.
    Decreased,
    /// Changed in a way that is not up or down.
    Changed,
    /// Not in the previous patch.
    Added,
    /// Not in this patch any more.
    Removed,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchValue {
    /// `-` when the unit or property is not in the patch.
    pub value: String,
    pub change: PatchChange,
}

/// A property of the unit, i.e. Life, with its value in each patch.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchUnitRow {
    pub label: String,
    /// The unit the row is about, i.e. for "Strong vs" one row per countered unit.
    #[serde(default)]
    pub unit: Option<String>,
    pub kind: PatchRowKind,
    /// One per version.
    pub values: Vec<PatchValue>,
}

/// A unit and its properties in every patch.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchUnit {
    pub name: String,
    pub race: String,
    /// The protocol versions, oldest first, the row values are in the same order.
    pub versions: Vec<u32>,
    pub rows: Vec<PatchUnitRow>,
}

/// A property of a unit that changed in a patch.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchUnitChange {
    pub unit: String,
    pub race: String,
    pub label: String,
    pub kind: PatchRowKind,
    pub previous: String,
    pub value: String,
    pub change: PatchChange,
}

/// The changes of a patch compared to the previous one.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PatchChanges {
    pub version: u32,
    /// None for the oldest patch in the balance data.
    pub previous_version: Option<u32>,
    pub changes: Vec<PatchUnitChange>,
}

/// The use of a unit in the games of a month of the snapshot.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct UnitMonthUsage {
    /// `YYYY-MM`
    pub month: String,
    /// The games of the snapshot in the month.
    pub games: usize,
    /// The games in which at least one player made the unit.
    pub games_with_unit: usize,
    /// The units made in the month.
    pub born: usize,
}

impl UnitMonthUsage {
    /// The percentage of the games in which the unit was made.
    pub fn games_percent(&self) -> f64 {
        100. * self.games_with_unit as f64 / self.games.max(1) as f64
    }

    /// The units made on average in the games in which it was made.
    pub fn born_per_game(&self) -> f64 {
        self.born as f64 / self.games_with_unit.max(1) as f64
    }
}

/// A player must make at least these many units in a game for its kills per unit to be compared,
/// one unit that killed a worker is not exceptional.
pub const EXCEPTIONAL_MIN_BORN: usize = 3;

/// The exceptional games shown.
pub const EXCEPTIONAL_GAMES_LIMIT: usize = 20;

/// A player that made the unit in a game in which it killed exceptionally many enemy units.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ExceptionalUnitGame {
    pub ext_fs_id: u64,
    pub replay_location: String,
    pub map_title: String,
    /// `YYYY-MM-DD`
    pub date: String,
    pub protocol_version: u32,
    /// The name as stored in the replay, with the clan tag escaped.
    pub player_name: String,
    pub born: usize,
    pub kills: usize,
    /// The minerals and vespene of the enemy units killed.
    pub value_killed: u32,
}

impl ExceptionalUnitGame {
    pub fn kills_per_unit(&self) -> f64 {
        self.kills as f64 / self.born.max(1) as f64
    }
}

/// How a unit was used in the games of the snapshot.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct UnitSnapshotStats {
    pub unit_name: String,
    /// The games of the snapshot.
    pub games: usize,
    /// The games in which at least one player made the unit.
    pub games_with_unit: usize,
    /// The units made, each unit once, i.e. a burrowed Roach is the same Roach.
    pub born: usize,
    /// The units that died.
    pub died: usize,
    /// The enemy units killed by the unit, the ones that cost resources.
    pub kills: usize,
    /// The minerals and vespene of the enemy units killed.
    pub value_killed: u32,
    /// The minerals and vespene of one unit, from the latest balance data.
    pub unit_cost: u32,
    pub by_month: Vec<UnitMonthUsage>,
    /// The average kills per unit of the players that made at least [`EXCEPTIONAL_MIN_BORN`] of
    /// the unit in a game.
    pub kills_per_unit_mean: f64,
    /// The kills per unit above which a game is exceptional, two standard deviations above the
    /// mean.
    pub kills_per_unit_threshold: f64,
    /// The exceptional games, the most kills per unit first.
    pub exceptional: Vec<ExceptionalUnitGame>,
}

impl UnitSnapshotStats {
    pub fn kills_per_unit(&self) -> f64 {
        self.kills as f64 / self.born.max(1) as f64
    }

    pub fn deaths_per_unit(&self) -> f64 {
        self.died as f64 / self.born.max(1) as f64
    }

    /// The value killed for each mineral and vespene invested in the unit.
    pub fn value_per_cost(&self) -> f64 {
        self.value_killed as f64 / (self.born as f64 * self.unit_cost as f64).max(1.)
    }
}
