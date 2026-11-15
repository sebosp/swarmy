#![doc = include_str!("race_games/README.md")]

use crate::data::*;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The games of the players of the race in each period of the grain: `period`, `games`,
/// `versus_games` and `versus_wins` (u32 sums), by period.
pub fn plan(
    source: &dyn SnapshotSource,
    grain: TimeGrain,
    race: &str,
) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .usage(grain, PLAYER_USAGE_IPC)?
        .filter(race_name_expr("player_race").eq(lit(race)))
        .group_by([col("period")])
        .agg([
            col("games").sum(),
            col("versus_games").sum(),
            col("versus_wins").sum(),
        ])
        .sort(["period"], Default::default()))
}

/// The games of the players of a race in a period, and its 1v1 games against other races.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RaceGames {
    pub games: u64,
    pub versus_games: u64,
    pub versus_wins: u64,
}

pub fn shape(frame: DataFrame) -> Result<BTreeMap<String, RaceGames>, SwarmyError> {
    let games = u32_values(&frame, "games")?;
    let versus_games = u32_values(&frame, "versus_games")?;
    let versus_wins = u32_values(&frame, "versus_wins")?;
    Ok(str_values(&frame, "period")?
        .into_iter()
        .enumerate()
        .map(|(idx, period)| {
            (
                period,
                RaceGames {
                    games: games[idx] as u64,
                    versus_games: versus_games[idx] as u64,
                    versus_wins: versus_wins[idx] as u64,
                },
            )
        })
        .collect())
}

#[cfg(test)]
mod tests;
