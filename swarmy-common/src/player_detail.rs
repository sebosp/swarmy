use crate::{ProductionIdle, UnitCount};
use serde::{Deserialize, Serialize};

/// The player to show, as stored in the replay, with the clan tag escaped.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PlayerDetailQuery {
    pub player_name: String,
}

/// Games and wins in a period of time, or for a protocol version.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GamesWins {
    /// The period (`YYYY-MM`), protocol version or race the counts are for.
    pub label: String,
    pub games: usize,
    pub wins: usize,
}

/// A unit type produced by the player and how many enemy units it killed.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct UnitEffectiveness {
    pub unit: String,
    pub produced: usize,
    pub kills: usize,
    /// The minerals and vespene of the enemy units killed.
    pub value_killed: u32,
    /// The minerals and vespene of one unit.
    pub unit_cost: u32,
}

impl UnitEffectiveness {
    pub fn kills_per_unit(&self) -> f64 {
        self.kills as f64 / self.produced.max(1) as f64
    }

    /// The value killed for each mineral and vespene invested in the unit.
    pub fn trade_ratio(&self) -> Option<f64> {
        let invested = self.produced as u32 * self.unit_cost;
        (invested > 0).then(|| self.value_killed as f64 / invested as f64)
    }
}

/// A game of the player against an opponent.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PlayerMatch {
    pub ext_fs_id: u64,
    pub date: String,
    pub map_title: String,
    pub race: String,
    pub opponent: String,
    pub opponent_race: String,
    pub won: bool,
    /// The replay protocol version (base build), 0 if the replay could not be read.
    pub protocol_version: u32,
}

/// The record of the player against an opponent.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct OpponentRecord {
    pub opponent: String,
    pub games: usize,
    pub wins: usize,
}

/// The economy of the player at a minute against the average of the snapshot, same matchup.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct BenchmarkComparison {
    /// i.e. `Zerg vs Terran`
    pub matchup: String,
    pub minute: u32,
    pub games: usize,
    pub workers: f32,
    pub supply: f32,
    /// Minerals and vespene per minute.
    pub income: f32,
    pub army_value: f32,
    pub snapshot_games: usize,
    pub snapshot_workers: f32,
    pub snapshot_supply: f32,
    pub snapshot_income: f32,
    pub snapshot_army_value: f32,
}

/// How often the player made a choice, i.e. an opening, in a context, i.e. a matchup or map.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ChoiceShare {
    pub context: String,
    pub choice: String,
    pub games: usize,
    /// The games of the player in the context.
    pub context_games: usize,
    pub wins: usize,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct UnitAverage {
    pub unit: String,
    pub average: f32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct RatingPoint {
    pub date: String,
    pub rating: i32,
}

/// Statistics from the analysis of every game of the player.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct PlayerInsights {
    pub analyzed_games: usize,
    pub benchmarks: Vec<BenchmarkComparison>,
    pub by_matchup: Vec<GamesWins>,
    pub by_map: Vec<GamesWins>,
    pub by_length: Vec<GamesWins>,
    pub openings_by_matchup: Vec<ChoiceShare>,
    pub openings_by_map: Vec<ChoiceShare>,
    pub first_army_units: Vec<ChoiceShare>,
    /// The most produced army unit of each game, by map.
    pub favourite_units_by_map: Vec<ChoiceShare>,
    pub avg_supply_blocked_seconds: f32,
    pub avg_floating_seconds: f32,
    pub avg_unspent: f32,
    pub avg_commands_per_minute: f32,
    pub avg_injects: f32,
    pub avg_chrono_boosts: f32,
    pub avg_mules: f32,
    /// The idle seconds per game of each type of production structure.
    pub production_idle: Vec<ProductionIdle>,
    pub avg_resources_lost: f32,
    pub avg_resources_killed: f32,
    pub avg_mining_seconds_lost: f32,
    pub avg_first_kill_seconds: Option<f32>,
    pub avg_first_scout_seconds: Option<f32>,
    pub first_scout_units: Vec<UnitCount>,
    pub avg_bases_at_10: f32,
    pub army_at_10: Vec<UnitAverage>,
    /// The games in which the army was below 70% of the enemy one after 5:00.
    pub behind_games: usize,
    /// The games won after being behind.
    pub comeback_wins: usize,
    pub proxy_games: usize,
    /// The games in which both sides lost at least two bases.
    pub base_trade_games: usize,
    /// The first upgrade researched, by game.
    pub upgrade_priorities: Vec<UnitCount>,
    /// The enemy units that killed the units of the player the most.
    pub killed_by: Vec<UnitCount>,
    /// Oldest first.
    pub ratings: Vec<RatingPoint>,
    /// The results of the last games, newest first.
    pub recent_form: Vec<bool>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct PlayerDetail {
    pub player_name: String,
    pub games: usize,
    pub wins: usize,
    /// The races played, most played first.
    pub races: Vec<GamesWins>,
    /// By month, oldest first.
    pub by_month: Vec<GamesWins>,
    /// By protocol version, oldest first.
    pub by_version: Vec<GamesWins>,
    /// The most killing units first.
    pub units: Vec<UnitEffectiveness>,
    /// The most played opponents first.
    pub opponents: Vec<OpponentRecord>,
    /// The newest first.
    pub matches: Vec<PlayerMatch>,
    pub insights: PlayerInsights,
}

/// The games between two players.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct HeadToHeadQuery {
    pub player_name: String,
    pub opponent_name: String,
}

/// What a player did in a game against the other player.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct HeadToHeadSide {
    pub race: String,
    pub won: bool,
    /// The first tech structures.
    pub opening: Vec<String>,
    pub first_army_unit: Option<String>,
    pub favourite_army_unit: Option<String>,
    pub first_upgrade: Option<String>,
    /// When the first expansion was started.
    pub first_expansion_seconds: Option<u32>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct HeadToHeadGame {
    pub ext_fs_id: u64,
    pub date: String,
    pub map_title: String,
    pub duration_seconds: u32,
    pub player: HeadToHeadSide,
    pub opponent: HeadToHeadSide,
}

/// The games between two players, newest first.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct HeadToHead {
    pub player_name: String,
    pub opponent_name: String,
    pub games: Vec<HeadToHeadGame>,
}
