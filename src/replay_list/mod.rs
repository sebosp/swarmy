//! Leptos view for replay lists.

pub mod actions;
pub mod view;

use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use swarmy_common::*;

#[derive(Store, Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReplayListDataTable {
    pub total: usize,
    // The replays are unique by id, the sha256_sum is the one of the map.
    #[store(key: u64 = |row| row.ext_fs_id)]
    pub data: Vec<ReplayListEntry>,
    pub start: usize,
    pub end: usize,
    pub page: usize,
    pub per_page: usize,
}
