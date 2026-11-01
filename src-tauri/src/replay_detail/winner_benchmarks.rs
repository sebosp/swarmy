//! What the winners of the same matchup had at 04:00, 06:00, 08:00 and 10:00 in the games of the
//! patch: the median workers, supply, income and army value of the human players who won a 1v1
//! with the same race against the same race, so that a replay shows where it was behind the
//! games that were won.

use super::data::BENCHMARK_MINUTES;
use crate::data::*;
use crate::replay_rerun::viewer::GAME_LOOPS_PER_SECOND;
use polars::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

/// The control of the human players in the details.
const HUMAN_CONTROL: u8 = 2;

/// The medians of fewer games are not shown.
pub const MIN_WINNER_SAMPLES: u32 = 5;

/// The stats of a benchmark minute are the last ones of the seconds before it, they are written
/// every 7 seconds.
const STATS_WINDOW_SECONDS: f64 = 15.;

/// The medians by race, enemy race and minute, with the games they come from.
pub type WinnerBenchmarks = HashMap<(String, String), (Vec<Benchmark>, u32)>;

type CacheKey = (String, u32, Option<std::time::SystemTime>);

fn cache() -> &'static Mutex<HashMap<CacheKey, Arc<WinnerBenchmarks>>> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Arc<WinnerBenchmarks>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// The benchmarks of the winners of the patch, computed once per snapshot generation.
pub fn winner_benchmarks_cached(
    ipc_path: &str,
    protocol_version: u32,
) -> Result<Arc<WinnerBenchmarks>, SwarmyError> {
    let key = (
        ipc_path.to_string(),
        protocol_version,
        snapshot_generation(ipc_path),
    );
    if let Some(benchmarks) = cache().lock().unwrap().get(&key) {
        return Ok(benchmarks.clone());
    }
    let benchmarks = Arc::new(winner_benchmarks(ipc_path, protocol_version)?);
    cache().lock().unwrap().insert(key, benchmarks.clone());
    Ok(benchmarks)
}

pub fn winner_benchmarks(
    ipc_path: &str,
    protocol_version: u32,
) -> Result<WinnerBenchmarks, SwarmyError> {
    // The tracker player id is the position of the player in the details.
    let players = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .filter(col("ext_protocol_version").eq(lit(protocol_version)))
        .with_columns([
            (int_range(lit(0), len(), 1, DataType::UInt32) + lit(1u32))
                .over([col("ext_fs_id")])?
                .cast(DataType::UInt8)
                .alias("player_id"),
            race_name_expr("player_race").alias("race"),
        ])
        .with_column(len().over([col("ext_fs_id")])?.alias("players"))
        .with_column(
            col("player_control")
                .eq(lit(HUMAN_CONTROL))
                .all(true)
                .over([col("ext_fs_id")])?
                .alias("humans"),
        )
        .filter(col("players").eq(lit(2u32)).and(col("humans")))
        .select([
            col("ext_fs_id"),
            col("player_id"),
            col("race"),
            col("player_result"),
        ])
        .collect()?;
    let enemies = players.clone().lazy().select([
        col("ext_fs_id"),
        col("player_id").alias("enemy_id"),
        col("race").alias("enemy_race"),
    ]);
    let winners = players
        .clone()
        .lazy()
        .filter(col("player_result").eq(lit("Win")))
        .join(
            enemies,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("player_id").neq(col("enemy_id")))
        .select([
            col("ext_fs_id"),
            col("player_id"),
            col("race"),
            col("enemy_race"),
        ]);
    let game_ids = u64_values(&players, "ext_fs_id")?;
    let stats = scan_snapshot_ipc(ipc_path, STATS_IPC)?.filter(is_in_replays(&game_ids));
    let minutes = BENCHMARK_MINUTES
        .iter()
        .map(|minute| {
            let end = *minute as f64 * 60. * GAME_LOOPS_PER_SECOND;
            let start = end - STATS_WINDOW_SECONDS * GAME_LOOPS_PER_SECOND;
            stats
                .clone()
                .filter(
                    col("ext_replay_loop")
                        .gt(lit(start as i64))
                        .and(col("ext_replay_loop").lt_eq(lit(end as i64))),
                )
                .sort(["ext_replay_loop"], Default::default())
                .group_by([col("ext_fs_id"), col("player_id")])
                .agg([
                    col("workers_active_count").last().alias("workers"),
                    col("food_used").last().alias("supply"),
                    col("minerals_collection_rate")
                        .last()
                        .alias("minerals_rate"),
                    col("vespene_collection_rate").last().alias("vespene_rate"),
                    (col("minerals_used_current_army") + col("vespene_used_current_army"))
                        .last()
                        .alias("army_value"),
                ])
                .with_column(lit(*minute).cast(DataType::UInt32).alias("minute"))
        })
        .collect::<Vec<_>>();
    let medians = concat(minutes, Default::default())?
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
            len().cast(DataType::UInt32).alias("samples"),
        ])
        .filter(col("samples").gt_eq(lit(MIN_WINNER_SAMPLES)))
        .sort(["minute"], Default::default())
        .collect()?;
    let mut benchmarks = WinnerBenchmarks::new();
    let column = |name: &str| -> Result<Vec<i32>, SwarmyError> {
        Ok(medians
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
    let samples = u32_values(&medians, "samples")?;
    for (idx, ((race, enemy_race), minute)) in str_values(&medians, "race")?
        .into_iter()
        .zip(str_values(&medians, "enemy_race")?)
        .zip(u32_values(&medians, "minute")?)
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

/// Sets the benchmarks of the winners of the matchup of the players of a 1v1.
pub fn annotate(detail: &mut ReplayDetail, benchmarks: &WinnerBenchmarks) {
    if detail.players.len() != 2 {
        return;
    }
    let races: Vec<String> = detail
        .players
        .iter()
        .map(|summary| summary.player.race.clone())
        .collect();
    for (idx, summary) in detail.players.iter_mut().enumerate() {
        let key = (races[idx].clone(), races[1 - idx].clone());
        if let Some((winners, games)) = benchmarks.get(&key) {
            summary.winner_benchmarks = winners.clone();
            summary.winner_games = *games;
        }
    }
}

#[cfg(test)]
mod snapshot_tests {
    /// `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test winner_benchmarks_of -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn winner_benchmarks_of_a_snapshot() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let version = std::env::var("SWARMY_TEST_PROTOCOL_VERSION")
            .map(|version| version.parse().unwrap())
            .unwrap_or(94137);
        let ipc_path = format!("{}/{}", replay_path, swarmy_common::IPC_DIR);
        let start = std::time::Instant::now();
        let benchmarks = super::winner_benchmarks(&ipc_path, version).unwrap();
        eprintln!("{:?}", start.elapsed());
        for (key, (values, games)) in &benchmarks {
            eprintln!("{key:?} {games} games {values:?}");
        }
        assert!(!benchmarks.is_empty());
    }
}
