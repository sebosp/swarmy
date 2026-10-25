pub mod data;
use crate::replay_rerun::viewer::{ensure_viewer_running, stream_replay_to_viewer};
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::{try_list_protocol_versions, try_list_years, try_query_replay_list};
use swarmy_common::*;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

#[tauri::command(rename_all = "snake_case")]
pub async fn query_replay_list(
    app_handle: tauri::AppHandle,
    query: ReplayListQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        let res = ApiResponseBuilder::new();
        res.process_result(try_query_replay_list(app_config.replay_path, query))
    })
    .await
}

/// The patches and years of the snapshot, for the dropdowns of the replay list.
#[tauri::command(rename_all = "snake_case")]
pub async fn list_replay_filters(app_handle: tauri::AppHandle) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        let filters = try_list_protocol_versions(app_config.replay_path.clone()).and_then(
            |protocol_versions| {
                Ok(ReplayListFilters {
                    protocol_versions,
                    years: try_list_years(app_config.replay_path)?,
                })
            },
        );
        ApiResponseBuilder::new().process_result(filters)
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn copy_path_to_clipboard(
    app_handle: tauri::AppHandle,
    data: String,
) -> Result<ApiResponse, SwarmyError> {
    let res = ApiResponseBuilder::new();
    tracing::info!("Writing {data} to clipboard.",);
    app_handle.clipboard().write_text(data).unwrap();
    Ok(res
        .with_message("Succesfully wrote to clipboard.".to_string())
        .with_success()
        .build())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_folder(
    app_handle: tauri::AppHandle,
    folder: String,
) -> Result<ApiResponse, SwarmyError> {
    tracing::info!("Requesting open on folder: {}", folder);
    let res = match app_handle.opener().open_path(folder, None::<&str>) {
        Ok(_) => ApiResponseBuilder::new()
            .with_message("Succesfully called open.".to_string())
            .with_success()
            .build(),
        Err(err) => {
            tracing::error!("Unable to open folder: {:?}", err);
            ApiResponseBuilder::new()
                .with_message(format!("Error requesting open: {:?}", err))
                .with_failure()
                .build()
        }
    };
    Ok(res)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn connect_swarmy_rerun(
    app_handle: tauri::AppHandle,
    replay_file_name: String,
) -> Result<ApiResponse, SwarmyError> {
    tracing::info!(
        "Requesting open rerun on replay_file_name: {}",
        replay_file_name
    );
    let app_config = load_stored_app_settings(&app_handle)?;
    ensure_viewer_running(&app_handle).await?;
    let stream_file_name = replay_file_name.clone();
    let streamed = tauri::async_runtime::spawn_blocking(move || {
        // SwarmyError is not Send, so it crosses the thread boundary as a String.
        stream_replay_to_viewer(&stream_file_name, &app_config.cache_path).map_err(String::from)
    })
    .await
    .map_err(|e| SwarmyError::Other(format!("Rerun streaming task failed: {e}")))?;
    let res = match streamed {
        Ok(total_events) => ApiResponseBuilder::new()
            .with_message(format!(
                "Succesfully sent {total_events} events to rerun for {replay_file_name}."
            ))
            .with_success()
            .build(),
        Err(err) => {
            tracing::error!("Unable to call rerun: {:?}", err);
            ApiResponseBuilder::new()
                .with_message(format!("Error calling rerun: {:?}", err))
                .with_failure()
                .build()
        }
    };
    Ok(res)
}
