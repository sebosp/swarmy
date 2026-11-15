#![doc = include_str!("medians/README.md")]

use crate::data::*;
use crate::replay_detail::data::BENCHMARK_MINUTES;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The control of the human players in the details.
pub const HUMAN_CONTROL: u8 = 2;

/// The medians of fewer games are not shown.
pub const MIN_WINNER_SAMPLES: u32 = 5;

/// What the winners of each matchup had at the benchmark minutes in the 1v1 games of two humans
/// played with the protocol version: `race`, `enemy_race`, `minute`, the medians `workers`,
/// `supply`, `minerals_rate`, `vespene_rate`, `army_value` (i32) and `samples` (u32, at least
/// [`MIN_WINNER_SAMPLES`]); by race, enemy race and minute.
pub fn plan(source: &dyn SnapshotSource, protocol_version: u32) -> Result<LazyFrame, SwarmyError> {
    // The stats are by tracker player id, the starting townhalls tell the id of each name.
    let ids = source
        .table(GAME_PLAYERS_IPC)?
        .select([col("ext_fs_id"), col("player_name"), col("player_id")])
        .unique(
            Some(cols(["ext_fs_id", "player_name"])),
            UniqueKeepStrategy::Any,
        );
    let players = source
        .table(DETAILS_IPC)?
        .filter(
            col("player_observe")
                .eq(lit(0u8))
                .and(col("ext_protocol_version").eq(lit(protocol_version))),
        )
        .join(
            ids,
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .select([
            col("ext_fs_id"),
            col("player_id"),
            race_name_expr("player_race").alias("race"),
            col("player_result"),
            col("player_control"),
        ]);
    // The 1v1 games of two humans.
    let games = players
        .clone()
        .group_by([col("ext_fs_id")])
        .agg([
            len().alias("players"),
            col("player_control").min().alias("min_control"),
            col("player_control").max().alias("max_control"),
        ])
        .filter(
            col("players")
                .eq(lit(2u32))
                .and(col("min_control").eq(lit(HUMAN_CONTROL)))
                .and(col("max_control").eq(lit(HUMAN_CONTROL))),
        )
        .select([col("ext_fs_id")]);
    let enemies = players.clone().select([
        col("ext_fs_id"),
        col("player_id").alias("enemy_id"),
        col("race").alias("enemy_race"),
    ]);
    let winners = players
        .join(
            games,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            enemies,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(
            col("player_result")
                .eq(lit("Win"))
                .and(col("player_id").neq(col("enemy_id"))),
        )
        .select([
            col("ext_fs_id"),
            col("player_id"),
            col("race"),
            col("enemy_race"),
        ]);
    let minutes = Series::new("minutes".into(), BENCHMARK_MINUTES.as_slice());
    Ok(source
        .table(GAME_MINUTES_IPC)?
        .filter(col("minute").is_in(lit(minutes).implode(false), false))
        .join(
            winners,
            [col("ext_fs_id"), col("player_id")],
            [col("ext_fs_id"), col("player_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("race"), col("enemy_race"), col("minute")])
        .agg([
            col("workers").median().cast(DataType::Int32),
            col("supply").median().cast(DataType::Int32),
            col("minerals_rate").median().cast(DataType::Int32),
            col("vespene_rate").median().cast(DataType::Int32),
            col("army_value").median().cast(DataType::Int32),
            len().alias("samples"),
        ])
        .filter(col("samples").gt_eq(lit(MIN_WINNER_SAMPLES)))
        .sort_by_exprs(
            [col("race"), col("enemy_race"), col("minute")],
            Default::default(),
        ))
}

/// The medians by race and enemy race, with the games they come from.
pub type WinnerBenchmarks = std::collections::HashMap<(String, String), (Vec<Benchmark>, u32)>;

pub fn shape(frame: DataFrame) -> Result<WinnerBenchmarks, SwarmyError> {
    let mut benchmarks = WinnerBenchmarks::new();
    let column = |name: &str| -> Result<Vec<i32>, SwarmyError> {
        Ok(frame
            .column(name)?
            .i32()?
            .iter()
            .map(Option::unwrap_or_default)
            .collect())
    };
    let (workers, supply) = (column("workers")?, column("supply")?);
    let (minerals, vespene, army) = (
        column("minerals_rate")?,
        column("vespene_rate")?,
        column("army_value")?,
    );
    let samples = u32_values(&frame, "samples")?;
    for (idx, ((race, enemy_race), minute)) in str_values(&frame, "race")?
        .into_iter()
        .zip(str_values(&frame, "enemy_race")?)
        .zip(u32_values(&frame, "minute")?)
        .enumerate()
    {
        let entry = benchmarks.entry((race, enemy_race)).or_default();
        entry.0.push(Benchmark {
            minute,
            workers: workers[idx],
            supply: supply[idx],
            minerals_rate: minerals[idx],
            vespene_rate: vespene[idx],
            army_value: army[idx],
        });
        entry.1 = entry.1.max(samples[idx]);
    }
    Ok(benchmarks)
}

#[cfg(test)]
mod tests;
