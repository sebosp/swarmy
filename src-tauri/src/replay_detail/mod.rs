//! The details of a single replay: build orders, expansions, kills, decisions and messages.

pub mod data;
pub mod focus;
pub mod lessons;
pub mod navigation;
pub mod opening_answers;
pub mod patch_timings;
pub mod report;
pub mod win_chance;
pub mod winner_benchmarks;
use crate::host::SwarmyHost;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::try_query_replay_detail;
use swarmy_common::*;

pub async fn query_replay_detail(
    host: &dyn SwarmyHost,
    ext_fs_id: u64,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .process_result(try_query_replay_detail(app_config.replay_path, ext_fs_id))
    })
    .await
}

/// The most costly lesson of the Main Player Name of Config in a replay file, for the message of
/// a new replay. None when the player is not in the replay or the game had nothing costly.
pub fn try_replay_lesson(
    replay_location: &str,
    main_player_name: &str,
) -> Result<Option<Lesson>, SwarmyError> {
    if main_player_name.is_empty() {
        return Ok(None);
    }
    let detail = data::analyze_replay_cached(replay_location)?;
    Ok(detail
        .players
        .iter()
        .find(|summary| summary.player.name.rsplit("<sp/>").next() == Some(main_player_name))
        .and_then(|summary| summary.lessons.first().cloned()))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn query_replay_lesson(
    app_handle: tauri::AppHandle,
    replay_location: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_replay_lesson(
            &replay_location,
            &app_config.main_player_name,
        ))
    })
    .await
}

/// The game as a single HTML page to share, and its map title.
pub async fn render_replay_report(
    host: &dyn SwarmyHost,
    ext_fs_id: u64,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(
            try_query_replay_detail(app_config.replay_path, ext_fs_id)
                .map(|detail| (report::render_report(&detail), detail.map_title.clone())),
        )
    })
    .await
}

/// Saves the game as a single HTML page to share, the folder and name are chosen in the dialog of
/// the system. The message is the file written, empty when the dialog was cancelled.
#[tauri::command(rename_all = "snake_case")]
pub async fn export_replay_report(
    app_handle: tauri::AppHandle,
    ext_fs_id: u64,
) -> Result<ApiResponse, SwarmyError> {
    use tauri_plugin_dialog::DialogExt;
    let app_config = load_stored_app_settings(&app_handle)?;
    let rendered = if app_config.is_remote() {
        crate::remote::call(
            &app_config,
            "render_replay_report",
            serde_json::json!({ "ext_fs_id": ext_fs_id }),
        )
        .await?
    } else {
        render_replay_report(&app_handle, ext_fs_id).await?
    };
    if !rendered.meta.success {
        return Ok(rendered);
    }
    let (html, map_title): (String, String) = serde_json::from_str(&rendered.message)?;
    let file_name = format!(
        "swarmy {} {ext_fs_id}.html",
        map_title.replace(['/', '\\', ':'], " ")
    );
    let dialog = app_handle
        .dialog()
        .file()
        .set_title("Save the game report")
        .set_file_name(file_name)
        .add_filter("HTML page", &["html"]);
    let picked = tokio::task::spawn_blocking(move || dialog.blocking_save_file()).await?;
    let Some(path) = picked else {
        return Ok(ApiResponseBuilder::new()
            .with_success()
            .with_message(String::new())
            .build());
    };
    let path = path
        .into_path()
        .map_err(|err| SwarmyError::Other(format!("Unsupported file: {err}")))?;
    std::fs::write(&path, html)?;
    Ok(ApiResponseBuilder::new()
        .with_success()
        .with_message(path.display().to_string())
        .build())
}

/// The newest replay, or the one before or after the current one, of the locked map and players.
pub async fn query_adjacent_replay(
    host: &dyn SwarmyHost,
    query: AdjacentReplayQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(navigation::try_adjacent_replay(
            app_config.replay_path,
            query,
        ))
    })
    .await
}
