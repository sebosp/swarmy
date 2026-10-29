//! The details of a single replay: build orders, expansions, kills, decisions and messages.

pub mod data;
pub mod focus;
pub mod lessons;
pub mod navigation;
pub mod opening_answers;
pub mod patch_timings;
pub mod win_chance;
pub mod winner_benchmarks;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::try_query_replay_detail;
use swarmy_common::*;

#[tauri::command(rename_all = "snake_case")]
pub async fn query_replay_detail(
    app_handle: tauri::AppHandle,
    ext_fs_id: u64,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
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

/// The newest replay, or the one before or after the current one, of the locked map and players.
#[tauri::command(rename_all = "snake_case")]
pub async fn query_adjacent_replay(
    app_handle: tauri::AppHandle,
    query: AdjacentReplayQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(navigation::try_adjacent_replay(
            app_config.replay_path,
            query,
        ))
    })
    .await
}
