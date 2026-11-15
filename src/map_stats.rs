//! Map stats module.

pub mod actions;
pub mod view;

use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use swarmy_common::*;

#[derive(Store, Debug, Default, Clone, Serialize, Deserialize)]
pub struct MapStatsDataTable {
    pub total: usize,
    // The rows are by title and map digest: the maps of the replays whose caches could not be read,
    // and all the AI Arena maps, have an empty digest.
    #[store(key: (String, String) = |row| (row.title.clone(), row.map_info_sha256.clone()))]
    pub data: Vec<MapStats>,
    pub start: usize,
    pub end: usize,
    pub page: usize,
    pub per_page: usize,
}
