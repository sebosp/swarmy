#![doc = include_str!("opponents/README.md")]

use super::matches;
use crate::data::*;
use polars::prelude::*;
use std::collections::HashMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The opponents of the player: `opponent`, `games` (u32) and `wins` (Int32) of the player
/// against them, the most games first.
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    player: &str,
) -> Result<LazyFrame, SwarmyError> {
    Ok(matches::plan(source, groups, player)?
        .filter(col("opponent").is_not_null())
        .group_by([col("opponent")])
        .agg([
            len().alias("games"),
            // Int32, the type of the SQL `SUM(CASE WHEN ... THEN 1 ELSE 0 END)` of the README.
            col("result")
                .eq(lit("Win"))
                .sum()
                .cast(DataType::Int32)
                .alias("wins"),
        ])
        .sort_by_exprs(
            [col("games"), col("opponent")],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        ))
}

pub fn shape(frame: DataFrame) -> Result<Vec<OpponentRecord>, SwarmyError> {
    Ok(str_values(&frame, "opponent")?
        .into_iter()
        .zip(u32_values(&frame, "games")?)
        .zip(i32_values(&frame, "wins")?)
        .map(|((opponent, games), wins)| OpponentRecord {
            opponent,
            games: games as usize,
            wins: wins as usize,
        })
        .collect())
}

#[cfg(test)]
mod tests;
