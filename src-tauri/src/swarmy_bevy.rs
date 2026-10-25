//! Runs the swarmy-bevy sidecar, the 3D view of a map, or of a replay on its map.

use crate::settings::load_stored_app_settings;
use swarmy_common::*;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

/// The sidecar name, tauri resolves it to `assets/swarmy-bevy-<target-triple>` at bundle time.
pub const SWARMY_BEVY_SIDECAR: &str = "swarmy-bevy";

/// Spawns swarmy-bevy with the arguments, it runs in its own window until the user closes it.
/// The errors of swarmy-bevy, i.e. the map cache is missing, are logged.
fn spawn_swarmy_bevy(
    app_handle: &AppHandle,
    args: Vec<String>,
) -> Result<ApiResponse, SwarmyError> {
    tracing::info!("Spawning swarmy-bevy with args: {args:?}");
    let (mut rx, child) = app_handle
        .shell()
        .sidecar(SWARMY_BEVY_SIDECAR)
        .map_err(|e| SwarmyError::Other(format!("Unable to create swarmy-bevy sidecar: {e}")))?
        .args(args)
        .spawn()
        .map_err(|e| SwarmyError::Other(format!("Unable to spawn swarmy-bevy sidecar: {e}")))?;
    let pid = child.pid();
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stderr(line) => {
                    tracing::debug!("swarmy-bevy: {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Error(err) => tracing::error!("swarmy-bevy: {err}"),
                CommandEvent::Terminated(payload) => {
                    tracing::info!("swarmy-bevy pid {pid} terminated: {payload:?}");
                    break;
                }
                _ => {}
            }
        }
    });
    Ok(ApiResponseBuilder::new()
        .with_status(true)
        .with_message(format!("Started swarmy-bevy, pid {pid}"))
        .build())
}

/// Shows the map of the cache ids.
#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_bevy_map_caches(
    app_handle: AppHandle,
    cache_ids: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    spawn_swarmy_bevy(
        &app_handle,
        vec![
            "--path".to_string(),
            app_config.cache_path,
            "--ids".to_string(),
            cache_ids,
        ],
    )
}

/// Shows the units born and died of the replay on its map, the replay has the cache ids.
#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_bevy_replay(
    app_handle: AppHandle,
    replay_file_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    spawn_swarmy_bevy(
        &app_handle,
        vec![
            "--path".to_string(),
            app_config.cache_path,
            "--replay".to_string(),
            replay_file_name,
        ],
    )
}
