//! Dataframe for map statistics.

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub fn try_query_map_stats(
    replay_path: String,
    query: MapStatsQuery,
) -> Result<Vec<MapStats>, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;

    tracing::info!(
        "Querying map stats from replay path: {} for map_title: {} and player_name: {}",
        ipc_path,
        query.map_title,
        query.player_name
    );

    let mut details_query = LazyFrame::scan_ipc(
        PlRefPath::new(&format!("{}/{}", ipc_path, DETAILS_IPC)),
        Default::default(),
        Default::default(),
    )?;

    if !query.map_title.is_empty() {
        details_query = details_query.filter(
            col("title")
                .str()
                .to_lowercase()
                .str()
                .contains(lit(query.map_title.to_lowercase()), false),
        );
    }
    if !query.player_name.is_empty() {
        details_query = details_query.filter(
            col("player_name")
                .str()
                .to_lowercase()
                .str()
                .contains(lit(query.player_name.to_lowercase()), false),
        );
    }
    details_query = details_query.unique(
        Some(Selector::Matches("ext_fs_id".into())),
        UniqueKeepStrategy::Any,
    );

    let res = details_query
        .group_by([col("title"), col("cache_handles")])
        .agg([
            col("ext_datetime")
                .min()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("min_date"),
            col("ext_datetime")
                .max()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("max_date"),
            len().alias("num_games"),
        ])
        .sort(
            ["num_games"],
            SortMultipleOptions::default()
                .with_order_descending(true)
                .with_nulls_last(true),
        )
        .limit(1000)
        .collect()?;
    println!("{res}");
    let res: Vec<MapStats> = (0..res.height())
        .map(|idx| extract_map_stats_from_df_row(&res.slice(idx as i64, 1)))
        .collect::<Result<_, _>>()?;
    Ok(res)
}

fn extract_map_stats_from_df_row(row: &DataFrame) -> Result<MapStats, SwarmyError> {
    let min_date = col_ymd_to_naive_date(row, "min_date")?;
    let max_date = col_ymd_to_naive_date(row, "max_date")?;
    let title = row
        .column("title")?
        .str()?
        .get(0)
        .unwrap_or("Empty Title")
        .to_string();
    let cache_handles = row
        .column("cache_handles")?
        .str()?
        .get(0)
        .unwrap_or("")
        .to_string();
    let num_games = row.column("num_games")?.u32()?.get(0).unwrap_or(0);
    Ok(MapStats {
        max_date,
        min_date,
        num_games,
        title,
        cache_handles,
    })
}
