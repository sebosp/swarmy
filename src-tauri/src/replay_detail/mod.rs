//! The details of a single replay: build orders, expansions, kills, decisions and messages.

pub mod data;
pub mod focus;
pub mod patch_timings;
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
