#![doc = include_str!("summary/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// One row: `min_date` and `max_date` (`YYYY-MM-DD`) of the games, `num_games` and `num_maps`
/// (u32).
pub fn plan(source: &dyn SnapshotSource) -> Result<LazyFrame, SwarmyError> {
    Ok(source.table(DETAILS_IPC)?.select([
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
        // The ids of the replays added later follow the existing ones, there can be gaps.
        col("ext_fs_id").n_unique().alias("num_games"),
        col("title").n_unique().alias("num_maps"),
    ]))
}

/// The dates and counts of the snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub min_date: chrono::NaiveDate,
    pub max_date: chrono::NaiveDate,
    pub num_games: u64,
    pub num_maps: u32,
}

pub fn shape(frame: DataFrame) -> Result<Summary, SwarmyError> {
    Ok(Summary {
        min_date: col_ymd_to_naive_date(&frame, "min_date")?,
        max_date: col_ymd_to_naive_date(&frame, "max_date")?,
        num_games: u32_values(&frame, "num_games")?
            .first()
            .copied()
            .unwrap_or_default() as u64,
        num_maps: u32_values(&frame, "num_maps")?
            .first()
            .copied()
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests;
