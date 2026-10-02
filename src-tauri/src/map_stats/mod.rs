use crate::SetupState;
pub mod data;
use crate::get_current_app_config;
use crate::majordomo::AsyncTask;
use swarmy_common::*;
use tauri::State;
use tauri_plugin_shell::ShellExt;

#[tauri::command(rename_all = "snake_case")]
pub async fn query_map_stats(
    state: State<'_, SetupState>,
    map_title: String,
    player_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = state.majordomo_tx.clone();
    let res = ApiResponseBuilder::new();
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();

    mdp_tx
        .send(AsyncTask::QueryMapStats {
            map_title,
            player_name,
            res_tx,
        })
        .await?;
    Ok(res.process_result(Ok(res_rx.await?)))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn exec_swarmy_bevy_map_caches(
    app_handle: tauri::AppHandle,
    map_title: String,
    cache_ids: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = get_current_app_config(app_handle.clone()).await?;
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
                "--path",
                &app_config.cache_path,
                "--ids",
                &cache_ids,
            ])
            .output()
            .await
            .unwrap()
    });
    let output = t.join().unwrap().await;
    Ok(ApiResponseBuilder::new()
        .with_status(output.status.success())
        .with_message(match output.status.success() {
            true => "Succesfully called swarmy-bevy".to_string(),
            false => match String::from_utf8(output.stderr.clone()) {
                Ok(utf8_str) => format!("Error executing swarmy bevy: {}", utf8_str),
                Err(_) => format!(
                    "Error executing swarmy bevy (also non-utf8): {:?}",
                    output.stderr
                ),
            },
        })
        .build())
}
