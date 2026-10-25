//! The average time of the upgrades and expansions in the games of the same patch, so that a
//! replay can show how much faster or slower each one was.
//!
//! The upgrades are compared by when they finish, as in the build order. The expansions are
//! compared by when they start: the snapshot only has the townhalls once they finish, the build
//! time of the patch is subtracted from their average.

use super::data::{is_cosmetic_upgrade, EXPANSION_TOWNHALLS};
use crate::data::*;
use crate::replay_rerun::decisions::{closest_balance_units, unit_cost};
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap};
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

fn average(values: &[u32]) -> Option<PatchAverage> {
    if values.len() < MIN_SAMPLES {
        return None;
    }
    Some(PatchAverage {
        seconds: (values.iter().map(|value| *value as u64).sum::<u64>() / values.len() as u64)
            as u32,
        samples: values.len(),
    })
}

/// The expansions of each player of each game, by townhall type and number, from the first
/// time each townhall appears, so that a Command Center landing again is not counted.
fn expansion_seconds(
    townhalls: &DataFrame,
) -> Result<BTreeMap<(String, usize), Vec<u32>>, SwarmyError> {
    let ext_fs_ids = u64_values(townhalls, "ext_fs_id")?;
    let players = u8_values(townhalls, "control_player_id")?;
    let tag_indexes = u32_values(townhalls, "unit_tag_index")?;
    let tag_recycles = u32_values(townhalls, "unit_tag_recycle")?;
    let names = str_values(townhalls, "unit_type_name")?;
    let seconds = u32_values(townhalls, "game_seconds")?;
    // The townhalls are sorted by time, the first row of each one is when it finished.
    let mut seen = std::collections::HashSet::new();
    let mut per_player: BTreeMap<(u64, Option<u8>), usize> = BTreeMap::new();
    let mut expansions: BTreeMap<(String, usize), Vec<u32>> = BTreeMap::new();
    for idx in 0..townhalls.height() {
        if !seen.insert((ext_fs_ids[idx], tag_indexes[idx], tag_recycles[idx])) {
            continue;
        }
        let number = per_player
            .entry((ext_fs_ids[idx], players[idx]))
            .or_default();
        *number += 1;
        expansions
            .entry((names[idx].clone(), *number))
            .or_default()
            .push(seconds[idx]);
    }
    Ok(expansions)
}

/// Computes the averages of the upgrades and expansions of the games played with the protocol
/// version.
pub fn patch_timings(ipc_path: &str, protocol_version: u32) -> Result<PatchTimings, SwarmyError> {
    let games = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .filter(col("ext_protocol_version").eq(lit(protocol_version)))
        .select([col("ext_fs_id")])
        .unique(None, UniqueKeepStrategy::Any)
        .collect()?;
    let game_ids = u64_values(&games, "ext_fs_id")?;

    let upgrades_df = scan_snapshot_ipc(ipc_path, UPGRADES_IPC)?
        .filter(is_in_replays(&game_ids).and(col("ext_replay_loop").gt(lit(0))))
        .select([col("name"), game_seconds()])
        .collect()?;
    let mut upgrade_seconds: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for (name, seconds) in str_values(&upgrades_df, "name")?
        .into_iter()
        .zip(u32_values(&upgrades_df, "game_seconds")?)
    {
        if !is_cosmetic_upgrade(&name) {
            upgrade_seconds.entry(name).or_default().push(seconds);
        }
    }

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
        ])
        .sort(["game_seconds"], Default::default())
        .collect()?;
    let balance = closest_balance_units(protocol_version)?;
    let expansions = expansion_seconds(&townhalls)?
        .into_iter()
        .filter_map(|((name, number), finished)| {
            let build_seconds = unit_cost(balance, &name).time as u32;
            let started: Vec<u32> = finished
                .iter()
                .map(|seconds| seconds.saturating_sub(build_seconds))
                .collect();
            Some(((name, number), average(&started)?))
        })
        .collect();
    Ok(PatchTimings {
        upgrades: upgrade_seconds
            .into_iter()
            .filter_map(|(name, seconds)| Some((name, average(&seconds)?)))
            .collect(),
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
            "ext_fs_id" => [1u64, 1, 1, 1, 2],
            "control_player_id" => [Some(1u8), Some(1), Some(2), Some(1), Some(1)],
            "unit_tag_index" => [10u32, 11, 12, 10, 10],
            "unit_tag_recycle" => [1u32, 1, 1, 1, 1],
            "unit_type_name" => ["CommandCenter", "CommandCenter", "Nexus", "CommandCenter", "CommandCenter"],
            "game_seconds" => [100u32, 200, 110, 300, 120],
        )
        .unwrap();
        let expansions = expansion_seconds(&townhalls).unwrap();
        // The Command Center 10 landing again at 300 is not a new expansion.
        assert_eq!(
            expansions[&("CommandCenter".to_string(), 1)],
            vec![100, 120]
        );
        assert_eq!(expansions[&("CommandCenter".to_string(), 2)], vec![200]);
        assert_eq!(expansions[&("Nexus".to_string(), 1)], vec![110]);
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
