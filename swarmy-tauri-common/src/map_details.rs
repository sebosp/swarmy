use serde::{Deserialize, Serialize};

/// Contains metadata information related to the minimun, maximum date of the map in the snapshot.
/// The cache_handles contain downloadable assets from blizzard's CDN, even tho two maps may have
/// the same title, if their cache_handles differ, they are considered different, maybe different
/// versions, tests, etc.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MapDetails {
    /// The number of games
    pub num_games: u32,
    /// The name of the map.
    pub title: String,
    /// The long description of the map.
    pub description: String,
    /// The top 10 players in this game.
    pub top_10_players: Vec<(String, usize)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MapPerDateFreq {
    /// The minimum date of the snapshot taken
    pub date: chrono::NaiveDate,
    /// The average duration for a game in seconds.
    pub avg_game_duration_seconds: u64,
}

/// Initial set of query params for the map stats arrow IPC file.
/// XXX: We need to figure out how to handle multiple players.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MapDetailsQuery {
    /// The name of the map.
    pub map_title: String,
    /// A player that must have played a game in the map.
    pub player_name: String,
}
