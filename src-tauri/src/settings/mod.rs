//! Module for application settings management.
use swarmy_common::*;

use crate::try_get_snapshot_metadata;
use tauri_plugin_store::StoreBuilder;

pub async fn load_app_settings(app_handle: tauri::AppHandle) -> Result<AppSettings, SwarmyError> {
    let store = StoreBuilder::new(&app_handle, "settings.json").build()?;

    // If there are no saved settings yet, this will return an error so we ignore the return value.
    let _ = store.reload();
    let disable_parallel_scans = store
        .get("disable_parallel_scans")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let replay_path = store
        .get("replay_path")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    // if the ipc directory exists do basic scan.
    let replay_path = replay_path.trim_end_matches('/').to_string();
    let ipc_path = std::path::Path::new(&replay_path).join(String::from(IPC_DIR.to_string()));
    let arrow_ipc_stats = if ipc_path.exists() && ipc_path.is_dir() {
        let replay_path_cp = replay_path.clone();
        let t = std::thread::spawn(move || match try_get_snapshot_metadata(replay_path_cp) {
            Ok(val) => val,
            Err(e) => {
                log::error!("Error getting snapshot metadata: {}", e);
                SnapshotStats::default()
            }
        });
        t.join().unwrap()
    } else {
        SnapshotStats::default()
    };

    Ok(AppSettings {
        disable_parallel_scans,
        replay_path,
        snapshot_stats: arrow_ipc_stats,
    })
}
