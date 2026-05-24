//! Map Details module.

use swarmy_tauri_common::*;

#[tauri::command(rename_all = "snake_case")]
pub async fn query_map_details(
    _app_handle: tauri::AppHandle,
    _map_title: String,
    _player_name: String,
    _min_date: String,
    _max_date: String,
) -> ApiResponse {
    unimplemented!()
}
