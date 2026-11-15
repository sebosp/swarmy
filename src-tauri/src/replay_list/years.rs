#![doc = include_str!("years/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The years with games in the snapshot: `year` (Int32), the newest first.
pub fn plan(source: &dyn SnapshotSource) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .table(DETAILS_IPC)?
        .select([col("ext_datetime").dt().year().alias("year")])
        .unique(None, UniqueKeepStrategy::Any)
        .sort(
            ["year"],
            SortMultipleOptions::default().with_order_descending(true),
        ))
}

pub fn shape(frame: DataFrame) -> Result<Vec<i32>, SwarmyError> {
    Ok(frame.column("year")?.i32()?.iter().flatten().collect())
}

#[cfg(test)]
mod tests;
