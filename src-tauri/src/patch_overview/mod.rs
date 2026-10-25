//! The units of each patch, from the s2protocol multi versioned balance data.

pub mod data;
pub mod unit_stats;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::{try_list_patch_units, try_query_patch_changes, try_query_unit_overview};
use swarmy_common::*;
use unit_stats::try_query_unit_snapshot_stats;

#[tauri::command(rename_all = "snake_case")]
pub async fn list_patch_units() -> Result<ApiResponse, SwarmyError> {
    Ok(ApiResponseBuilder::new().process_result(try_list_patch_units()))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn query_unit_overview(unit_name: String) -> Result<ApiResponse, SwarmyError> {
    Ok(ApiResponseBuilder::new().process_result(try_query_unit_overview(unit_name)))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn query_patch_changes(version: u32) -> Result<ApiResponse, SwarmyError> {
    Ok(ApiResponseBuilder::new().process_result(try_query_patch_changes(version)))
}

/// How the unit was used in the games of the snapshot.
#[tauri::command(rename_all = "snake_case")]
pub async fn query_unit_snapshot_stats(
    app_handle: tauri::AppHandle,
    unit_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_unit_snapshot_stats(
            app_config.replay_path,
            unit_name,
        ))
    })
    .await
}
