#![doc = include_str!("player_games/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The matchup of the games of more than two players.
pub const TEAM_GAMES: &str = "Team games";

/// Every game of a player of the race of the ability, with its uses: `ext_fs_id`, `player_name`,
/// `title`, `patch`, `date` (`YYYY-MM-DD`), `matchup` (`ZvT`, or `Team games`), `uses` (u32, null
/// when not used) and `first_seconds` (u32, null when not used); by game and player.
pub fn plan(
    source: &dyn SnapshotSource,
    tracked: &TrackedAbility,
) -> Result<LazyFrame, SwarmyError> {
    // The rows of a game numbered, so that the other row of a 1v1 is found even when two players
    // have the same name (two A.I. of the same level).
    let sided = snapshot_players_of(source)?.with_column(
        int_range(lit(0), len(), 1, DataType::UInt32)
            .over([col("ext_fs_id")])?
            .alias("side"),
    );
    let opponents = sided
        .clone()
        .filter(len().over([col("ext_fs_id")])?.eq(lit(2u32)))
        .select([
            col("ext_fs_id"),
            col("side").alias("other_side"),
            col("race").alias("opponent_race"),
        ]);
    let mine = sided
        .filter(col("race").eq(lit(tracked.race)))
        .join(
            opponents,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .filter(
            col("other_side")
                .is_null()
                .or(col("other_side").neq(col("side"))),
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("title"),
            col("patch"),
            when(col("opponent_race").is_null())
                .then(lit(TEAM_GAMES))
                .otherwise(concat_str(
                    [
                        lit(&tracked.race[..1]),
                        lit("v"),
                        col("opponent_race").str().slice(lit(0), lit(1)),
                    ],
                    "",
                    false,
                ))
                .alias("matchup"),
        ]);
    let uses = source
        .table(GAME_ABILITIES_IPC)?
        .filter(col("ability").eq(lit(tracked.ability)))
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("uses"),
            col("first_seconds"),
        ]);
    let dates = source
        .table(DETAILS_IPC)?
        .group_by([col("ext_fs_id")])
        .agg([col("ext_datetime")
            .min()
            .dt()
            .to_string("%Y-%m-%d")
            .alias("date")]);
    let keys = [col("ext_fs_id"), col("player_name")];
    Ok(mine
        .join(uses, keys.clone(), keys, JoinArgs::new(JoinType::Left))
        .join(
            dates,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("title"),
            col("patch"),
            col("date"),
            col("matchup"),
            col("uses"),
            col("first_seconds"),
        ])
        .sort_by_exprs([col("ext_fs_id"), col("player_name")], Default::default()))
}

/// A player game of the race of the ability, with its uses.
#[derive(Debug, Clone)]
pub struct PlayerGame {
    pub game: AbilityGameUse,
    pub patch: u32,
}

pub fn shape(frame: DataFrame) -> Result<Vec<PlayerGame>, SwarmyError> {
    let ids = u64_values(&frame, "ext_fs_id")?;
    let names = str_values(&frame, "player_name")?;
    let titles = str_values(&frame, "title")?;
    let patches = u32_values(&frame, "patch")?;
    let dates = str_values(&frame, "date")?;
    let matchups = str_values(&frame, "matchup")?;
    let uses: Vec<Option<u32>> = frame.column("uses")?.u32()?.iter().collect();
    let first: Vec<Option<u32>> = frame.column("first_seconds")?.u32()?.iter().collect();
    Ok((0..frame.height())
        .map(|idx| PlayerGame {
            game: AbilityGameUse {
                ext_fs_id: ids[idx],
                player_name: names[idx].clone(),
                map_title: titles[idx].clone(),
                date: dates[idx].clone(),
                matchup: matchups[idx].clone(),
                uses: uses[idx].unwrap_or_default() as usize,
                first_seconds: first[idx],
            },
            patch: patches[idx],
        })
        .collect())
}

#[cfg(test)]
mod tests;
