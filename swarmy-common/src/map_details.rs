use crate::{GamesWins, MapTerrain, ReplayListEntry};
use serde::{Deserialize, Serialize};

/// Contains the statistics of the games played in a map, identified by its title.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct MapDetails {
    /// The name of the map.
    pub title: String,
    /// The long description of the map, from the map cache.
    pub description: String,
    pub num_games: usize,
    /// The date of the first game in the snapshot, `YYYY-MM-DD`.
    pub min_date: String,
    /// The date of the last game in the snapshot, `YYYY-MM-DD`.
    pub max_date: String,
    pub avg_duration_seconds: u32,
    /// The comma separated cache handles, used as swarmy-bevy parameters.
    pub cache_handles: String,
    /// The wins of the first race of the matchup, the races are sorted, i.e. `Terran vs Zerg`,
    /// mirrors only count games.
    pub matchups: Vec<GamesWins>,
    /// The players with the most games, labelled with the name as stored in the replay.
    pub top_players: Vec<GamesWins>,
    /// The newest games first.
    pub recent_games: Vec<ReplayListEntry>,
    /// The terrain, None without the map cache.
    pub terrain: Option<MapTerrain>,
    /// The width and height of the map in map units.
    #[serde(default)]
    pub map_size: (f32, f32),
    /// The minimap of the map cache, None without the map cache.
    #[serde(default)]
    pub minimap: Option<MapImage>,
    /// The resources and start locations, None without the map cache.
    #[serde(default)]
    pub resources: Option<MapResources>,
}

/// Initial set of query params for the map stats arrow IPC file.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MapDetailsQuery {
    /// The title of the map.
    pub map_title: String,
}

/// The minimap image of the map cache, covering the playable area.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct MapImage {
    /// The PNG image as a data URL.
    pub data_url: String,
    /// The playable area in map units, the image covers it.
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

/// A mineral field or a geyser of the map.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct MapResource {
    pub unit_type: String,
    pub x: f32,
    pub y: f32,
    pub is_geyser: bool,
    /// Gold mineral fields and rich vespene geysers, faster to mine.
    pub is_rich: bool,
    /// The resources it contains, the default of its type unless the map changes it.
    pub amount: u32,
}

/// The main base resources of a start location.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct StartLocation {
    pub name: String,
    pub x: f32,
    pub y: f32,
    /// The mineral fields of the main base, the closest ones to the start location.
    pub mineral_fields: usize,
    /// The average distance from the start location to its mineral fields, in map units.
    pub avg_mineral_distance: f32,
    pub min_mineral_distance: f32,
    pub max_mineral_distance: f32,
    /// The average distance from the start location to its geysers.
    pub avg_geyser_distance: f32,
}

/// The resources placed in the map, from the Objects of the map cache.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct MapResources {
    pub resources: Vec<MapResource>,
    pub mineral_fields: usize,
    pub rich_mineral_fields: usize,
    pub geysers: usize,
    pub rich_geysers: usize,
    /// The minerals of all the mineral fields.
    pub minerals: u32,
    /// The vespene of all the geysers.
    pub vespene: u32,
    /// The resources whose amount is the default of their type, not set by the map.
    pub default_amounts: usize,
    pub start_locations: Vec<StartLocation>,
}

/// The map cells grouped in a siege spot, a square of this many cells per side.
pub const SIEGE_SPOT_CELLS: u32 = 4;

/// Where Siege Tanks sieged on a map and the enemy units they killed while sieged there, from
/// the games of the snapshot. swarmy-bevy reads it as JSON with `--siege-spots`.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct SiegeSpot {
    /// The spot, the map cell divided by [`SIEGE_SPOT_CELLS`].
    pub x: u32,
    pub y: u32,
    /// The times a tank sieged in the spot.
    pub sieges: usize,
    /// The enemy units that cost resources killed by the tanks sieged in the spot.
    pub kills: usize,
    /// The games with a siege in the spot.
    pub games: usize,
}

impl SiegeSpot {
    pub fn kills_per_siege(&self) -> f64 {
        self.kills as f64 / self.sieges.max(1) as f64
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct SiegeSpots {
    pub map_title: String,
    /// [`SIEGE_SPOT_CELLS`]
    pub spot_cells: u32,
    /// The games of the map with at least one siege.
    pub games: usize,
    pub sieges: usize,
    pub kills: usize,
    /// The most kills first.
    pub spots: Vec<SiegeSpot>,
}
