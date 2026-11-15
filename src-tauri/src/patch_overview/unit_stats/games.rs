#![doc = include_str!("games/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// One row per game of the snapshot: `ext_fs_id`, `month` (`YYYY-MM`), `date` (`YYYY-MM-DD`),
/// `title`, `ext_fs_file_name` and `ext_protocol_version`; the base of the unit statistics by
/// month and of the exceptional games.
pub fn plan(source: &dyn SnapshotSource) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .table(DETAILS_IPC)?
        .group_by([col("ext_fs_id")])
        .agg([
            col("ext_datetime")
                .min()
                .dt()
                .to_string("%Y-%m")
                .alias("month"),
            col("ext_datetime")
                .min()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("date"),
            col("title").first(),
            col("ext_fs_file_name").first(),
            col("ext_protocol_version").first(),
        ])
        .sort(["ext_fs_id"], Default::default()))
}

#[cfg(test)]
mod tests;
