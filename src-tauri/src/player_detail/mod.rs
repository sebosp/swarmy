//! Statistics of a player: races, results over time, units and opponents.

pub mod data;
pub mod head_to_head;
pub mod insights;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::try_query_player_detail;
use head_to_head::{try_list_players, try_query_head_to_head};
use swarmy_common::*;

#[tauri::command(rename_all = "snake_case")]
pub async fn query_player_detail(
    app_handle: tauri::AppHandle,
    player_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .process_result(try_query_player_detail(app_config.replay_path, player_name))
    })
    .await
}

/// The players of the snapshot, for the player dropdowns.
#[tauri::command(rename_all = "snake_case")]
pub async fn list_players(app_handle: tauri::AppHandle) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_list_players(app_config.replay_path))
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn query_head_to_head(
    app_handle: tauri::AppHandle,
    player_name: String,
    opponent_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    let query = HeadToHeadQuery {
        player_name,
        opponent_name,
    };
    run_query(move || {
        ApiResponseBuilder::new()
            .process_result(try_query_head_to_head(app_config.replay_path, query))
    })
    .await
}
