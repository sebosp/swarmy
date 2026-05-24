//! Map stats module.

mod actions;
pub mod view;

use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use swarmy_tauri_common::*;

#[derive(Store, Debug, Default, Clone, Serialize, Deserialize)]
pub struct MapStatsDataTable {
    pub total: usize,
    #[store(key: String = |row| format!("{}:{}",row.title.clone(), row.cache_handles.clone()))]
    pub data: Vec<MapStats>,
    pub start: usize,
    pub end: usize,
    pub page: usize,
    pub per_page: usize,
}
