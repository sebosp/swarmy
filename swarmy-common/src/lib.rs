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
pub mod replay_list;
pub use replay_list::*;

pub mod dataframe;
#[cfg(not(target_arch = "wasm32"))]
pub use dataframe::*;

pub const DETAILS_IPC: &str = "details.arrow";
pub const INIT_DATA_IPC: &str = "init_data.arrow";
pub const UNIT_BORN_IPC: &str = "unit_born.arrow";

pub const IPC_DIR: &str = "ipcs";
pub const CACHES_DIR: &str = "caches";
