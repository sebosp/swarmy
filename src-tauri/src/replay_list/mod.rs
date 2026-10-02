pub mod data;
use crate::get_current_app_config;
use data::try_query_replay_list;
use swarmy_common::*;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_shell::ShellExt;

#[tauri::command(rename_all = "snake_case")]
pub async fn query_replay_list(
    app_handle: tauri::AppHandle,
    map_title: String,
    player_name: String,
    min_date: chrono::NaiveDate,
    max_date: chrono::NaiveDate,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = get_current_app_config(app_handle.clone()).await?;
    let t = std::thread::spawn(move || {
        let res = ApiResponseBuilder::new();
        let query = ReplayListQuery {
            map_title,
            player_name,
            min_date,
            max_date,
        };
        res.process_result(try_query_replay_list(app_config.replay_path, query))
    });
    Ok(t.join().unwrap())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_rerun_replay(
    app_handle: tauri::AppHandle,
    map_title: String,
    cache_ids: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = get_current_app_config(app_handle.clone()).await?;
    let res = ApiResponseBuilder::new();
    tracing::info!(
        "Trying /home/seb/git/swarmy-bevy/target/release/swarmy-bevy {} {} {}",
        &map_title,
        &app_config.cache_path,
        &cache_ids
    );
    let t = std::thread::spawn(async move || {
        let shell = app_handle.shell();
        shell
            .command("/home/seb/git/swarmy-bevy/target/debug/swarmy-bevy")
            .args([
                "--map-title",
                &map_title,
                "--snapshot-path",
                &app_config.cache_path,
                "--cache-handle-ids",
                &cache_ids,
            ])
            .output()
            .await
            .unwrap()
    });
    let output = t.join().unwrap().await;
    let message = match output.status.success() {
        true => "Succesfully called swarmy-bevy".to_string(),
        false => match String::from_utf8(output.stderr.clone()) {
            Ok(utf8_str) => format!("Error executing swarmy bevy: {}", utf8_str),
            Err(_) => format!(
                "Error executing swarmy bevy (also non-utf8): {:?}",
                output.stderr
            ),
        },
    };
    Ok(res
        .with_status(output.status.success())
        .with_message(message)
        .build())
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
    let res = match app_handle
        .opener()
        .open_path(&replay_file_name, None::<&str>)
    {
        Ok(_) => ApiResponseBuilder::new()
            .with_message(format!("Succesfully called rerun for {replay_file_name}."))
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
