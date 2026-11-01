use crate::host::SwarmyHost;
pub mod data;
use crate::majordomo::AsyncTask;
use swarmy_common::*;

pub async fn query_map_stats(
    host: &dyn SwarmyHost,
    map_title: String,
    player_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = host.majordomo();
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
