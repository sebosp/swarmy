use serde::{Deserialize, Serialize};

/// Contains metadata information related to the minimun, maximum date of the map in the snapshot.
/// The cache_handles contain downloadable assets from blizzard's CDN, even tho two maps may have
/// the same title, if their cache_handles differ, they are considered different, maybe different
/// versions, tests, etc.
/// TODO: We should not use the cache_handle IDs joined to dedup them, we should use some internal
/// Information i.e. inside MapInfo or T3HeightMap/etc.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MapStats {
    /// The number of games
    pub num_games: u32,
    /// The name of the map.
    pub title: String,
    /// The cache_handles for the map.
    pub cache_handles: String,
    /// The minimum date of the snapshot taken
    pub min_date: chrono::NaiveDate,
    /// The maximum date of the snapshot taken
    pub max_date: chrono::NaiveDate,
}

impl Default for MapStats {
    fn default() -> Self {
        Self {
            min_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            max_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            num_games: 0,
            title: String::new(),
            cache_handles: String::new(),
        }
    }
}

/// Initial set of query params for the map stats arrow IPC file.
/// XXX: We need to figure out how to handle multiple players.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MapStatsQuery {
    /// The name of the map.
    pub map_title: String,
    /// A player that must have played a game in the map.
    pub player_name: String,
}

/// The parameters for the swarmy-bevy binary.
/// TODO: Potentially we should add a "mode" so that:
/// - One mode shows the map height.
/// - Another mode shows the expansions
/// - Another mode shows the minerals, distances between bases.
/// - Another mode shows the frequency of units per location ? Maybe effective?
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SwarmyBevyMapCacheParams {
    /// The name of the map. I don't know where to read it yet from the Downloaded Caches.
    pub map_title: String,
    /// A string that contains the comma separated list of cacheids to search for t3 height map and
    /// mapinfo
    pub cache_ids: String,
}
