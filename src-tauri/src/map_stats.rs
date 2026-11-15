//! Map List: the maps of the snapshot with their games and dates, filtered by map or player. The
//! statistic is a query file of this directory (AGENTS.md, "A query is a file"); the command runs
//! it through the majordomo, the coordinator thread of the long queries.

pub mod maps;

use crate::data::{IpcSnapshot, QueryMeta};
use crate::host::SwarmyHost;
use crate::majordomo::AsyncTask;
use swarmy_common::*;

/// The queries of the view, for the debug links of the panels.
pub const QUERIES: [QueryMeta; 1] = [maps::META];

/// The maps that match the query, the most played first.
pub fn try_query_map_stats(
    replay_path: &str,
    query: &MapStatsQuery,
) -> Result<Vec<MapStats>, SwarmyError> {
    let source = IpcSnapshot::open(replay_path)?;
    maps::shape(maps::plan(&source, query)?.collect()?)
}

pub async fn query_map_stats(
    host: &dyn SwarmyHost,
    map_title: String,
    player_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = host.majordomo();
    let res = ApiResponseBuilder::new().with_sources(QUERIES.map(QuerySource::from));
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
