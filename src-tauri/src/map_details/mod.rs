//! Map Details module.

pub mod data;
pub mod resources;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::{try_list_maps, try_query_map_details};
use swarmy_common::*;

#[tauri::command(rename_all = "snake_case")]
pub async fn query_map_details(
    app_handle: tauri::AppHandle,
    map_title: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_map_details(
            app_config.replay_path,
            app_config.cache_path,
            map_title,
        ))
    })
    .await
}

/// The maps of the snapshot, for the map dropdown.
#[tauri::command(rename_all = "snake_case")]
pub async fn list_maps(app_handle: tauri::AppHandle) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_list_maps(app_config.replay_path))
    })
    .await
}
