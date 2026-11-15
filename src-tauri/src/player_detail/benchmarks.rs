#![doc = include_str!("benchmarks/README.md")]

use crate::data::*;
use crate::replay_detail::data::BENCHMARK_MINUTES;
use polars::prelude::*;
use std::collections::HashMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The average economy per matchup and benchmark minute of the 1v1 games of the snapshot:
/// `matchup` (`ZvT`, the player's race first), `minute`, `games` (u32) and the means `workers`,
/// `supply`, `income` (minerals and vespene per minute) and `army_value` (f64); by matchup and
/// minute.
pub fn plan(source: &dyn SnapshotSource) -> Result<LazyFrame, SwarmyError> {
    // The two rows of a 1v1 numbered, so that the other row is found even when two players have
    // the same name (two A.I. of the same level).
    let pairs = snapshot_players_of(source)?
        .filter(len().over([col("ext_fs_id")])?.eq(lit(2u32)))
        .with_column(
            int_range(lit(0), len(), 1, DataType::UInt32)
                .over([col("ext_fs_id")])?
                .alias("side"),
        );
    let others = pairs.clone().select([
        col("ext_fs_id"),
        col("side").alias("other_side"),
        col("race").str().slice(lit(0), lit(1)).alias("opponent"),
    ]);
    // The stats are by tracker player id.
    let matchups = pairs
        .join(
            others,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("other_side").neq(col("side")))
        .select([
            col("ext_fs_id"),
            col("player_id"),
            concat_str(
                [
                    col("race").str().slice(lit(0), lit(1)),
                    lit("v"),
                    col("opponent"),
                ],
                "",
                false,
            )
            .alias("matchup"),
        ]);
    let minutes = Series::new("minutes".into(), BENCHMARK_MINUTES.as_slice());
    Ok(source
        .table(GAME_MINUTES_IPC)?
        .filter(col("minute").is_in(lit(minutes).implode(false), false))
        .join(
            matchups,
            [col("ext_fs_id"), col("player_id")],
            [col("ext_fs_id"), col("player_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("matchup"), col("minute")])
        .agg([
            len().alias("games"),
            col("workers").mean(),
            col("supply").mean(),
            (col("minerals_rate") + col("vespene_rate"))
                .mean()
                .alias("income"),
            col("army_value").mean(),
        ])
        .sort_by_exprs([col("matchup"), col("minute")], Default::default()))
}

/// The average economy at a minute of a matchup in the snapshot.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct BenchmarkAverage {
    pub games: usize,
    pub workers: f32,
    pub supply: f32,
    pub income: f32,
    pub army_value: f32,
}

impl BenchmarkAverage {
    /// Adds a player game to the average, the running mean (the player insights add their own
    /// games the same way).
    pub fn add(&mut self, workers: i32, supply: i32, income: i32, army_value: i32) {
        self.games += 1;
        let weight = 1. / self.games as f32;
        self.workers += (workers as f32 - self.workers) * weight;
        self.supply += (supply as f32 - self.supply) * weight;
        self.income += (income as f32 - self.income) * weight;
        self.army_value += (army_value as f32 - self.army_value) * weight;
    }
}

/// By matchup and minute.
pub type BenchmarkTable = HashMap<(String, u32), BenchmarkAverage>;

pub fn shape(frame: DataFrame) -> Result<BenchmarkTable, SwarmyError> {
    let matchups = str_values(&frame, "matchup")?;
    let minutes = u32_values(&frame, "minute")?;
    let games = u32_values(&frame, "games")?;
    let means = |name: &str| -> Result<Vec<f32>, SwarmyError> {
        Ok(frame
            .column(name)?
            .f64()?
            .iter()
            .map(|value| value.unwrap_or_default() as f32)
            .collect())
    };
    let (workers, supply, income, army_value) = (
        means("workers")?,
        means("supply")?,
        means("income")?,
        means("army_value")?,
    );
    Ok(matchups
        .into_iter()
        .zip(minutes)
        .enumerate()
        .map(|(idx, key)| {
            (
                key,
                BenchmarkAverage {
                    games: games[idx] as usize,
                    workers: workers[idx],
                    supply: supply[idx],
                    income: income[idx],
                    army_value: army_value[idx],
                },
            )
        })
        .collect())
}

#[cfg(test)]
mod tests;
