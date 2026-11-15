#![doc = include_str!("by_month/README.md")]

use super::games;
use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// Per month of the snapshot: the `games` (u32), the `games_with_unit` (u32) and the units
/// `born` (u32), the oldest month first.
pub fn plan(source: &dyn SnapshotSource, unit: &str) -> Result<LazyFrame, SwarmyError> {
    let made = source
        .table(GAME_UNITS_IPC)?
        .filter(col("unit").eq(lit(unit)).and(col("born").gt(lit(0u32))))
        .group_by([col("ext_fs_id")])
        .agg([col("born").sum()]);
    Ok(games::plan(source)?
        .join(
            made,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .group_by([col("month")])
        .agg([
            len().alias("games"),
            col("born").count().alias("games_with_unit"),
            col("born").sum(),
        ])
        .sort(["month"], Default::default()))
}

pub fn shape(frame: DataFrame) -> Result<Vec<UnitMonthUsage>, SwarmyError> {
    let months = str_values(&frame, "month")?;
    let games = u32_values(&frame, "games")?;
    let with_unit = u32_values(&frame, "games_with_unit")?;
    let born = u32_values(&frame, "born")?;
    Ok((0..frame.height())
        .map(|idx| UnitMonthUsage {
            month: months[idx].clone(),
            games: games[idx] as usize,
            games_with_unit: with_unit[idx] as usize,
            born: born[idx] as usize,
        })
        .collect())
}

#[cfg(test)]
mod tests;
