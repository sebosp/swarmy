#![doc = include_str!("matchups/README.md")]

use super::one_vs_one;
use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The 1v1 games of the race by matchup: `matchup` (`ZvT`, the race first), `games` (u32) and
/// `wins` (Int32), by matchup.
pub fn plan(source: &dyn SnapshotSource, race: &str) -> Result<LazyFrame, SwarmyError> {
    Ok(one_vs_one(source, race)?
        .group_by([super::matchup_expr(race)])
        .agg(super::games_wins())
        .sort(["matchup"], Default::default()))
}

pub fn shape(frame: DataFrame) -> Result<Vec<GamesWins>, SwarmyError> {
    super::games_wins_rows(&frame, "matchup")
}

#[cfg(test)]
mod tests;
