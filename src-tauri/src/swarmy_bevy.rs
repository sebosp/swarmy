//! Runs the swarmy-bevy sidecar, the 3D view of a map, or of a replay on its map.

use crate::settings::load_stored_app_settings;
use swarmy_common::*;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

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

/// The directory of the map caches for swarmy-bevy: the Map Caches Path, or with a remote
/// backend the copy on this computer of its caches, the ones missing are downloaded first.
async fn map_caches_dir(
    app_handle: &AppHandle,
    settings: &AppSettings,
    cache_ids: &[String],
) -> Result<String, SwarmyError> {
    if !settings.is_remote() {
        return Ok(settings.cache_path.clone());
    }
    let dir = crate::remote::local_files_dir(app_handle, settings)?.join("caches");
    crate::remote::download_map_caches(settings, cache_ids, &dir).await?;
    Ok(dir.display().to_string())
}

/// The cache ids of a replay of the snapshot, a comma-separated list.
fn split_cache_ids(cache_ids: &str) -> Vec<String> {
    cache_ids
        .split(',')
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect()
}

/// The cache ids of the map of a replay file, from its init data.
fn replay_cache_ids(replay: &std::path::Path) -> Result<Vec<String>, SwarmyError> {
    let name = replay.display().to_string();
    let (mpq, contents) = s2protocol::read_mpq(&name)
        .map_err(|err| SwarmyError::Other(format!("Unable to read {name}: {err}")))?;
    let init_data = s2protocol::read_init_data(&name, &mpq, &contents)
        .map_err(|err| SwarmyError::Other(format!("Unable to read {name}: {err}")))?;
    Ok(init_data.sync_lobby_state.game_description.cache_handles)
}

/// Shows the map of the cache ids.
#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_bevy_map_caches(
    app_handle: AppHandle,
    cache_ids: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    let caches = map_caches_dir(&app_handle, &app_config, &split_cache_ids(&cache_ids)).await?;
    spawn_swarmy_bevy(
        &app_handle,
        vec!["--path".to_string(), caches, "--ids".to_string(), cache_ids],
    )
}

/// Shows the units born and died of the replay on its map, the replay has the cache ids.
#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_bevy_replay(
    app_handle: AppHandle,
    replay_file_name: String,
    replay_seconds: Option<u32>,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    // With a remote backend the replay is on its computer: it is copied here with its map caches.
    let (caches, replay) = if app_config.is_remote() {
        let dir = crate::remote::local_files_dir(&app_handle, &app_config)?;
        let replay =
            crate::remote::download_replay(&app_config, &replay_file_name, &dir.join("replays"))
                .await?;
        let cache_ids = replay_cache_ids(&replay)?;
        let caches = map_caches_dir(&app_handle, &app_config, &cache_ids).await?;
        (caches, replay.display().to_string())
    } else {
        (app_config.cache_path, replay_file_name)
    };
    let mut args = vec!["--path".to_string(), caches, "--replay".to_string(), replay];
    if let Some(seconds) = replay_seconds {
        args.extend(["--replay-seconds".to_string(), seconds.to_string()]);
    }
    spawn_swarmy_bevy(&app_handle, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_cache_ids_of_a_replay() {
        let replay = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("assets")
            .join("test.SC2Replay");
        let cache_ids = replay_cache_ids(&replay).unwrap();
        assert!(!cache_ids.is_empty());
        assert!(
            cache_ids
                .iter()
                .all(|id| id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
        );
    }

    #[test]
    fn it_splits_the_cache_ids() {
        assert_eq!(
            split_cache_ids("a, b,,"),
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(split_cache_ids("").is_empty());
    }
}
