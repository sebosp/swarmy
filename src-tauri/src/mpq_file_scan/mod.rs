//! Swarmy Tauri UI - SC2Replay Directory Scan and Export to Arrow IPC Module

use s2protocol::arrow_store::ArrowIpcTypes;
use s2protocol::dir_stats::SC2ReplaysDirStats;
use s2protocol::game_events::read_balance_data_from_included_assets;
use std::path::PathBuf;
use swarmy_common::*;
use tauri_plugin_store::StoreBuilder;

use crate::try_get_snapshot_metadata;

#[tauri::command(rename_all = "snake_case")]
pub async fn basic_scan_replay_path(
    app_handle: tauri::AppHandle,
    replay_path: String,
    disable_parallelism: bool,
) -> Result<SC2ReplaysDirStats, String> {
    let store = StoreBuilder::new(&app_handle, "settings.json")
        .build()
        .map_err(|e| {
            tracing::error!("Error building store: {}", e);
            format!("Error building store: {:?}", e)
        })?;

    // If there are no saved settings yet, this will return an error so we ignore the return value.
    let _ = store.reload();

    store.set("disable_parallelism", disable_parallelism);
    store.set("replay_path", replay_path.clone());
    // create a thread to scan the directory in the background:
    let t = std::thread::spawn(move || {
        tracing::info!("Scanning replays directory: {}", replay_path);
        match SC2ReplaysDirStats::from_directory(&replay_path, disable_parallelism) {
            Ok(s) => {
                tracing::info!(
                    "Finished scanning replays directory: {} with res: {:?}",
                    replay_path,
                    s
                );
                Ok(s)
            }
            Err(e) => {
                tracing::error!("Error scanning replays directory: {}", e);
                Err(format!("Error scanning replays directory: {:?}", e))
            }
        }
    });
    t.join().unwrap()
}

#[tauri::command(rename_all = "snake_case")]
pub async fn optimize_replay_path(
    _app_handle: tauri::AppHandle,
    replay_path: String,
    cache_path: String,
    disable_parallelism: bool,
) -> ApiResponse {
    // create a thread to scan the directory in the background:
    let t = std::thread::spawn(move || {
        let init_time = std::time::Instant::now();
        match try_optimize_replay_path(replay_path, cache_path, disable_parallelism) {
            Ok(val) => ApiResponse::new(
                ResponseMetaBuilder::new(true)
                    .duration_ms(init_time.elapsed().as_millis() as u64)
                    .build(),
                serde_json::to_string(&val).unwrap_or_default(),
            ),
            Err(e) => {
                tracing::error!("Error optimizing replays: {}", e);
                ApiResponse::new(
                    ResponseMetaBuilder::new(true)
                        .duration_ms(init_time.elapsed().as_millis() as u64)
                        .build(),
                    format!("Error optimizing replays: {:?}", e),
                )
            }
        }
    });
    t.join().unwrap()
}

#[tracing::instrument(level = "debug")]
fn try_optimize_replay_path(
    replay_path: String,
    cache_path: String,
    disable_parallelism: bool,
) -> Result<SnapshotStats, SwarmyError> {
    let path = PathBuf::from(&replay_path);
    let destination = path.join(PathBuf::from(IPC_DIR));
    if !destination.exists() {
        std::fs::create_dir_all(&destination)?;
    }
    tracing::info!(
        "Optimizing replays directory: {} and storing into {}",
        path.display(),
        destination.display()
    );
    let versioned_abilities = read_balance_data_from_included_assets()?;
    // TODO: Move from cli on s2protocol and create a leptos view to configure this.
    let props = s2protocol::WriteArrowIpcProps {
        scan_max_files: 1000000,
        process_max_files: 100000,
        traverse_max_depth: 8,
        min_version: None,
        max_version: None,
    };
    ArrowIpcTypes::handle_arrow_ipc_cmd(
        path,
        destination,
        &props,
        &versioned_abilities,
        disable_parallelism,
    )?;
    // if the ipc directory exists do basic scan.
    let ipc_path = std::path::Path::new(&replay_path).join(String::from(IPC_DIR.to_string()));
    let arrow_ipc_stats = if ipc_path.exists() && ipc_path.is_dir() {
        match try_get_snapshot_metadata(replay_path, cache_path) {
            Ok(val) => val,
            Err(e) => {
                tracing::error!("Error getting snapshot metadata: {}", e);
                SnapshotStats::default()
            }
        }
    } else {
        SnapshotStats::default()
    };
    Ok(arrow_ipc_stats)
}
