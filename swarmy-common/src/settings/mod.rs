//! Module for application settings management.
use super::snapshot_stats::SnapshotStats;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub disable_parallel_scans: bool,
    pub replay_path: String,
    pub snapshot_stats: SnapshotStats,
}
