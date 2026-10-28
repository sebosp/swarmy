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

/// Shows the spots of the map where the Siege Tanks sieged and killed the most in the games of
/// the snapshot. The spots are written as JSON to the temporary directory for swarmy-bevy.
#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_bevy_siege_spots(
    app_handle: AppHandle,
    map_title: String,
    cache_ids: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    let replay_path = app_config.replay_path.clone();
    let spots = tauri::async_runtime::spawn_blocking(move || {
        // The errors are not Send, their message is.
        crate::map_details::siege_spots::try_query_siege_spots(replay_path, map_title)
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| SwarmyError::Other(format!("The siege spots query failed: {err}")))?
    .map_err(SwarmyError::Other)?;
    if spots.spots.is_empty() {
        return Err(SwarmyError::Other(format!(
            "No Siege Tank sieged in the games of {}",
            spots.map_title
        )));
    }
    let file = std::env::temp_dir().join(format!("swarmy-siege-spots-{}.json", std::process::id()));
    std::fs::write(&file, serde_json::to_string(&spots)?)?;
    spawn_swarmy_bevy(
        &app_handle,
        vec![
            "--path".to_string(),
            app_config.cache_path,
            "--ids".to_string(),
            cache_ids,
            "--siege-spots".to_string(),
            file.display().to_string(),
        ],
    )
}

/// The siege spots of a map, for the Map Details view.
#[tauri::command(rename_all = "snake_case")]
pub async fn query_siege_spots(
    app_handle: AppHandle,
    map_title: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    crate::run_query(move || {
        ApiResponseBuilder::new().process_result(
            crate::map_details::siege_spots::try_query_siege_spots(
                app_config.replay_path,
                map_title,
            ),
        )
    })
    .await
}
