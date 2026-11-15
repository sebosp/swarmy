#![doc = include_str!("by_player/README.md")]

use super::game_uses;
use crate::data::players::name_without_clan_expr;
use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The players ranked by uses per game of each tracked ability in each patch (and in all, patch
/// 0), the players with at least [`ABILITY_RANK_MIN_GAMES`] games of the race: `patch`, `race`,
/// `ability`, `player_name` (without the clan tag), `uses` (u32), `games` (u32, the player's
/// games of the race in the patch) and `per_game` (f64); by patch and ability, the most per game
/// first, then by name.
pub fn plan(source: &dyn SnapshotSource, tracked: DataFrame) -> Result<LazyFrame, SwarmyError> {
    // The names without the clan tag: a player keeps the rank when the tag changes.
    let player_games = game_uses::players_of_both(source)?
        .with_column(name_without_clan_expr().alias("player_name"))
        .group_by([col("patch"), col("race"), col("player_name")])
        .agg([len().alias("games")]);
    Ok(game_uses::plan(source, tracked)?
        .with_column(name_without_clan_expr().alias("player_name"))
        .group_by([
            col("patch"),
            col("race"),
            col("ability"),
            col("player_name"),
        ])
        .agg([col("uses").sum()])
        .join(
            player_games,
            [col("patch"), col("race"), col("player_name")],
            [col("patch"), col("race"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("games").gt_eq(lit(ABILITY_RANK_MIN_GAMES as u32)))
        .with_column(
            (col("uses").cast(DataType::Float64) / col("games").cast(DataType::Float64))
                .alias("per_game"),
        )
        .select([
            col("patch"),
            col("race"),
            col("ability"),
            col("player_name"),
            col("uses"),
            col("games"),
            col("per_game"),
        ])
        .sort_by_exprs(
            [
                col("patch"),
                col("ability"),
                col("per_game"),
                col("player_name"),
            ],
            SortMultipleOptions::default().with_order_descending_multi([false, false, true, false]),
        ))
}

#[cfg(test)]
mod tests;
