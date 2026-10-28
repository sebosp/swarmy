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
    store.set("watch_new_replays", app_settings.watch_new_replays);
    store.set("duplicates_path", app_settings.duplicates_path);
    store.set("notify_new_replays", app_settings.notify_new_replays);
    // The watcher follows the replay path and the watch setting.
    crate::replay_watcher::configure(&app_handle);
    Ok(())
}

/// The stored settings and the stats of the snapshot, for the Config and Scan views.
pub async fn load_app_settings(app_handle: tauri::AppHandle) -> Result<AppSettings, SwarmyError> {
    let mut settings = load_stored_app_settings(&app_handle)?;
    // if the ipc directory exists do basic scan.
    let ipc_path = std::path::Path::new(&settings.replay_path).join(IPC_DIR);
    if ipc_path.exists() && ipc_path.is_dir() {
        let replay_path_cp = settings.replay_path.clone();
        let cache_path_cp = settings.cache_path.clone();
        let t = std::thread::spawn(move || {
            match try_get_snapshot_metadata(replay_path_cp, cache_path_cp) {
                Ok(val) => val,
                Err(e) => {
                    tracing::error!("Error getting snapshot metadata: {}", e);
                    SnapshotStats::default()
                }
            }
        });
        settings.snapshot_stats = t.join().unwrap();
    }
    Ok(settings)
}

/// The stored settings only, without scanning the snapshot, for the commands that only need
/// the paths. The `snapshot_stats` are left empty.
pub fn load_stored_app_settings(app_handle: &tauri::AppHandle) -> Result<AppSettings, SwarmyError> {
    let store = StoreBuilder::new(app_handle, "settings.json").build()?;

    // If there are no saved settings yet, this will return an error so we ignore the return value.
    let _ = store.reload();
    let disable_parallelism = store
        .get("disable_parallelism")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let replay_path = store
        .get("replay_path")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|path| !path.is_empty())
        .unwrap_or_else(default_replay_path)
        .trim_end_matches('/')
        .to_string();

    let cache_path = store
        .get("cache_path")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|path| !path.is_empty())
        .unwrap_or_else(default_cache_path)
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
        .unwrap_or(DEFAULT_MIN_VERSION);

    let max_version = store
        .get("max_version")
        .and_then(|v| v.as_i64())
        .unwrap_or(DEFAULT_MAX_VERSION);

    let watch_new_replays = store
        .get("watch_new_replays")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let notify_new_replays = store
        .get("notify_new_replays")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let duplicates_path = store
        .get("duplicates_path")
        .and_then(|v| v.as_str().map(|s| s.trim_end_matches('/').to_string()))
        .unwrap_or_default();

    Ok(AppSettings {
        replay_path,
        cache_path,
        snapshot_stats: SnapshotStats::default(),
        disable_parallelism,
        scan_max_files,
        process_max_files,
        traverse_max_depth,
        min_version,
        max_version,
        watch_new_replays,
        duplicates_path,
        notify_new_replays,
    })
}

/// The directory of the map caches until one is configured: `swarmy-caches` in the home
/// directory, `%USERPROFILE%\swarmy-caches` on Windows. It is created when it is missing.
pub fn default_cache_path() -> String {
    let Some(home) = dirs::home_dir() else {
        return String::new();
    };
    let path = home.join("swarmy-caches");
    if let Err(err) = std::fs::create_dir_all(&path) {
        tracing::warn!(
            "Unable to create the map caches directory {}: {err}",
            path.display()
        );
    }
    path.display().to_string()
}

/// The directory where StarCraft II saves the replays, until one is configured, empty when it
/// does not exist: `Documents\StarCraft II\Accounts` on Windows (`%USERPROFILE%\Documents`
/// unless the Documents folder was moved) and `Library/Application Support/Blizzard/StarCraft
/// II/Accounts` on macOS. On Linux StarCraft II runs in a Wine prefix, there is no usual place.
pub fn default_replay_path() -> String {
    let accounts = if cfg!(target_os = "windows") {
        dirs::document_dir()
            .or_else(|| dirs::home_dir().map(|home| home.join("Documents")))
            .map(|documents| documents.join("StarCraft II").join("Accounts"))
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()
            .map(|home| home.join("Library/Application Support/Blizzard/StarCraft II/Accounts"))
    } else {
        None
    };
    accounts
        .filter(|path| path.is_dir())
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

/// The versions of swarmy and the libraries it is built with, see build.rs.
#[tauri::command(rename_all = "snake_case")]
pub async fn get_app_versions() -> Result<ApiResponse, SwarmyError> {
    let versions = AppVersions {
        swarmy: env!("CARGO_PKG_VERSION").to_string(),
        s2protocol: env!("SWARMY_S2PROTOCOL_VERSION").to_string(),
        rerun: env!("SWARMY_RERUN_VERSION").to_string(),
        polars: env!("SWARMY_POLARS_VERSION").to_string(),
        tauri: env!("SWARMY_TAURI_VERSION").to_string(),
    };
    Ok(ApiResponseBuilder::new().process_result(Ok(versions)))
}

/// The links the frontend may open, the trailing slash keeps the host fixed to github.com.
const EXTERNAL_URL_PREFIXES: [&str; 4] = [
    "https://github.com/sebosp/",
    // The projects swarmy is based on, referenced in the Home view.
    "https://github.com/Blizzard/s2protocol/",
    "https://github.com/arkx/mpyq/",
    "https://github.com/RFEphemeration/sc2-map-analyzer/",
];

/// Whether the link is one of the project pages, other schemes or hosts could launch local
/// files or protocol handlers.
fn is_allowed_external_url(url: &str) -> bool {
    EXTERNAL_URL_PREFIXES
        .iter()
        .any(|prefix| url.starts_with(prefix))
}

/// Opens a link, i.e. to report issues, in the default browser.
#[tauri::command(rename_all = "snake_case")]
pub async fn open_external_url(
    app_handle: tauri::AppHandle,
    url: String,
) -> Result<ApiResponse, SwarmyError> {
    use tauri_plugin_opener::OpenerExt;
    if !is_allowed_external_url(&url) {
        tracing::warn!("Refused to open {url}");
        return Ok(ApiResponseBuilder::new()
            .with_message(format!(
                "Refused to open {url}, only project links are allowed"
            ))
            .with_failure()
            .build());
    }
    Ok(match app_handle.opener().open_url(&url, None::<&str>) {
        Ok(_) => ApiResponseBuilder::new()
            .with_message(format!("Opened {url}"))
            .with_success()
            .build(),
        Err(err) => ApiResponseBuilder::new()
            .with_message(format!("Unable to open {url}: {err:?}"))
            .with_failure()
            .build(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_only_allows_project_links() {
        assert!(is_allowed_external_url(
            "https://github.com/sebosp/swarmy/issues"
        ));
        assert!(!is_allowed_external_url("file:///usr/bin/xterm"));
        assert!(!is_allowed_external_url("http://github.com/sebosp/swarmy"));
        assert!(!is_allowed_external_url(
            "https://github.com/sebosp.evil.com/"
        ));
        assert!(!is_allowed_external_url("https://github.com/other/repo"));
        assert!(is_allowed_external_url(
            "https://github.com/Blizzard/s2protocol/"
        ));
        assert!(!is_allowed_external_url(
            "https://github.com/Blizzard/other"
        ));
    }
}
