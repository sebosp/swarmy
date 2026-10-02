//! Swarmy Tauri UI - SC2Replay Directory Scan and Export to Arrow IPC Module

use crate::SetupState;
use crate::majordomo::AsyncTask;
use s2protocol::ArrowIpcTypes;
use s2protocol::game_events::read_balance_data_from_included_assets;
use std::path::PathBuf;
use swarmy_common::*;
use tauri::AppHandle;
use tauri::State;
use tracing::instrument;

use crate::try_get_snapshot_metadata;

#[instrument]
#[tauri::command(rename_all = "snake_case")]
pub async fn basic_scan_replay_path(
    state: State<'_, SetupState>,
    app_handle: AppHandle,
) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = state.majordomo_tx.clone();
    let res = ApiResponseBuilder::new();
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();

    mdp_tx.send(AsyncTask::BasicScanReplayPath(res_tx)).await?;
    let stats = res_rx.await?;
    Ok(res.with_message(serde_json::to_string(&stats)?).build())
}

#[instrument]
#[tauri::command(rename_all = "snake_case")]
pub async fn optimize_replay_path(
    state: State<'_, SetupState>,
    app_handle: AppHandle,
    replay_path: String,
    cache_path: String,
    disable_parallelism: bool,
) -> Result<ApiResponse, SwarmyError> {
    let res = ApiResponseBuilder::new();
    let mdp_tx = state.majordomo_tx.clone();
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();
    // create a thread to scan the directory in the background:
    if let Err(e) = mdp_tx.send(AsyncTask::OptimizeReplayPath(res_tx)).await {
        tracing::error!("Error optimizing replay path.: {}", e);
        return Ok(ApiResponseBuilder::new()
            .with_failure()
            .with_message(format!(
                "Error triggering try_optimize_replay_path: {:?}",
                e
            ))
            .build());
    }
    let snapshot_stats = res_rx.await?;
    Ok(res
        .with_message(serde_json::to_string(&snapshot_stats)?)
        .build())
}

#[tracing::instrument(level = "debug")]
pub async fn try_optimize_replay_path(
    app_handle_clone: AppHandle,
) -> Result<SnapshotStats, SwarmyError> {
    let app_settings = crate::settings::load_app_settings(app_handle_clone)
        .await
        .unwrap();
    let replay_path = app_settings.replay_path.clone();
    let cache_path = app_settings.cache_path.clone();
    let disable_parallelism = app_settings.disable_parallelism;
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
        destination.clone(),
        &props,
        &versioned_abilities,
        disable_parallelism,
        cache_path.clone(),
    )
    .await?;
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
