//! Rerun Viewer sidecar management and replay streaming.
//!
//! A single `rerun-cli` sidecar instance is kept alive and shared across "Load Rerun Preview"
//! requests. Each request opens a new recording on that viewer and streams the replay events
//! through the gRPC proxy.

use super::decisions::{balance_units, closest_balance_units};
use super::terrain::Terrain;
use super::{ReplayRerunLogger, NEUTRAL_PLAYER};
use s2protocol::basic_replay_data::SC2ReplayBasicData;
use s2protocol::{SC2EventIterator, SC2EventType};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use swarmy_common::{clean_player_name_clan_escapes, SwarmyError};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::Mutex;

/// The sidecar name, tauri resolves it to `assets/rerun-cli-<target-triple>` at bundle time.
pub const RERUN_CLI_SIDECAR: &str = "rerun-cli";

/// The Rerun application id, recordings are grouped under it in the viewer.
pub const RERUN_APP_ID: &str = "swarmy";

/// The game loops per second at "Faster" game speed.
pub const GAME_LOOPS_PER_SECOND: f64 = 22.4;

/// How long to wait for a freshly spawned viewer to start accepting connections.
const VIEWER_STARTUP_TIMEOUT: Duration = Duration::from_secs(20);

/// The state of the Rerun Viewer sidecar, there must be only one instance running.
#[derive(Default)]
pub struct RerunViewerState {
    child: Mutex<Option<CommandChild>>,
}

impl RerunViewerState {
    /// Kills the viewer sidecar if it was spawned by us.
    pub fn kill(&self) {
        if let Some(child) = self
            .child
            .try_lock()
            .ok()
            .and_then(|mut child| child.take())
        {
            tracing::info!("Killing rerun-cli sidecar pid {}", child.pid());
            let _ = child.kill();
        }
    }
}

fn viewer_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], rerun::DEFAULT_SERVER_PORT))
}

fn viewer_url() -> String {
    format!(
        "rerun+http://127.0.0.1:{}/proxy",
        rerun::DEFAULT_SERVER_PORT
    )
}

/// Returns true if something is accepting connections on the viewer port.
fn is_viewer_listening() -> bool {
    TcpStream::connect_timeout(&viewer_addr(), Duration::from_millis(200)).is_ok()
}

async fn wait_for_viewer(timeout: Duration) -> Result<(), SwarmyError> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if is_viewer_listening() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Err(SwarmyError::Other(format!(
        "Rerun Viewer did not start listening on {} after {:?}",
        viewer_addr(),
        timeout
    )))
}

/// Spawns the rerun-cli sidecar unless an instance is already running.
pub async fn ensure_viewer_running(app_handle: &AppHandle) -> Result<(), SwarmyError> {
    let state = app_handle.state::<RerunViewerState>();
    let mut child = state.child.lock().await;
    if child.is_some() {
        tracing::info!("Reusing running rerun-cli sidecar");
    } else if is_viewer_listening() {
        // Started outside of swarmy, i.e. manually, we can stream to it all the same.
        tracing::info!("Reusing existing Rerun Viewer on {}", viewer_addr());
    } else {
        let port = rerun::DEFAULT_SERVER_PORT.to_string();
        let (mut rx, cmd_child) = app_handle
            .shell()
            .sidecar(RERUN_CLI_SIDECAR)
            .map_err(|e| SwarmyError::Other(format!("Unable to create rerun-cli sidecar: {e}")))?
            .args(["--port", &port])
            .spawn()
            .map_err(|e| SwarmyError::Other(format!("Unable to spawn rerun-cli sidecar: {e}")))?;
        tracing::info!("Spawned rerun-cli sidecar pid {}", cmd_child.pid());
        *child = Some(cmd_child);
        // Forget the child once it exits so that the next request spawns a new one.
        let app_handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                        tracing::debug!("rerun-cli: {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Error(err) => tracing::error!("rerun-cli: {err}"),
                    CommandEvent::Terminated(payload) => {
                        tracing::info!("rerun-cli sidecar terminated: {:?}", payload);
                        *app_handle.state::<RerunViewerState>().child.lock().await = None;
                        break;
                    }
                    _ => {}
                }
            }
        });
    }
    drop(child);
    wait_for_viewer(VIEWER_STARTUP_TIMEOUT).await
}

/// The terrain is optional, the cache handles may not have been downloaded.
fn load_terrain(cache_path: &str, cache_handles: &[String]) -> Option<Terrain> {
    match Terrain::load(cache_path, cache_handles) {
        Ok(terrain) => Some(terrain),
        Err(err) => {
            tracing::warn!("Unable to load the map terrain from {cache_path}: {err:?}");
            None
        }
    }
}

/// The whole replay is sent at once, the default batcher flushes on every 2 MiB logged which
/// splits each entity into thousands of tiny chunks, bigger batches are much smaller on the wire.
fn bulk_batcher_config() -> rerun::log::ChunkBatcherConfig {
    rerun::log::ChunkBatcherConfig {
        flush_tick: Duration::from_secs(2),
        flush_num_bytes: 64 * 1024 * 1024,
        ..rerun::log::ChunkBatcherConfig::DEFAULT
    }
}

/// Streams the replay events into a new recording on the running Rerun Viewer.
/// The map terrain is loaded from the cache handles in `cache_path` when available.
/// Returns the number of events sent.
pub fn stream_replay_to_viewer(
    replay_file_name: &str,
    cache_path: &str,
) -> Result<usize, SwarmyError> {
    let source = PathBuf::from(replay_file_name);
    let basic_data = SC2ReplayBasicData::new(&source, 0)?;
    let mut event_iterator = SC2EventIterator::new(&basic_data, balance_units()?)?;
    let recording_name = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| replay_file_name.to_string());
    let recording_stream = rerun::RecordingStreamBuilder::new(RERUN_APP_ID)
        .recording_name(recording_name)
        .batcher_config(bulk_batcher_config())
        .connect_grpc_opts(viewer_url())?;
    let mut logger = ReplayRerunLogger::new(
        recording_stream.clone(),
        &event_iterator.sc2_state,
        closest_balance_units(basic_data.init_data.version)?,
        load_terrain(cache_path, &basic_data.details.cache_handles),
    )?;

    let mut total_events = 0usize;
    // Not a for loop, the iterator state is needed to snapshot the alive units.
    while let Some(item) = event_iterator.next() {
        let game_loop = match &item.event_type {
            SC2EventType::Tracker { tracker_loop, .. } => *tracker_loop,
            SC2EventType::Game { game_loop, .. } => *game_loop,
        };
        logger.advance_to(game_loop, &event_iterator.sc2_state)?;
        match &item.event_type {
            SC2EventType::Tracker { event, .. } => {
                logger.add_tracker_event(event, &item.change_hint, &event_iterator.sc2_state)?;
            }
            SC2EventType::Game {
                user_id,
                player_name,
                event,
                ..
            } => {
                let player_name = match player_name {
                    Some(name) => clean_player_name_clan_escapes(name),
                    None => NEUTRAL_PLAYER.to_string(),
                };
                logger.add_game_event(*user_id, &player_name, event, &item.change_hint)?;
            }
        }
        total_events += 1;
    }
    logger.finish(&event_iterator.sc2_state)?;
    recording_stream
        .flush_blocking()
        .map_err(|e| SwarmyError::Other(format!("Unable to flush rerun recording: {e}")))?;
    Ok(total_events)
}
