//! A race in the games of the snapshot: its matchups, the maps that favour it, the units it makes
//! and the abilities it uses.

use crate::{AbilityPatchUsage, GamesWins};
use serde::{Deserialize, Serialize};

/// The races with a Race Detail.
pub const RACE_NAMES: [&str; 3] = ["Terran", "Zerg", "Protoss"];

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct RaceDetailQuery {
    /// Terran, Zerg or Protoss.
    pub race: String,
}

/// The maps need this many games of the matchup to be compared.
pub const RACE_MAP_MIN_GAMES: usize = 5;

/// The wins of the race on a map against each other race, in the 1v1 games that are not mirrors.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct RaceMapStats {
    pub map_title: String,
    /// The games and wins against each of the other races, the label is the matchup, i.e. `ZvT`.
    pub matchups: Vec<GamesWins>,
}

impl RaceMapStats {
    pub fn games(&self) -> usize {
        self.matchups.iter().map(|matchup| matchup.games).sum()
    }

    pub fn wins(&self) -> usize {
        self.matchups.iter().map(|matchup| matchup.wins).sum()
    }

    pub fn win_percent(&self) -> f64 {
        100. * self.wins() as f64 / self.games().max(1) as f64
    }
}

/// How a unit of the race is used in its games.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct RaceUnitStats {
    pub unit: String,
    /// The player games in which at least one was made.
    pub games: usize,
    /// Each unit once, a burrowed Roach is the same Roach.
    pub born: usize,
    /// The units of the other players killed by it.
    pub kills: usize,
    /// The player games of the race in which it was made and won.
    pub wins: usize,
}

impl RaceUnitStats {
    pub fn born_per_game(&self) -> f64 {
        self.born as f64 / self.games.max(1) as f64
    }

    pub fn kills_per_unit(&self) -> f64 {
        self.kills as f64 / self.born.max(1) as f64
    }

    pub fn win_percent(&self) -> f64 {
        100. * self.wins as f64 / self.games.max(1) as f64
    }
}

/// A race in the games of the snapshot.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct RaceDetail {
    pub race: String,
    /// The games of each player of the race, one per player and game.
    pub player_games: usize,
    /// The 1v1 games against each race, the label is the matchup, i.e. `ZvT`.
    pub matchups: Vec<GamesWins>,
    /// The 1v1 games by patch, the label is the patch.
    pub by_patch: Vec<GamesWins>,
    /// The maps with at least [`RACE_MAP_MIN_GAMES`] games, the best win rate first.
    pub maps: Vec<RaceMapStats>,
    /// The units made, the most made per game first.
    pub units: Vec<RaceUnitStats>,
    /// The tracked abilities of the race in all the games of the snapshot, the patch is 0.
    pub abilities: Vec<AbilityPatchUsage>,
    /// The players with the most games of the race.
    pub top_players: Vec<GamesWins>,
}
