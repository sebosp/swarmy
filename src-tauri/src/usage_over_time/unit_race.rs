#![doc = include_str!("unit_race/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The race that makes a unit: the one whose players made the most of it, from the yearly
/// usage; one row, `race` and `born` (u64).
pub fn plan(source: &dyn SnapshotSource, unit: &str) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .usage(TimeGrain::Year, UNIT_USAGE_IPC)?
        .filter(col("unit").eq(lit(unit)))
        .group_by([race_name_expr("player_race").alias("race")])
        .agg([col("born").sum()])
        .sort_by_exprs(
            [col("born"), col("race")],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        )
        .limit(1))
}

/// The race, empty when the unit was never made.
pub fn shape(frame: DataFrame) -> Result<String, SwarmyError> {
    Ok(str_values(&frame, "race")?
        .into_iter()
        .next()
        .unwrap_or_default())
}

#[cfg(test)]
mod tests;
