//! Runs the swarmy-bevy sidecar, the 3D view of a map, or of a replay on its map.

use crate::host::SwarmyHost;
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

/// A query of the snapshot for swarmy-bevy, run in the remote backend when one is set: the
/// message of the response is the JSON of the result.
async fn query_for_bevy<T: serde::de::DeserializeOwned>(
    app_handle: &AppHandle,
    settings: &AppSettings,
    command: &str,
    map_title: String,
) -> Result<T, SwarmyError> {
    let response = if settings.is_remote() {
        crate::remote::call(
            settings,
            command,
            serde_json::json!({ "map_title": map_title }),
        )
        .await?
    } else if command == "query_siege_spots" {
        query_siege_spots(app_handle, map_title).await?
    } else {
        query_map_layers(app_handle, map_title).await?
    };
    if !response.meta.success {
        return Err(SwarmyError::Other(response.message));
    }
    Ok(serde_json::from_str(&response.message)?)
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

/// Shows the spots of the map where the Siege Tanks sieged and killed the most in the games of
/// the snapshot. The spots are written as JSON to the temporary directory for swarmy-bevy.
#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_bevy_siege_spots(
    app_handle: AppHandle,
    map_title: String,
    cache_ids: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    let spots: SiegeSpots =
        query_for_bevy(&app_handle, &app_config, "query_siege_spots", map_title).await?;
    if spots.spots.is_empty() {
        return Err(SwarmyError::Other(format!(
            "No Siege Tank sieged in the games of {}",
            spots.map_title
        )));
    }
    let file = std::env::temp_dir().join(format!("swarmy-siege-spots-{}.json", std::process::id()));
    std::fs::write(&file, serde_json::to_string(&spots)?)?;
    let caches = map_caches_dir(&app_handle, &app_config, &split_cache_ids(&cache_ids)).await?;
    spawn_swarmy_bevy(
        &app_handle,
        vec![
            "--path".to_string(),
            caches,
            "--ids".to_string(),
            cache_ids,
            "--siege-spots".to_string(),
            file.display().to_string(),
        ],
    )
}

/// Shows what happens where on the map in the games of the snapshot: the fights, proxies, creep
/// tumors and early buildings, written as JSON to the temporary directory for swarmy-bevy.
#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_bevy_map_layers(
    app_handle: AppHandle,
    map_title: String,
    cache_ids: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    let layers: MapLayers =
        query_for_bevy(&app_handle, &app_config, "query_map_layers", map_title).await?;
    let file = std::env::temp_dir().join(format!("swarmy-map-layers-{}.json", std::process::id()));
    std::fs::write(&file, serde_json::to_string(&layers)?)?;
    let caches = map_caches_dir(&app_handle, &app_config, &split_cache_ids(&cache_ids)).await?;
    spawn_swarmy_bevy(
        &app_handle,
        vec![
            "--path".to_string(),
            caches,
            "--ids".to_string(),
            cache_ids,
            "--map-layers".to_string(),
            file.display().to_string(),
        ],
    )
}

/// The layers of a map, for the Map Details view.
pub async fn query_map_layers(
    host: &dyn SwarmyHost,
    map_title: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    crate::run_query(move || {
        ApiResponseBuilder::new().process_result(
            crate::map_details::map_layers::try_query_map_layers(app_config.replay_path, map_title),
        )
    })
    .await
}

/// The siege spots of a map, for the Map Details view.
pub async fn query_siege_spots(
    host: &dyn SwarmyHost,
    map_title: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
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
        assert!(cache_ids
            .iter()
            .all(|id| id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit())));
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
