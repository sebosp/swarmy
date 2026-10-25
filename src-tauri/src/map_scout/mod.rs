//! The scouting patterns of the players in a map.

pub mod data;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::try_query_map_scout;
use swarmy_common::*;

#[tauri::command(rename_all = "snake_case")]
pub async fn query_map_scout(
    app_handle: tauri::AppHandle,
    map_title: String,
    player_name: String,
    unit_name: String,
    max_seconds: u32,
    layer: ScoutLayer,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    let query = MapScoutQuery {
        map_title,
        player_name,
        unit_name,
        max_seconds,
        layer,
    };
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_map_scout(
            app_config.replay_path,
            app_config.cache_path,
            query,
        ))
    })
    .await
}
