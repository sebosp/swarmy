#![doc = include_str!("totals/README.md")]

use super::game_uses;
use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The use of each tracked ability in the games of each patch (and of all, patch 0): `patch`,
/// `race`, `ability`, `games_used` (u32, the player games it was used in), `uses` (u32) and
/// `player_games` (u32, the games of the race in the patch); by patch, race and ability.
pub fn plan(source: &dyn SnapshotSource, tracked: DataFrame) -> Result<LazyFrame, SwarmyError> {
    let race_games = game_uses::players_of_both(source)?
        .group_by([col("patch"), col("race")])
        .agg([len().alias("player_games")]);
    Ok(game_uses::plan(source, tracked)?
        .group_by([col("patch"), col("race"), col("ability")])
        .agg([len().alias("games_used"), col("uses").sum()])
        .join(
            race_games,
            [col("patch"), col("race")],
            [col("patch"), col("race")],
            JoinArgs::new(JoinType::Inner),
        )
        .select([
            col("patch"),
            col("race"),
            col("ability"),
            col("games_used"),
            col("uses"),
            col("player_games"),
        ])
        .sort_by_exprs(
            [col("patch"), col("race"), col("ability")],
            Default::default(),
        ))
}

#[cfg(test)]
mod tests;
