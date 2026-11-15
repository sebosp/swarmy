//! The average time of the upgrades and expansions in the games of the same patch, so that a
//! replay can show how much faster or slower each one was.
//!
//! The upgrades are compared by when they finish, as in the build order. The expansions are
//! compared by when they start: the snapshot only has the townhalls once they finish, the build
//! time of the patch is subtracted from their average. The averages are query files of this
//! directory (AGENTS.md, "A query is a file").

pub mod expansion_averages;
pub mod games;
pub mod upgrade_averages;

use super::data::EXPANSION_TOWNHALLS;
use crate::data::{IpcSnapshot, QueryMeta, collect_all};
use crate::replay_rerun::decisions::{closest_balance_units, unit_cost};
use polars::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

pub use upgrade_averages::MIN_SAMPLES;

/// The queries of the timings, for the debug links.
pub const QUERIES: [QueryMeta; 3] = [
    games::META,
    upgrade_averages::META,
    expansion_averages::META,
];

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PatchTimings {
    /// The average time each upgrade finishes.
    pub upgrades: HashMap<String, PatchAverage>,
    /// The average time the nth expansion of each townhall type starts, 1 is the first one
    /// after the initial townhall.
    pub expansions: HashMap<(String, usize), PatchAverage>,
}

/// The snapshot directory, the protocol version and the time of the last Optimize.
type CacheKey = (String, u32, Option<std::time::SystemTime>);

fn timings_cache() -> &'static Mutex<HashMap<CacheKey, Arc<PatchTimings>>> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Arc<PatchTimings>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// The timings of the patch, computed once per snapshot generation.
pub fn patch_timings_cached(
    ipc_path: &str,
    protocol_version: u32,
) -> Result<Arc<PatchTimings>, SwarmyError> {
    let generated = std::fs::metadata(
        std::path::Path::new(ipc_path).join(s2protocol::snapshot::SNAPSHOT_METADATA_FNAME),
    )
    .and_then(|metadata| metadata.modified())
    .ok();
    let key = (ipc_path.to_string(), protocol_version, generated);
    if let Some(timings) = timings_cache().lock().unwrap().get(&key) {
        return Ok(timings.clone());
    }
    let timings = Arc::new(patch_timings(ipc_path, protocol_version)?);
    timings_cache().lock().unwrap().insert(key, timings.clone());
    Ok(timings)
}

/// Computes the averages of the upgrades and expansions of the games played with the protocol
/// version.
pub fn patch_timings(ipc_path: &str, protocol_version: u32) -> Result<PatchTimings, SwarmyError> {
    let source = IpcSnapshot::at(ipc_path);
    // The games of the patch are few, as a literal list their ids are pushed down to the scans.
    let game_ids = games::shape(games::plan(&source, protocol_version)?.collect()?)?;
    let [upgrades_df, expansions_df]: [DataFrame; 2] = collect_all(vec![
        upgrade_averages::plan(&source, &game_ids)?,
        expansion_averages::plan(&source, &game_ids)?,
    ])?
    .try_into()
    .map_err(|_| SwarmyError::Other("The patch timings returned no results".to_string()))?;
    let upgrades = upgrade_averages::shape(upgrades_df)?;
    // The townhalls are in the snapshot once they finish, the build time of the patch is
    // subtracted to compare when they start.
    let balance = closest_balance_units(protocol_version)?;
    let expansions = expansion_averages::shape(expansions_df)?
        .into_iter()
        .map(|((name, number), (finished, samples))| {
            let build_seconds = unit_cost(balance, &name).time as u32;
            let average = PatchAverage {
                seconds: finished.saturating_sub(build_seconds),
                samples,
            };
            ((name, number), average)
        })
        .collect();
    Ok(PatchTimings {
        upgrades,
        expansions,
    })
}

/// Sets the patch averages of the upgrades and expansions of the replay.
pub fn annotate(detail: &mut ReplayDetail, timings: &PatchTimings) {
    for summary in &mut detail.players {
        let mut numbers: HashMap<String, usize> = HashMap::new();
        for entry in &mut summary.build_order {
            if entry.is_upgrade {
                entry.patch_average = timings.upgrades.get(&entry.name).cloned();
            } else if EXPANSION_TOWNHALLS.contains(&entry.name.as_str()) {
                let number = numbers.entry(entry.name.clone()).or_default();
                *number += 1;
                entry.patch_average = timings
                    .expansions
                    .get(&(entry.name.clone(), *number))
                    .cloned();
            }
        }
        for (number, entry) in summary.expansions.iter_mut().enumerate() {
            entry.patch_average = timings
                .expansions
                .get(&(entry.name.clone(), number + 1))
                .cloned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_compares_to_the_average() {
        let average = PatchAverage {
            seconds: 100,
            samples: 3,
        };
        assert_eq!(average.compare(95), "5s faster than average");
        assert_eq!(average.compare(107), "7s slower than average");
        assert_eq!(average.compare(100), "same as average");
    }
}

#[cfg(test)]
mod snapshot_tests {
    /// Runs against a real snapshot: `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test -- --ignored`
    #[test]
    #[ignore]
    fn it_computes_the_patch_timings_of_a_snapshot() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let version = std::env::var("SWARMY_TEST_PROTOCOL_VERSION")
            .map(|version| version.parse().unwrap())
            .unwrap_or(95841);
        let ipc_path = format!("{}/{}", replay_path, swarmy_common::IPC_DIR);
        let start = std::time::Instant::now();
        let timings = super::patch_timings(&ipc_path, version).unwrap();
        eprintln!("{:?}", start.elapsed());
        let mut upgrades: Vec<_> = timings.upgrades.iter().collect();
        upgrades.sort_by(|a, b| a.0.cmp(b.0));
        let mut expansions: Vec<_> = timings.expansions.iter().collect();
        expansions.sort_by(|a, b| a.0.cmp(b.0));
        eprintln!("{upgrades:?}");
        eprintln!("{expansions:?}");
        assert!(!timings.upgrades.is_empty());
    }
}
