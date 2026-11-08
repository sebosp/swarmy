//! Swarmy Tauri UI - SC2Replay Directory Scan and Export to Arrow IPC Module

use crate::host::SwarmyHost;
use crate::majordomo::AsyncTask;
use s2protocol::game_events::read_balance_data_from_included_assets;
use s2protocol::snapshot::SnapshotProgress;
use s2protocol::ArrowIpcTypes;
use std::path::PathBuf;
use std::sync::Arc;
use swarmy_common::*;
use tracing::instrument;

use crate::try_get_snapshot_metadata;

#[instrument(level = "debug", skip(host))]
pub async fn basic_scan_replay_path(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = host.majordomo();
    let res = ApiResponseBuilder::new();
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();

    mdp_tx.send(AsyncTask::BasicScanReplayPath(res_tx)).await?;
    let stats = res_rx.await?;
    Ok(res.with_message(stats.message).build())
}

#[instrument(skip(host))]
pub async fn optimize_replay_path(
    host: &dyn SwarmyHost,
    replay_path: String,
    cache_path: String,
    disable_parallelism: bool,
    recreate: bool,
) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = host.majordomo();
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();
    // create a thread to scan the directory in the background:
    if let Err(e) = mdp_tx
        .send(AsyncTask::OptimizeReplayPath { recreate, res_tx })
        .await
    {
        tracing::error!("Error optimizing replay path.: {}", e);
        return Ok(ApiResponseBuilder::new()
            .with_failure()
            .with_message(format!(
                "Error triggering try_optimize_replay_path: {:?}",
                e
            ))
            .build());
    }
    // The response is a success or a failure of the snapshot already, with the stats as message.
    Ok(res_rx.await?)
}

#[tracing::instrument(level = "debug")]
/// Adds the replays that are not in the snapshot yet, `recreate` deletes the snapshot first.
pub async fn try_optimize_replay_path(
    host: Arc<dyn SwarmyHost>,
    recreate: bool,
) -> Result<SnapshotStats, SwarmyError> {
    let emitter = host.clone();
    let app_settings = crate::settings::load_stored_app_settings(host.as_ref())?;
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
        recreate,
    };
    // The progress of the snapshot is forwarded to the frontend as events.
    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel();
    let emit_progress =
        move |host: &dyn SwarmyHost, progress: &SnapshotProgress| match serde_json::to_value(
            progress,
        ) {
            Ok(progress) => host.emit(OPTIMIZE_PROGRESS_EVENT, progress),
            Err(err) => tracing::error!("Error sending the optimize progress: {}", err),
        };
    let forward_host = emitter.clone();
    let forward_progress = tokio::spawn(async move {
        // The end of the snapshot is not the end of Optimize, the games are analysed after it.
        let mut done = None;
        while let Some(progress) = progress_rx.recv().await {
            match progress {
                SnapshotProgress::Done { .. } => done = Some(progress),
                SnapshotProgress::Step { index, total, name } => emit_progress(
                    forward_host.as_ref(),
                    &SnapshotProgress::Step {
                        index,
                        total: total + 1,
                        name,
                    },
                ),
                progress => emit_progress(forward_host.as_ref(), &progress),
            }
        }
        done
    });
    // The snapshot reads the replays without yielding until the map caches, on a worker of the
    // runtime it would block the progress forwarding spawned from it, so nothing would be shown
    // until every new replay header was read. It runs on a blocking thread instead.
    let runtime = tokio::runtime::Handle::current();
    let snapshot_destination = destination.clone();
    let snapshot_cache_path = cache_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        runtime
            .block_on(ArrowIpcTypes::handle_arrow_ipc_cmd_with_progress(
                path,
                snapshot_destination,
                &props,
                &versioned_abilities,
                disable_parallelism,
                snapshot_cache_path,
                Some(progress_tx),
            ))
            .map(|_| ())
            .map_err(|err| format!("Error optimizing the replays: {err}"))
    })
    .await
    .unwrap_or_else(|err| Err(format!("The optimize task failed: {err}")));
    // The sender is dropped with the snapshot, so the forwarding ends after the last update.
    let done = forward_progress.await.ok().flatten();
    result.map_err(SwarmyError::Other)?;
    // What the Player view reads of each game, so that it does not read the replays of a player.
    let insights_host = emitter.clone();
    let insights_path = destination.clone();
    tokio::task::spawn_blocking(move || {
        let host = insights_host;
        emit_progress(
            host.as_ref(),
            &SnapshotProgress::Step {
                index: 7,
                total: 7,
                name: "Analysing each game for the Player view".to_string(),
            },
        );
        let report = |done: usize, total: usize| {
            emit_progress(
                host.as_ref(),
                &SnapshotProgress::ReplaysProcessed { done, total },
            );
        };
        crate::player_detail::game_insights::build_missing(&insights_path, &report)
            .inspect_err(|err| tracing::error!("Unable to analyse the games: {err:?}"))
            .ok();
        if let Some(done) = done {
            emit_progress(host.as_ref(), &done);
        }
    })
    .await
    .map_err(|err| SwarmyError::Other(format!("The analysis of the games failed: {err}")))?;
    // The replays added are no longer new, the copies and unreadable ones are not either.
    crate::replay_watcher::record_skipped_replays(&replay_path);
    host.replays_changed();
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
