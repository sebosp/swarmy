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
) -> ApiResponse {
    let app_config = match get_current_app_config(app_handle.clone()).await {
        Ok(config) => config,
        Err(e) => {
            return ApiResponse::new(
                ResponseMetaBuilder::new(false).duration_ms(0).build(),
                format!("Error getting current app config: {:?}", e),
            );
        }
    };
    let t = std::thread::spawn(move || {
        let init_time = std::time::Instant::now();
        let query = ReplayListQuery {
            map_title,
            player_name,
            min_date,
            max_date,
        };
        match try_query_replay_list(app_config.replay_path, query) {
            Ok(val) => ApiResponse::new(
                ResponseMetaBuilder::new(true)
                    .duration_ms(init_time.elapsed().as_millis() as u64)
                    .build(),
                serde_json::to_string(&val).unwrap_or_default(),
            ),
            Err(e) => {
                log::error!("Error query maps: {}", e);
                ApiResponse::new(
                    ResponseMetaBuilder::new(false)
                        .duration_ms(init_time.elapsed().as_millis() as u64)
                        .build(),
                    format!("Error querying maps: {:?}", e),
                )
            }
        }
    });
    t.join().unwrap()
}

#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_rerun_replay(
    app_handle: tauri::AppHandle,
    map_title: String,
    cache_ids: String,
) -> ApiResponse {
    let app_config = match get_current_app_config(app_handle.clone()).await {
        Ok(config) => config,
        Err(e) => {
            return ApiResponse::new(
                ResponseMetaBuilder::new(false).duration_ms(0).build(),
                format!("Error getting current app config: {:?}", e),
            );
        }
    };
    let file_cache_path = format!("{}/{}/", app_config.replay_path, CACHES_DIR);
    log::info!(
        "Trying /home/seb/git/swarmy-bevy/target/release/swarmy-bevy {} {} {}",
        &map_title,
        &file_cache_path,
        &cache_ids
    );
    let init_time = std::time::Instant::now();
    let t = std::thread::spawn(async move || {
        let shell = app_handle.shell();
        shell
            .command("/home/seb/git/swarmy-bevy/target/debug/swarmy-bevy")
            .args([
                "--map-title",
                &map_title,
                "--snapshot-path",
                &file_cache_path,
                "--cache-handle-ids",
                &cache_ids,
            ])
            .output()
            .await
            .unwrap()
    });
    let output = t.join().unwrap().await;
    ApiResponse::new(
        ResponseMetaBuilder::new(output.status.success())
            .duration_ms(init_time.elapsed().as_millis() as u64)
            .build(),
        match output.status.success() {
            true => "Succesfully called swarmy-bevy".to_string(),
            false => match String::from_utf8(output.stderr.clone()) {
                Ok(utf8_str) => format!("Error executing swarmy bevy: {}", utf8_str),
                Err(_) => format!(
                    "Error executing swarmy bevy (also non-utf8): {:?}",
                    output.stderr
                ),
            },
        },
    )
}

#[tauri::command(rename_all = "snake_case")]
pub async fn copy_path_to_clipboard(app_handle: tauri::AppHandle, data: String) -> ApiResponse {
    log::info!("Writing {data} to clipboard.",);
    app_handle.clipboard().write_text(data).unwrap();
    ApiResponse::new(
        ResponseMetaBuilder::new(true).duration_ms(0 as u64).build(),
        "Succesfully wrote to clipboard.".to_string(),
    )
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_folder(app_handle: tauri::AppHandle, folder: String) -> ApiResponse {
    log::info!("Requesting open on folder: {}", folder);
    match app_handle.opener().open_path(folder, None::<&str>) {
        Ok(_) => ApiResponse::new(
            ResponseMetaBuilder::new(true).duration_ms(0 as u64).build(),
            "Succesfully called open.".to_string(),
        ),
        Err(err) => ApiResponse::new(
            ResponseMetaBuilder::new(false)
                .duration_ms(0 as u64)
                .build(),
            format!("Error requesting open: {:?}", err),
        ),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn connect_swarmy_rerun(
    app_handle: tauri::AppHandle,
    replay_file_name: String,
) -> ApiResponse {
    log::info!(
        "Requesting open rerun on replay_file_name: {}",
        replay_file_name
    );
    match app_handle
        .opener()
        .open_path(replay_file_name, None::<&str>)
    {
        Ok(_) => ApiResponse::new(
            ResponseMetaBuilder::new(true).duration_ms(0 as u64).build(),
            "Succesfully called open.".to_string(),
        ),
        Err(err) => ApiResponse::new(
            ResponseMetaBuilder::new(false)
                .duration_ms(0 as u64)
                .build(),
            format!("Error requesting open: {:?}", err),
        ),
    }
}
