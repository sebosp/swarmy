//! The average time of the upgrades and expansions in the games of the same patch, so that a
//! replay can show how much faster or slower each one was.
//!
//! The upgrades are compared by when they finish, as in the build order. The expansions are
//! compared by when they start: the snapshot only has the townhalls once they finish, the build
//! time of the patch is subtracted from their average.

use super::data::{cosmetic_upgrade_expr, EXPANSION_TOWNHALLS};
use crate::data::*;
use crate::replay_rerun::decisions::{closest_balance_units, unit_cost};
use polars::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

/// The averages with fewer samples are not shown.
pub const MIN_SAMPLES: usize = 3;

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

/// The average seconds and the samples of each group, the groups with fewer than [`MIN_SAMPLES`]
/// are left out.
fn averages(query: LazyFrame, by: &[&str]) -> LazyFrame {
    query
        .group_by(by.iter().map(|name| col(*name)).collect::<Vec<_>>())
        .agg([
            col("game_seconds").mean().cast(DataType::UInt32),
            len().cast(DataType::UInt64).alias("samples"),
        ])
        .filter(col("samples").gt_eq(lit(MIN_SAMPLES as u64)))
}

fn patch_average(seconds: u32, samples: u64) -> PatchAverage {
    PatchAverage {
        seconds,
        samples: samples as usize,
    }
}

/// The time the nth expansion of each townhall type finishes, from the townhalls of each player
/// sorted by time. The first time each townhall appears is when it finished, a Command Center
/// landing again is not counted.
fn expansion_averages(townhalls: LazyFrame) -> Result<LazyFrame, SwarmyError> {
    let finished = townhalls
        .sort(["game_seconds"], Default::default())
        .unique_stable(
            Some(cols(["ext_fs_id", "unit_tag_index", "unit_tag_recycle"])),
            UniqueKeepStrategy::First,
        )
        .with_column(
            (int_range(lit(0), len(), 1, DataType::UInt32) + lit(1u32))
                .over([col("ext_fs_id"), col("control_player_id")])?
                .alias("number"),
        );
    Ok(averages(finished, &["unit_type_name", "number"]))
}

/// Computes the averages of the upgrades and expansions of the games played with the protocol
/// version.
pub fn patch_timings(ipc_path: &str, protocol_version: u32) -> Result<PatchTimings, SwarmyError> {
    // The games of the patch are few, as a literal list their ids are pushed down to the scans.
    let games = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .filter(col("ext_protocol_version").eq(lit(protocol_version)))
        .select([col("ext_fs_id")])
        .unique(None, UniqueKeepStrategy::Any)
        .collect()?;
    let game_ids = u64_values(&games, "ext_fs_id")?;

    let upgrades = scan_snapshot_ipc(ipc_path, UPGRADES_IPC)?
        .filter(
            is_in_replays(&game_ids)
                .and(col("ext_replay_loop").gt(lit(0)))
                .and(cosmetic_upgrade_expr("name").not()),
        )
        .select([col("name"), game_seconds()]);
    let townhalls = scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
        .filter(
            is_in_replays(&game_ids)
                .and(col("ext_replay_loop").gt(lit(0)))
                .and(col("unit_type_name").is_in(
                    lit(Series::new("townhalls".into(), EXPANSION_TOWNHALLS)).implode(false),
                    false,
                )),
        )
        .select([
            col("ext_fs_id"),
            col("control_player_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            col("unit_type_name"),
            game_seconds(),
        ]);
    let [upgrades_df, expansions_df]: [DataFrame; 2] = collect_all(vec![
        averages(upgrades, &["name"]),
        expansion_averages(townhalls)?,
    ])?
    .try_into()
    .map_err(|_| SwarmyError::Other("The patch timings returned no results".to_string()))?;

    let upgrades = str_values(&upgrades_df, "name")?
        .into_iter()
        .zip(u32_values(&upgrades_df, "game_seconds")?)
        .zip(u64_values(&upgrades_df, "samples")?)
        .map(|((name, seconds), samples)| (name, patch_average(seconds, samples)))
        .collect();
    // The townhalls are in the snapshot once they finish, the build time of the patch is
    // subtracted to compare when they start.
    let balance = closest_balance_units(protocol_version)?;
    let expansions = str_values(&expansions_df, "unit_type_name")?
        .into_iter()
        .zip(u32_values(&expansions_df, "number")?)
        .zip(u32_values(&expansions_df, "game_seconds")?)
        .zip(u64_values(&expansions_df, "samples")?)
        .map(|(((name, number), finished), samples)| {
            let build_seconds = unit_cost(balance, &name).time as u32;
            let average = patch_average(finished.saturating_sub(build_seconds), samples);
            ((name, number as usize), average)
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
    fn it_numbers_the_expansions_once_per_townhall() {
        let townhalls = df!(
            "ext_fs_id" => [1u64, 1, 1, 1, 2, 3, 4],
            "control_player_id" => [Some(1u8), Some(1), Some(2), Some(1), Some(1), Some(1), Some(1)],
            "unit_tag_index" => [10u32, 11, 12, 10, 10, 10, 10],
            "unit_tag_recycle" => [1u32, 1, 1, 1, 1, 1, 1],
            "unit_type_name" => ["CommandCenter", "CommandCenter", "Nexus", "CommandCenter", "CommandCenter", "CommandCenter", "CommandCenter"],
            "game_seconds" => [100u32, 200, 110, 300, 120, 140, 150],
        )
        .unwrap();
        let expansions = expansion_averages(townhalls.lazy())
            .unwrap()
            .sort(["unit_type_name", "number"], Default::default())
            .collect()
            .unwrap();
        // The Command Center 10 landing again at 300 is not a new expansion, the second
        // Command Center and the Nexus have too few samples.
        assert_eq!(
            str_values(&expansions, "unit_type_name").unwrap(),
            ["CommandCenter"]
        );
        assert_eq!(u32_values(&expansions, "number").unwrap(), [1]);
        assert_eq!(u32_values(&expansions, "game_seconds").unwrap(), [127]);
        assert_eq!(u64_values(&expansions, "samples").unwrap(), [4]);
    }

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
