#![doc = include_str!("games_summary/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The games played on a map, in one row: `games`, `min_date`, `max_date` (`YYYY-MM-DD`),
/// `avg_duration_seconds` (Float64) and the `cache_handles` of the newest game.
pub fn plan(source: &dyn SnapshotSource, map_title: &str) -> Result<LazyFrame, SwarmyError> {
    let games = source
        .table(DETAILS_IPC)?
        .filter(col("title").eq(lit(map_title)))
        .group_by([col("ext_fs_id")])
        .agg([
            col("ext_datetime").min().alias("played_at"),
            col("cache_handles").min(),
        ]);
    // The newest game first, numbered from 1: its cache handles are the map's.
    let ranked = games
        .sort_by_exprs(
            [col("played_at"), col("ext_fs_id")],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .with_column(int_range(lit(1), len() + lit(1), 1, DataType::UInt32).alias("newest"));
    let durations = source
        .table(GAME_PLAYERS_IPC)?
        .group_by([col("ext_fs_id")])
        .agg([col("game_seconds").min()]);
    Ok(ranked
        .join(
            durations,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .select([
            len().alias("games"),
            col("played_at")
                .min()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("min_date"),
            col("played_at")
                .max()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("max_date"),
            col("game_seconds").mean().alias("avg_duration_seconds"),
            when(col("newest").eq(lit(1u32)))
                .then(col("cache_handles"))
                .otherwise(lit(NULL))
                .max()
                .alias("cache_handles"),
        ]))
}

/// The summary of the map, as the view shows it in its first line.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GamesSummary {
    pub games: usize,
    pub min_date: String,
    pub max_date: String,
    pub avg_duration_seconds: u32,
    /// The comma separated cache handles of the newest game, the map cache to load.
    pub cache_handles: String,
}

/// An error when the map has no games.
pub fn shape(frame: DataFrame, map_title: &str) -> Result<GamesSummary, SwarmyError> {
    let games = frame.column("games")?.u32()?.get(0).unwrap_or_default() as usize;
    if games == 0 {
        return Err(SwarmyError::Other(format!(
            "No games found for map {map_title}"
        )));
    }
    Ok(GamesSummary {
        games,
        min_date: str_values(&frame, "min_date")?.remove(0),
        max_date: str_values(&frame, "max_date")?.remove(0),
        avg_duration_seconds: frame
            .column("avg_duration_seconds")?
            .f64()?
            .get(0)
            .unwrap_or_default() as u32,
        cache_handles: str_values(&frame, "cache_handles")?.remove(0),
    })
}

#[cfg(test)]
mod tests;
