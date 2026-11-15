#![doc = include_str!("players/README.md")]

use super::player_rows;
use crate::data::*;
use polars::prelude::*;
use std::collections::HashMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The players of the snapshot: `player`, `games` (u32) and `wins` (Int32), the most games first.
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
) -> Result<LazyFrame, SwarmyError> {
    Ok(player_rows::plan(source, groups)?
        .filter(col("player").neq(lit("")))
        .group_by([col("player")])
        .agg([
            col("ext_fs_id").n_unique().alias("games"),
            // Int32, the type of the SQL `SUM(CASE WHEN ... THEN 1 ELSE 0 END)` of the README.
            col("player_result")
                .eq(lit("Win"))
                .sum()
                .cast(DataType::Int32)
                .alias("wins"),
        ])
        .sort_by_exprs(
            [col("games"), col("player")],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        ))
}

/// The players as [`GamesWins`] for the player fields.
pub fn shape(frame: DataFrame) -> Result<Vec<GamesWins>, SwarmyError> {
    Ok(str_values(&frame, "player")?
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
