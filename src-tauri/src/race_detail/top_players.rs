#![doc = include_str!("top_players/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The players with the most games of the race shown.
pub const TOP_PLAYERS: u32 = 10;

/// The players with the most games as the race: `player_name`, `games` (u32) and `wins` (Int32),
/// the most games first.
pub fn plan(source: &dyn SnapshotSource, race: &str) -> Result<LazyFrame, SwarmyError> {
    Ok(snapshot_players_of(source)?
        .filter(col("race").eq(lit(race)))
        .group_by([col("player_name")])
        .agg(super::games_wins())
        .sort_by_exprs(
            [col("games"), col("player_name")],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        )
        .limit(TOP_PLAYERS))
}

pub fn shape(frame: DataFrame) -> Result<Vec<GamesWins>, SwarmyError> {
    super::games_wins_rows(&frame, "player_name")
}

#[cfg(test)]
mod tests;
