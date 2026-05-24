use serde::{Deserialize, Serialize};
/// Contains metadata information related to the minimun, maximum date of the snapshot taken, the number of
/// files analyzed, the number of maps and the number of players in the analyzed collection
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotStats {
    /// The size of the IPC files
    pub ipc_dir_size: u64,
    /// The time of modification of the details IPC file.
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
        }
    }
}
