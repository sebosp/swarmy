#![doc = include_str!("matchups/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The 1v1 games of a map by matchup: `label` (`PvT`, the races sorted), `games` and `wins` of
/// the first race (Int32, 0 in a mirror).
pub fn plan(source: &dyn SnapshotSource, map_title: &str) -> Result<LazyFrame, SwarmyError> {
    let players = source
        .table(DETAILS_IPC)?
        .filter(
            col("title")
                .eq(lit(map_title))
                .and(col("player_observe").eq(lit(0u8))),
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            race_letter_expr("player_race").alias("race"),
            col("player_result").eq(lit("Win")).alias("won"),
        ])
        // An arcade race is not a matchup.
        .filter(col("race").neq(lit("?")));
    // The two sides of a game in the order of their race letters (P, T, Z), the name second so
    // that a mirror has a first side too.
    let games = players
        .sort_by_exprs(
            [col("ext_fs_id"), col("race"), col("player_name")],
            SortMultipleOptions::default(),
        )
        .group_by_stable([col("ext_fs_id")])
        .agg([
            col("race").first().alias("race"),
            col("race").last().alias("other_race"),
            col("won").first().alias("first_won"),
            len().alias("players"),
        ])
        // Only the 1v1 games have a matchup.
        .filter(col("players").eq(lit(2u32)));
    Ok(games
        .with_column(
            concat_str([col("race"), lit("v"), col("other_race")], "", false).alias("label"),
        )
        .group_by([col("label")])
        .agg([
            len().alias("games"),
            // The wins of the first race; a mirror always has a winner, its wins stay 0.
            // Int32, the type of the SQL `SUM(CASE WHEN ... THEN 1 ELSE 0 END)` of the README.
            (col("first_won").and(col("race").neq(col("other_race"))))
                .sum()
                .cast(DataType::Int32)
                .alias("wins"),
        ])
        .sort(["label"], Default::default()))
}

/// The matchups as [`GamesWins`], the label is the matchup.
pub fn shape(frame: DataFrame) -> Result<Vec<GamesWins>, SwarmyError> {
    Ok(str_values(&frame, "label")?
        .into_iter()
        .zip(u32_values(&frame, "games")?)
        .zip(i32_values(&frame, "wins")?)
        .map(|((label, games), wins)| GamesWins {
            label,
            games: games as usize,
            wins: wins as usize,
        })
        .collect())
}

#[cfg(test)]
mod tests;
