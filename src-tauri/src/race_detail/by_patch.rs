#![doc = include_str!("by_patch/README.md")]

use super::one_vs_one;
use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The non-mirror 1v1 games of the race by patch: `patch` (u32), `games` (u32) and `wins`
/// (Int32), the oldest patch first.
pub fn plan(source: &dyn SnapshotSource, race: &str) -> Result<LazyFrame, SwarmyError> {
    Ok(one_vs_one(source, race)?
        .filter(super::not_mirror(race))
        .group_by([col("patch")])
        .agg(super::games_wins())
        .sort(["patch"], Default::default()))
}

pub fn shape(frame: DataFrame) -> Result<Vec<GamesWins>, SwarmyError> {
    super::games_wins_rows(&frame, "patch")
}

#[cfg(test)]
mod tests;
