use serde::{Deserialize, Serialize};

/// Contains metadata information related to the minimun, maximum date of the map in the snapshot.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ReplayList {
    /// The title of the map
    pub map_title: String,
    /// The game date
    pub replay_date: chrono::NaiveDate,
    /// The list of players in this replay
    pub player_list: Vec<String>,
    /// The sha256sum of the replay
    pub sha256_sum: String,
    /// The replay location
    pub replay_location: String,
    /// The duration of the game
    pub duration: i32,
    /// The player(s) that won the game:
    pub winner_list: Vec<String>,
}

/// Initial set of query params for the map stats arrow IPC file.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ReplayListQuery {
    /// The name of the map.
    pub map_title: String,
    /// A player that must have played a game in the map.
    pub player_name: String,
    /// The minimum date of the snapshot taken
    pub min_date: chrono::NaiveDate,
    /// The maximum date of the snapshot taken
    pub max_date: chrono::NaiveDate,
}
