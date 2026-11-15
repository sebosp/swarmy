//! What the winners of the same matchup had at 04:00, 06:00, 08:00 and 10:00 in the games of the
//! patch: the median workers, supply, income and army value of the human players who won a 1v1
//! with the same race against the same race, so that a replay shows where it was behind the
//! games that were won. The medians are the query file `medians.rs` (AGENTS.md, "A query is a
//! file"), computed once per snapshot generation.

pub mod medians;

use crate::data::{IpcSnapshot, QueryMeta, snapshot_generation};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

pub use medians::{MIN_WINNER_SAMPLES, WinnerBenchmarks};

/// The queries of the benchmarks, for the debug links.
pub const QUERIES: [QueryMeta; 1] = [medians::META];

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
    let source = IpcSnapshot::at(ipc_path);
    medians::shape(medians::plan(&source, protocol_version)?.collect()?)
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
