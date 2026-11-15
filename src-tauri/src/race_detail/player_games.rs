#![doc = include_str!("player_games/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The games played by players of the race, in one row: `player_games` (u32).
pub fn plan(source: &dyn SnapshotSource, race: &str) -> Result<LazyFrame, SwarmyError> {
    Ok(snapshot_players_of(source)?
        .filter(col("race").eq(lit(race)))
        .select([len().alias("player_games")]))
}

pub fn shape(frame: DataFrame) -> Result<usize, SwarmyError> {
    Ok(frame
        .column("player_games")?
        .u32()?
        .get(0)
        .unwrap_or_default() as usize)
}

#[cfg(test)]
mod tests;
