use serde::{Deserialize, Serialize};

/// The games to collect the scouting from.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MapScoutQuery {
    /// A substring of the map title.
    pub map_title: String,
    /// A substring of the player name, empty for all players.
    pub player_name: String,
    /// A substring of the commanded unit name, i.e. SCV, Overlord, empty for all units.
    pub unit_name: String,
    /// Only commands issued up to this game second are included.
    pub max_seconds: u32,
    pub layer: ScoutLayer,
}

/// What is drawn on the map.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ScoutLayer {
    /// The targets of the commands, i.e. where units were sent to.
    #[default]
    Commands,
    /// Where the units of the players died.
    Deaths,
}

/// The commands issued to a square of the map.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ScoutCell {
    pub x: u32,
    pub y: u32,
    pub count: u32,
    /// The average game second of the commands.
    pub avg_seconds: f32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct MapScout {
    pub map_title: String,
    pub games: usize,
    pub commands: usize,
    pub width: u32,
    pub height: u32,
    /// The side, in map units, of the square cells.
    pub cell_size: u32,
    /// Command targets, rotated so that the player always starts at `own_start`.
    pub cells: Vec<ScoutCell>,
    pub own_start: Option<(f32, f32)>,
    pub enemy_start: Option<(f32, f32)>,
    /// The terrain, None without the map cache.
    pub terrain: Option<MapTerrain>,
}

/// The terrain levels of a map, to draw it as a minimap.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MapTerrain {
    /// The levels, `width` per row, starting from the bottom of the map.
    pub levels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// The side, in map units, of each level square.
    pub cell_size: u32,
}

/// The versions of the application and the libraries it is built with.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppVersions {
    pub swarmy: String,
    pub s2protocol: String,
    pub rerun: String,
    pub polars: String,
    pub tauri: String,
}
