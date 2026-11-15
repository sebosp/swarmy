#![doc = include_str!("games/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The games played with the protocol version: `ext_fs_id`, one per game, by id.
pub fn plan(source: &dyn SnapshotSource, protocol_version: u32) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .table(DETAILS_IPC)?
        .filter(col("ext_protocol_version").eq(lit(protocol_version)))
        .select([col("ext_fs_id")])
        .unique(None, UniqueKeepStrategy::Any)
        .sort(["ext_fs_id"], Default::default()))
}

pub fn shape(frame: DataFrame) -> Result<Vec<u64>, SwarmyError> {
    u64_values(&frame, "ext_fs_id")
}

#[cfg(test)]
mod tests;
