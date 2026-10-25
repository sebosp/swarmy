use crate::SetupState;
pub mod data;
use crate::majordomo::AsyncTask;
use swarmy_common::*;
use tauri::State;

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
    let map_stats = res_rx.await?;
    Ok(res
        .with_message(map_stats.message)
        .with_status(map_stats.meta.success)
        .build())
}
