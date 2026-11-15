#![doc = include_str!("kills_per_unit/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The kills per unit of each player that made at least `min_born` of the unit in a game:
/// `ext_fs_id`, `player_name`, `born`, `kills` (u32), `value_killed` (u64) and `kills_per_unit`
/// (f64), by game and player.
pub fn plan(
    source: &dyn SnapshotSource,
    unit: &str,
    min_born: usize,
) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .table(GAME_UNITS_IPC)?
        .filter(
            col("unit")
                .eq(lit(unit))
                .and(col("born").gt_eq(lit(min_born as u32))),
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("born"),
            col("kills"),
            col("value_killed"),
            (col("kills").cast(DataType::Float64) / col("born").cast(DataType::Float64))
                .alias("kills_per_unit"),
        ])
        .sort_by_exprs([col("ext_fs_id"), col("player_name")], Default::default()))
}

/// The mean of the kills per unit and the threshold of an exceptional game: two standard
/// deviations above it. Both 0 when nobody made enough of the unit.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Threshold {
    pub mean: f64,
    pub threshold: f64,
}

/// The mean and the threshold from the frame of [`plan`].
pub fn threshold(frame: &DataFrame) -> Result<Threshold, SwarmyError> {
    let stats = frame
        .clone()
        .lazy()
        .select([
            col("kills_per_unit")
                .mean()
                .fill_null(lit(0.))
                .alias("mean"),
            col("kills_per_unit").std(1).fill_null(lit(0.)).alias("std"),
        ])
        .collect()?;
    let first = |name: &str| -> Result<f64, SwarmyError> {
        Ok(stats.column(name)?.f64()?.get(0).unwrap_or_default())
    };
    let mean = first("mean")?;
    Ok(Threshold {
        mean,
        threshold: mean + 2. * first("std")?,
    })
}

#[cfg(test)]
mod tests;
