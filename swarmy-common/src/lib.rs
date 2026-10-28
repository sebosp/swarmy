pub mod settings;
pub use settings::*;
pub mod error;
pub use error::*;
pub mod response;
pub use response::*;
pub mod snapshot_stats;
pub use snapshot_stats::*;
pub mod map_stats;
pub use map_stats::*;
pub mod map_details;
pub use map_details::*;
pub mod matchup;
pub use matchup::*;
pub mod replay_list;
pub use replay_list::*;
pub mod replay_detail;
pub use replay_detail::*;
pub mod player_detail;
pub use player_detail::*;
pub mod map_scout;
pub use map_scout::*;
pub mod patch_overview;
pub use patch_overview::*;
pub mod abilities;
pub use abilities::*;
pub mod race_detail;
pub use race_detail::*;
#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::oneshot;

pub mod dataframe;
#[cfg(not(target_arch = "wasm32"))]
pub use dataframe::*;

pub const DETAILS_IPC: &str = "details.arrow";
pub const INIT_DATA_IPC: &str = "init_data.arrow";
pub const UNIT_BORN_IPC: &str = "unit_born.arrow";
pub const STATS_IPC: &str = "stats.arrow";
pub const UNIT_DIED_IPC: &str = "unit_died.arrow";
pub const UPGRADES_IPC: &str = "upgrades.arrow";
pub const CMD_TARGET_POINT_IPC: &str = "cmd_target_point.arrow";
pub const CMD_TARGET_UNIT_IPC: &str = "cmd_target_unit.arrow";

pub const IPC_DIR: &str = "ipcs";

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub enum AsyncTask {
    Shutdown,
    BasicScanReplayPath(oneshot::Sender<ApiResponse>),
    /// Adds the new replays to the snapshot, `recreate` deletes it first.
    OptimizeReplayPath {
        recreate: bool,
        res_tx: oneshot::Sender<ApiResponse>,
    },
    QueryMapStats {
        map_title: String,
        player_name: String,
        res_tx: oneshot::Sender<ApiResponse>,
    },
    GetSnapshotStats(oneshot::Sender<ApiResponse>),
}
