#![doc = include_str!("top_players/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The players shown, by number of games.
pub const TOP_PLAYERS: u32 = 10;

/// The players with the most games on a map: `label` (the name as stored in the replay),
/// `games` and `wins` (Int32), the most games first.
pub fn plan(source: &dyn SnapshotSource, map_title: &str) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .table(DETAILS_IPC)?
        .filter(
            col("title")
                .eq(lit(map_title))
                .and(col("player_observe").eq(lit(0u8))),
        )
        .group_by([col("player_name").alias("label")])
        .agg([
            len().alias("games"),
            // Int32, the type of the SQL `SUM(CASE WHEN ... THEN 1 ELSE 0 END)` of the README.
            col("player_result")
                .eq(lit("Win"))
                .sum()
                .cast(DataType::Int32)
                .alias("wins"),
        ])
        .sort_by_exprs(
            [col("games"), col("wins"), col("label")],
            SortMultipleOptions::default().with_order_descending_multi([true, true, false]),
        )
        .limit(TOP_PLAYERS))
}

pub fn shape(frame: DataFrame) -> Result<Vec<GamesWins>, SwarmyError> {
    Ok(str_values(&frame, "label")?
        .into_iter()
        .zip(u32_values(&frame, "games")?)
        .zip(i32_values(&frame, "wins")?)
        .map(|((label, games), wins)| GamesWins {
            label,
            games: games as usize,
            wins: wins as usize,
        })
        .collect())
}

#[cfg(test)]
mod tests;
