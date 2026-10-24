//! Module for application settings management.
use super::snapshot_stats::SnapshotStats;
use serde::{Deserialize, Serialize};

pub const DEFAULT_DISABLE_PARALLELISM: bool = false;
pub const DEFAULT_SCAN_MAX_FILES: i64 = 100000i64;
pub const DEFAULT_PROCESS_MAX_FILES: i64 = 100000i64;
pub const DEFAULT_TRAVERSE_MAX_DEPTH: i64 = 16i64;
pub const DEFAULT_MIN_VERSION: i64 = 0i64;
pub const DEFAULT_MAX_VERSION: i64 = 0i64;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    /// The current directory for replay_path.
    /// Altho it may be possible to create separate snapshots...
    pub replay_path: String,
    /// The cache_path, this is shared across replays/snapshots and downloaded from blizzard.
    pub cache_path: String,
    /// Whether to disable parallel scan/optimization
    pub disable_parallelism: bool,
    /// The current snapshots stats from the current replay_path
    pub snapshot_stats: SnapshotStats,
    /// The max files to scan, decrease in case the scan seems stuck.
    /// This is not ideal since there's no ordering to the scan at this point.
    pub scan_max_files: i64,
    /// The max number of files to process for Arrow snapshot generation.
    pub process_max_files: i64,
    /// The maximum number of subdirectories to inspect, decreate in case it seems
    /// that either scan or optimization is stuck, maybe some circular references we may not be
    /// handling properly.
    pub traverse_max_depth: i64,
    /// The minimum SC2 patch version ot consider when creating the snapshot.
    pub min_version: i64,
    /// The maximum SC2 patch version ot consider when creating the snapshot.
    pub max_version: i64,
}
