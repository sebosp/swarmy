//! Watches the replay directory in the background, with inotify on Linux, and tells the
//! frontend how many replays are not in the snapshot, so that the user runs Optimize.
//!
//! Optimize only parses the new replays and the replays of the days written by another version
//! of s2protocol, see `s2protocol::arrow_store::partitioned::remove_outdated_partitions`.

use crate::settings::load_stored_app_settings;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use s2protocol::arrow_store::partitioned::{is_current_partition, list_partitions, SnapshotIndex};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use swarmy_common::*;
use tauri::{AppHandle, Emitter, Manager};

/// The replays found are counted up to this many files, like the Optimize scan.
const MAX_FILES: usize = 1_000_000;
const MAX_DEPTH: usize = 8;

#[derive(Default)]
struct WatchedDir {
    replay_path: String,
    /// The replays not in the snapshot, by path.
    new_replays: HashSet<PathBuf>,
    outdated_days: usize,
}

impl WatchedDir {
    fn status(&self) -> NewReplaysStatus {
        NewReplaysStatus {
            watching: true,
            new_replays: self.new_replays.len(),
            outdated_days: self.outdated_days,
        }
    }
}

/// The watcher of the replay directory, none when the `watch_new_replays` setting is off.
#[derive(Default)]
pub struct ReplayWatcherState {
    watcher: Mutex<Option<RecommendedWatcher>>,
    dir: Arc<Mutex<WatchedDir>>,
}

fn is_replay(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("SC2Replay"))
}

fn emit_status(app_handle: &AppHandle, status: &NewReplaysStatus) {
    if let Err(err) = app_handle.emit(NEW_REPLAYS_EVENT, status) {
        tracing::error!("Error sending the new replays status: {err}");
    }
}

/// The replays Optimize did not add, copies of another replay or unreadable, in the snapshot
/// directory, one path per line.
const SKIPPED_REPLAYS_FILE: &str = "swarmy-skipped-replays.txt";

fn read_skipped_replays(ipc_path: &Path) -> HashSet<String> {
    std::fs::read_to_string(ipc_path.join(SKIPPED_REPLAYS_FILE))
        .map(|skipped| skipped.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Remembers the replays that Optimize did not add, so that they are not new replays.
pub fn record_skipped_replays(replay_path: &str) {
    let ipc_path = Path::new(replay_path).join(IPC_DIR);
    // The previous ones are skipped again if they are still not in the snapshot.
    let _ = std::fs::remove_file(ipc_path.join(SKIPPED_REPLAYS_FILE));
    let (skipped, _) = find_new_replays(replay_path);
    let mut skipped: Vec<String> = skipped
        .iter()
        .filter_map(|path| path.to_str().map(str::to_string))
        .collect();
    skipped.sort();
    if let Err(err) = std::fs::write(ipc_path.join(SKIPPED_REPLAYS_FILE), skipped.join("\n")) {
        tracing::error!("Unable to write the skipped replays: {err}");
    }
}

/// The replays of the directory that are not in the snapshot nor skipped by Optimize, and the
/// days of the snapshot written by another version of s2protocol.
fn find_new_replays(replay_path: &str) -> (HashSet<PathBuf>, usize) {
    let ipc_path = Path::new(replay_path).join(IPC_DIR);
    let index = SnapshotIndex::read(&ipc_path).unwrap_or_default();
    let skipped = read_skipped_replays(&ipc_path);
    let outdated_days = list_partitions(&ipc_path)
        .unwrap_or_default()
        .iter()
        .filter(|day| !is_current_partition(day))
        .count();
    let new_replays =
        s2protocol::get_matching_files(PathBuf::from(replay_path), MAX_FILES, MAX_DEPTH)
            .unwrap_or_default()
            .into_iter()
            .filter(|path| {
                let path = path.to_str().unwrap_or_default();
                !index.file_names.contains(path) && !skipped.contains(path)
            })
            .collect();
    (new_replays, outdated_days)
}

/// Counts the replays not in the snapshot again, i.e. after Optimize, and tells the frontend.
pub fn refresh(app_handle: &AppHandle) {
    let state = app_handle.state::<ReplayWatcherState>();
    let dir = state.dir.clone();
    let replay_path = dir.lock().unwrap().replay_path.clone();
    if replay_path.is_empty() {
        return;
    }
    let app_handle = app_handle.clone();
    std::thread::spawn(move || {
        let (new_replays, outdated_days) = find_new_replays(&replay_path);
        let status = {
            let mut dir = dir.lock().unwrap();
            // The replay path changed while the replays were counted.
            if dir.replay_path != replay_path {
                return;
            }
            dir.new_replays = new_replays;
            dir.outdated_days = outdated_days;
            dir.status()
        };
        tracing::info!("Replays to optimize in {replay_path}: {status:?}");
        emit_status(&app_handle, &status);
    });
}

/// Starts or stops watching the replay directory from the stored settings.
pub fn configure(app_handle: &AppHandle) {
    let settings = match load_stored_app_settings(app_handle) {
        Ok(settings) => settings,
        Err(err) => {
            tracing::error!("Unable to read the settings for the replay watcher: {err:?}");
            return;
        }
    };
    let state = app_handle.state::<ReplayWatcherState>();
    let mut watcher = state.watcher.lock().unwrap();
    let replay_path = settings.replay_path;
    let enabled = settings.watch_new_replays && Path::new(&replay_path).is_dir();
    if !enabled {
        if watcher.take().is_some() {
            tracing::info!("Stopped watching the replay directory");
        }
        *state.dir.lock().unwrap() = WatchedDir::default();
        emit_status(app_handle, &NewReplaysStatus::default());
        return;
    }
    if watcher.is_some() && state.dir.lock().unwrap().replay_path == replay_path {
        return;
    }
    *state.dir.lock().unwrap() = WatchedDir {
        replay_path: replay_path.clone(),
        ..Default::default()
    };
    let dir = state.dir.clone();
    let event_app_handle = app_handle.clone();
    let ipc_path = Path::new(&replay_path).join(IPC_DIR);
    let handler = move |event: notify::Result<notify::Event>| {
        let event = match event {
            Ok(event) => event,
            Err(err) => {
                tracing::warn!("Replay watcher error: {err}");
                return;
            }
        };
        // New files and files moved into the directory, the snapshot itself is ignored.
        let is_new = matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(notify::event::ModifyKind::Name(_))
        );
        let replays: Vec<PathBuf> = event
            .paths
            .into_iter()
            .filter(|path| is_new && is_replay(path) && !path.starts_with(&ipc_path))
            .filter(|path| path.is_file())
            .collect();
        if replays.is_empty() {
            return;
        }
        let status = {
            let mut dir = dir.lock().unwrap();
            let before = dir.new_replays.len();
            dir.new_replays.extend(replays);
            if dir.new_replays.len() == before {
                return;
            }
            dir.status()
        };
        emit_status(&event_app_handle, &status);
    };
    let mut new_watcher = match notify::recommended_watcher(handler) {
        Ok(new_watcher) => new_watcher,
        Err(err) => {
            tracing::error!("Unable to create the replay watcher: {err}");
            return;
        }
    };
    if let Err(err) = new_watcher.watch(Path::new(&replay_path), RecursiveMode::Recursive) {
        tracing::error!("Unable to watch {replay_path}: {err}");
        return;
    }
    tracing::info!("Watching {replay_path} for new replays");
    *watcher = Some(new_watcher);
    drop(watcher);
    refresh(app_handle);
}

/// The replays to optimize, for the frontend to show when it starts.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_new_replays_status(
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse, SwarmyError> {
    let state = app_handle.state::<ReplayWatcherState>();
    let watching = state.watcher.lock().unwrap().is_some();
    let status = if watching {
        state.dir.lock().unwrap().status()
    } else {
        NewReplaysStatus::default()
    };
    Ok(ApiResponseBuilder::new().process_result(Ok(status)))
}
