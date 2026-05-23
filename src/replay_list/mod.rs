//! Leptos view for replay lists.

mod actions;
pub mod view;

use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use swarmy_tauri_common::*;

#[derive(Store, Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReplayListDataTable {
    pub total: usize,
    #[store(key: String = |row| row.sha256_sum.clone())]
    pub data: Vec<ReplayList>,
    pub start: usize,
    pub end: usize,
    pub page: usize,
    pub per_page: usize,
}
