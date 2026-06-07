//! Module for application settings management.
use swarmy_common::*;

use crate::try_get_snapshot_metadata;
use tauri_plugin_store::StoreBuilder;

#[tauri::command]
pub async fn get_current_app_config(
    app_handle: tauri::AppHandle,
) -> Result<AppSettings, SwarmyError> {
    load_app_settings(app_handle).await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn set_current_app_config(
    app_handle: tauri::AppHandle,
    app_settings: AppSettings,
) -> Result<(), SwarmyError> {
    let store = StoreBuilder::new(&app_handle, "settings.json").build()?;

    // If there are no saved settings yet, this will return an error so we ignore the return value.
    let _ = store.reload();
    store.set("disable_parallelism", app_settings.disable_parallelism);
    store.set("replay_path", app_settings.replay_path);
    store.set("cache_path", app_settings.cache_path);
    store.set("disable_parallelism", app_settings.disable_parallelism);
    store.set("traverse_max_depth", app_settings.traverse_max_depth);
    store.set("scan_max_files", app_settings.scan_max_files);
    store.set("process_max_files", app_settings.process_max_files);
    store.set("min_version", app_settings.min_version);
    store.set("max_version", app_settings.max_version);
    Ok(())
}

pub async fn load_app_settings(app_handle: tauri::AppHandle) -> Result<AppSettings, SwarmyError> {
    let store = StoreBuilder::new(&app_handle, "settings.json").build()?;

    // If there are no saved settings yet, this will return an error so we ignore the return value.
    let _ = store.reload();
    let disable_parallelism = store
        .get("disable_parallelism")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let replay_path = store
        .get("replay_path")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string();

    let cache_path = store
        .get("cache_path")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string();

    let scan_max_files = store
        .get("scan_max_files")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_SCAN_MAX_FILES);

    let process_max_files = store
        .get("process_max_files")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_PROCESS_MAX_FILES);

    let traverse_max_depth = store
        .get("traverse_max_depth")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_TRAVERSE_MAX_DEPTH);

    let min_version = store
        .get("min_version")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_SCAN_MAX_FILES);

    let max_version = store
        .get("max_version")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_SCAN_MAX_FILES);

    // if the ipc directory exists do basic scan.
    let ipc_path = std::path::Path::new(&replay_path).join(String::from(IPC_DIR.to_string()));
    let arrow_ipc_stats = if ipc_path.exists() && ipc_path.is_dir() {
        let replay_path_cp = replay_path.clone();
        let cache_path_cp = cache_path.clone();
        let t = std::thread::spawn(move || {
            match try_get_snapshot_metadata(replay_path_cp, cache_path_cp) {
                Ok(val) => val,
                Err(e) => {
                    tracing::error!("Error getting snapshot metadata: {}", e);
                    SnapshotStats::default()
                }
            }
        });
        t.join().unwrap()
    } else {
        SnapshotStats::default()
    };

    Ok(AppSettings {
        replay_path,
        cache_path,
        snapshot_stats: arrow_ipc_stats,
        disable_parallelism: disable_parallelism,
        scan_max_files,
        process_max_files,
        traverse_max_depth,
        min_version,
        max_version,
    })
}
