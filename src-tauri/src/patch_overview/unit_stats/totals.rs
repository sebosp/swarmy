#![doc = include_str!("totals/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// One row: the `games` of the snapshot (u32), the `games_with_unit` (u32, a player made it),
/// the units `born`, `died`, their `kills` (u32 sums) and `value_killed` (u64).
pub fn plan(source: &dyn SnapshotSource, unit: &str) -> Result<LazyFrame, SwarmyError> {
    let games = source
        .table(DETAILS_IPC)?
        .select([col("ext_fs_id").n_unique().alias("games")]);
    let rows = source
        .table(GAME_UNITS_IPC)?
        .filter(col("unit").eq(lit(unit)));
    // The units the player made: their kills. A unit received (mind controlled, given by an ally)
    // counts its death below, not its kills.
    let made = rows.clone().filter(col("born").gt(lit(0u32))).select([
        col("ext_fs_id").n_unique().alias("games_with_unit"),
        col("born").sum(),
        col("kills").sum(),
        col("value_killed").sum(),
    ]);
    let lost = rows.select([col("died").sum()]);
    Ok(games.cross_join(made, None).cross_join(lost, None).select([
        col("games"),
        col("games_with_unit"),
        col("born"),
        col("died"),
        col("kills"),
        col("value_killed"),
    ]))
}

/// The totals, zero when the unit was never made.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    pub games: usize,
    pub games_with_unit: usize,
    pub born: usize,
    pub died: usize,
    pub kills: usize,
    pub value_killed: u32,
}

pub fn shape(frame: DataFrame) -> Result<Totals, SwarmyError> {
    let first_u32 = |name: &str| -> Result<usize, SwarmyError> {
        Ok(u32_values(&frame, name)?
            .first()
            .copied()
            .unwrap_or_default() as usize)
    };
    Ok(Totals {
        games: first_u32("games")?,
        games_with_unit: first_u32("games_with_unit")?,
        born: first_u32("born")?,
        died: first_u32("died")?,
        kills: first_u32("kills")?,
        value_killed: u64_values(&frame, "value_killed")?
            .first()
            .copied()
            .unwrap_or_default() as u32,
    })
}

#[cfg(test)]
mod tests;
