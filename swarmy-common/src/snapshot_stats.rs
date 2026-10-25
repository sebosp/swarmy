use serde::{Deserialize, Serialize};
/// Contains metadata information related to the minimun, maximum date of the snapshot taken, the number of
/// files analyzed, the number of maps and the number of players in the analyzed collection
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotStats {
    /// The size of the IPC files
    pub ipc_dir_size: u64,
    /// The time of the last Optimize.
    pub date_modified: std::time::SystemTime,
    /// The number of games
    pub num_games: u64,
    /// The number of maps in the snapshot
    pub num_maps: u32,
    /// The minimum date of the snapshot taken
    pub min_date: chrono::NaiveDate,
    /// The maximum date of the snapshot taken
    pub max_date: chrono::NaiveDate,
    /// The number of cache files in the snapshot
    pub num_caches: u64,
    /// The size of the file caches in the snapshot
    pub caches_size: u64,
    /// The s2protocol version that generated the snapshot, from its metadata.json, None for
    /// snapshots generated before the metadata existed.
    #[serde(default)]
    pub s2protocol_version: Option<String>,
    /// Whether the snapshot can be read by the s2protocol version of swarmy.
    #[serde(default)]
    pub is_compatible: bool,
    /// The days the snapshot is partitioned by, 0 for a snapshot written before the partitions.
    #[serde(default)]
    pub partitions: usize,
    /// The replays added by the last Optimize, the others were already in the snapshot.
    #[serde(default)]
    pub added_replays: usize,
}

impl Default for SnapshotStats {
    fn default() -> Self {
        SnapshotStats {
            ipc_dir_size: 0,
            date_modified: std::time::SystemTime::UNIX_EPOCH,
            num_maps: 0,
            min_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            max_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            num_games: 0,
            num_caches: 0,
            caches_size: 0,
            s2protocol_version: None,
            is_compatible: false,
            partitions: 0,
            added_replays: 0,
        }
    }
}

/// The Tauri event that carries the [`s2protocol::snapshot::SnapshotProgress`] of Optimize.
pub const OPTIMIZE_PROGRESS_EVENT: &str = "optimize-progress";

/// The Tauri event that carries the [`NewReplaysStatus`] of the replay directory watcher.
pub const NEW_REPLAYS_EVENT: &str = "new-replays";

/// The replays that Optimize would add to the snapshot, found by the replay directory watcher.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct NewReplaysStatus {
    /// Whether the replay directory is watched, the `watch_new_replays` setting.
    pub watching: bool,
    /// The replays of the replay directory that are not in the snapshot.
    pub new_replays: usize,
    /// The days of the snapshot written by another version of s2protocol, Optimize parses their
    /// replays again.
    pub outdated_days: usize,
}

impl NewReplaysStatus {
    /// Whether Optimize would change the snapshot.
    pub fn needs_optimize(&self) -> bool {
        self.new_replays > 0 || self.outdated_days > 0
    }
}
