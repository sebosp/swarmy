//! What the Player view needs from each game of a player. Optimize computes it from the files of
//! the snapshot of each day (`snapshot_insights`, the replays are not read) and writes a row per
//! game and player next to the day; the view adds up the rows of the games of the player.

use polars::prelude::*;
use rayon::prelude::*;
use s2protocol::arrow_store::partitioned::{list_partitions, partition_dir};
use s2protocol::snapshot::{SnapshotMetadata, S2PROTOCOL_VERSION};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use swarmy_common::*;

pub use super::snapshot_insights::name_without_clan;

/// A row per game and player, a column per field of [`PlayerGameInsight`], the lists of each game
/// as list columns (`benchmark_minute`, `benchmark_workers`...).
const INSIGHTS_FILE: &str = "swarmy_player_games.arrow";
/// What the file of the day was computed from, written last.
const INSIGHTS_SOURCE_FILE: &str = "swarmy_player_games.json";
/// Changes when [`PlayerGameInsight`] or its computation change, the days are computed again.
/// 4: the first scout is not an egg or a larva; 3: the idle production without the time the
/// supply was full; 2: from the snapshot, typed columns; 1 was the analysis of the replays as JSON.
const INSIGHTS_VERSION: u32 = 4;

/// The text that tells whether the file of a day is current: the day and the analysis.
fn day_source(day: &Path) -> Option<String> {
    let metadata = SnapshotMetadata::read_from_dir(day).ok()?;
    (metadata.s2protocol_version == S2PROTOCOL_VERSION)
        .then(|| format!("{INSIGHTS_VERSION}:{}", metadata.created_at.to_rfc3339()))
}

/// The days of the current version whose games were not analysed.
pub fn find_gaps(ipc_path: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    Ok(list_partitions(ipc_path)?
        .into_iter()
        .filter(|day| {
            day_source(day).is_some_and(|source| {
                std::fs::read_to_string(day.join(INSIGHTS_SOURCE_FILE))
                    .ok()
                    .as_deref()
                    != Some(source.as_str())
            })
        })
        .collect())
}

/// The games of a day.
fn day_games(day: &Path) -> Result<usize, SwarmyError> {
    Ok(LazyFrame::scan_ipc(
        PlRefPath::new(day.join(DETAILS_IPC).to_string_lossy().as_ref()),
        Default::default(),
        UnifiedScanArgs {
            hive_options: HiveOptions::new_disabled(),
            ..Default::default()
        },
    )?
    .select([col("ext_fs_id").n_unique()])
    .collect()?
    .column("ext_fs_id")?
    .cast(&DataType::UInt64)?
    .u64()?
    .get(0)
    .unwrap_or_default() as usize)
}

/// A list column, a list per row.
fn list_column<T, F>(name: &str, rows: &[(u64, String, PlayerGameInsight)], values: F) -> Column
where
    F: Fn(&PlayerGameInsight) -> Vec<T>,
    Series: NamedFrom<Vec<T>, [T]>,
{
    let lists: Vec<Series> = rows
        .iter()
        .map(|(_, _, insight)| Series::new("".into(), values(insight)))
        .collect();
    let list: ListChunked = lists.into_iter().map(Some).collect();
    list.with_name(name.into()).into_series().into_column()
}

/// The rows of the insights as a frame of typed columns.
fn insights_frame(rows: &[(u64, String, PlayerGameInsight)]) -> Result<DataFrame, SwarmyError> {
    macro_rules! scalar {
        ($name:literal, $ty:ty, $value:expr) => {
            Column::new(
                $name.into(),
                rows.iter()
                    .map(|(_, _, insight)| $value(insight))
                    .collect::<Vec<$ty>>(),
            )
        };
    }
    let columns = vec![
        Column::new(
            "ext_fs_id".into(),
            rows.iter().map(|row| row.0).collect::<Vec<u64>>(),
        ),
        Column::new(
            "name".into(),
            rows.iter().map(|row| row.1.as_str()).collect::<Vec<&str>>(),
        ),
        scalar!("won", bool, |i: &PlayerGameInsight| i.won),
        scalar!("matchup", String, |i: &PlayerGameInsight| i.matchup.clone()),
        scalar!("map_title", String, |i: &PlayerGameInsight| i
            .map_title
            .clone()),
        scalar!("duration_seconds", u32, |i: &PlayerGameInsight| i
            .duration_seconds),
        list_column("opening", rows, |i| i.opening.clone()),
        scalar!(
            "first_army_unit",
            Option<String>,
            |i: &PlayerGameInsight| i.first_army_unit.clone()
        ),
        scalar!(
            "favourite_army_unit",
            Option<String>,
            |i: &PlayerGameInsight| i.favourite_army_unit.clone()
        ),
        list_column("benchmark_minute", rows, |i| {
            i.benchmarks.iter().map(|b| b.minute).collect()
        }),
        list_column("benchmark_workers", rows, |i| {
            i.benchmarks.iter().map(|b| b.workers).collect()
        }),
        list_column("benchmark_supply", rows, |i| {
            i.benchmarks.iter().map(|b| b.supply).collect()
        }),
        list_column("benchmark_minerals_rate", rows, |i| {
            i.benchmarks.iter().map(|b| b.minerals_rate).collect()
        }),
        list_column("benchmark_vespene_rate", rows, |i| {
            i.benchmarks.iter().map(|b| b.vespene_rate).collect()
        }),
        list_column("benchmark_army_value", rows, |i| {
            i.benchmarks.iter().map(|b| b.army_value).collect()
        }),
        list_column("production_idle_structure", rows, |i| {
            i.production_idle
                .iter()
                .map(|p| p.structure.clone())
                .collect()
        }),
        list_column("production_idle_structures", rows, |i| {
            i.production_idle
                .iter()
                .map(|p| p.structures as u32)
                .collect()
        }),
        list_column("production_idle_seconds", rows, |i| {
            i.production_idle.iter().map(|p| p.idle_seconds).collect()
        }),
        scalar!(
            "first_scout_unit",
            Option<String>,
            |i: &PlayerGameInsight| i.first_scout_unit.clone()
        ),
        scalar!(
            "first_scout_seconds",
            Option<u32>,
            |i: &PlayerGameInsight| i.first_scout_seconds
        ),
        scalar!("first_upgrade", Option<String>, |i: &PlayerGameInsight| i
            .first_upgrade
            .clone()),
        list_column("killed_by_unit", rows, |i| {
            i.killed_by.iter().map(|u| u.unit.clone()).collect()
        }),
        list_column("killed_by_count", rows, |i| {
            i.killed_by.iter().map(|u| u.count as u32).collect()
        }),
        list_column("army_at_10_unit", rows, |i| {
            i.army_at_10.iter().map(|u| u.unit.clone()).collect()
        }),
        list_column("army_at_10_count", rows, |i| {
            i.army_at_10.iter().map(|u| u.count as u32).collect()
        }),
        scalar!("was_behind", bool, |i: &PlayerGameInsight| i.was_behind),
        scalar!("proxy", bool, |i: &PlayerGameInsight| i.proxy),
        scalar!("base_trade", bool, |i: &PlayerGameInsight| i.base_trade),
        scalar!("rating", Option<i32>, |i: &PlayerGameInsight| i.rating),
        scalar!("supply_blocked_seconds", f32, |i: &PlayerGameInsight| i
            .supply_blocked_seconds),
        scalar!("floating_seconds", f32, |i: &PlayerGameInsight| i
            .floating_seconds),
        scalar!("avg_unspent", f32, |i: &PlayerGameInsight| i.avg_unspent),
        scalar!("injects", f32, |i: &PlayerGameInsight| i.injects),
        scalar!("chrono_boosts", f32, |i: &PlayerGameInsight| i
            .chrono_boosts),
        scalar!("mules", f32, |i: &PlayerGameInsight| i.mules),
        scalar!("commands_per_minute", f32, |i: &PlayerGameInsight| i
            .commands_per_minute),
        scalar!("resources_lost", f32, |i: &PlayerGameInsight| i
            .resources_lost),
        scalar!("resources_killed", f32, |i: &PlayerGameInsight| i
            .resources_killed),
        scalar!("mining_seconds_lost", f32, |i: &PlayerGameInsight| i
            .mining_seconds_lost),
        scalar!("bases_at_10", f32, |i: &PlayerGameInsight| i.bases_at_10),
        scalar!(
            "first_kill_seconds",
            Option<u32>,
            |i: &PlayerGameInsight| i.first_kill_seconds
        ),
        list_column("energy_ability", rows, |i| {
            i.energy.iter().map(|e| e.ability.clone()).collect()
        }),
        list_column("energy_caster", rows, |i| {
            i.energy.iter().map(|e| e.caster.clone()).collect()
        }),
        list_column("energy_uses", rows, |i| {
            i.energy.iter().map(|e| e.uses).collect()
        }),
        list_column("energy_caster_minutes", rows, |i| {
            i.energy.iter().map(|e| e.caster_minutes).collect()
        }),
        list_column("energy_possible", rows, |i| {
            i.energy.iter().map(|e| e.possible).collect()
        }),
    ];
    Ok(DataFrame::new(rows.len(), columns)?)
}

/// Computes the insights of the games of a day from its files and writes them.
fn build_day(day: &Path, source: &str, done: &(dyn Fn(usize) + Sync)) -> Result<(), SwarmyError> {
    let _ = std::fs::remove_file(day.join(INSIGHTS_SOURCE_FILE));
    let rows = super::snapshot_insights::day_insights(day)?;
    let mut frame = insights_frame(&rows)?;
    // Not `.tmp`: a day with a `.tmp` file is an interrupted generation, this one has ended.
    let tmp = day.join(format!("{INSIGHTS_FILE}.partial"));
    IpcWriter::new(std::fs::File::create(&tmp)?)
        .with_compression(Some(IpcCompression::ZSTD(Default::default())))
        .finish(&mut frame)?;
    std::fs::rename(tmp, day.join(INSIGHTS_FILE))?;
    std::fs::write(day.join(INSIGHTS_SOURCE_FILE), source)?;
    done(day_games(day).unwrap_or_default());
    Ok(())
}

/// Analyses the games of the days that were not analysed. `progress` is told the replays done and
/// their total.
pub fn build_missing(
    ipc_path: &Path,
    progress: &(dyn Fn(usize, usize) + Sync),
) -> Result<usize, SwarmyError> {
    let days = find_gaps(ipc_path)?;
    let total: usize = days.iter().filter_map(|day| day_games(day).ok()).sum();
    let finished = AtomicUsize::new(0);
    let step = (total / 100).max(1);
    let done = |games: usize| {
        let before = finished.fetch_add(games, Ordering::Relaxed);
        let after = before + games;
        if after / step != before / step || after == total {
            progress(after, total);
        }
    };
    days.par_iter()
        .filter_map(|day| Some((day, day_source(day)?)))
        .try_for_each(|(day, source)| {
            build_day(day, &source, &done).map_err(|err| format!("{}: {err:?}", day.display()))
        })
        .map_err(SwarmyError::Other)?;
    Ok(days.len())
}

/// The analysis of the games, by game and name, of the days of the snapshot they were played; the
/// games without a row, i.e. added since the last Optimize, are not in it.
pub fn read_game_insights(
    ipc_path: &str,
    games: &[&PlayerMatch],
) -> Result<HashMap<(u64, String), PlayerGameInsight>, SwarmyError> {
    let days: HashSet<PathBuf> = games
        .iter()
        .filter_map(|game| game.date.parse().ok())
        .map(partition_dir)
        .collect();
    let scans = days
        .iter()
        .map(|day| Path::new(ipc_path).join(day))
        // The days computed by this version, an older file has other columns.
        .filter(|day| {
            day_source(day).is_some_and(|source| {
                std::fs::read_to_string(day.join(INSIGHTS_SOURCE_FILE))
                    .ok()
                    .as_deref()
                    == Some(source.as_str())
            })
        })
        .map(|day| day.join(INSIGHTS_FILE))
        .filter(|path| path.is_file())
        .map(|path| {
            LazyFrame::scan_ipc(
                PlRefPath::new(path.to_string_lossy().as_ref()),
                Default::default(),
                UnifiedScanArgs {
                    hive_options: HiveOptions::new_disabled(),
                    ..Default::default()
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if scans.is_empty() {
        return Ok(HashMap::new());
    }
    let ids: Vec<u64> = games.iter().map(|game| game.ext_fs_id).collect();
    let rows = concat(scans, UnionArgs::default())?
        .filter(crate::data::is_in_replays(&ids))
        .collect()?;
    decode_insights(&rows)
}

/// The rows of a frame of [`insights_frame`], by game and name.
fn decode_insights(
    rows: &DataFrame,
) -> Result<HashMap<(u64, String), PlayerGameInsight>, SwarmyError> {
    let column = |name: &str| rows.column(name).map_err(SwarmyError::from);
    let strs = |name: &str| -> Result<Vec<Option<String>>, SwarmyError> {
        Ok(column(name)?
            .str()?
            .iter()
            .map(|value| value.map(str::to_string))
            .collect())
    };
    let bools = |name: &str| -> Result<Vec<bool>, SwarmyError> {
        Ok(column(name)?
            .bool()?
            .iter()
            .map(Option::unwrap_or_default)
            .collect())
    };
    let f32s = |name: &str| -> Result<Vec<f32>, SwarmyError> {
        Ok(column(name)?
            .f32()?
            .iter()
            .map(Option::unwrap_or_default)
            .collect())
    };
    let u32s = |name: &str| -> Result<Vec<Option<u32>>, SwarmyError> {
        Ok(column(name)?.u32()?.iter().collect())
    };
    let lists = |name: &str| -> Result<Vec<Series>, SwarmyError> {
        let list = column(name)?.list()?.clone();
        Ok((0..list.len())
            .map(|idx| list.get_as_series(idx).unwrap_or_default())
            .collect())
    };
    let list_strs = |name: &str| -> Result<Vec<Vec<String>>, SwarmyError> {
        lists(name)?
            .iter()
            .map(|series| {
                Ok(series
                    .str()?
                    .iter()
                    .map(|value| value.unwrap_or_default().to_string())
                    .collect())
            })
            .collect()
    };
    let list_u32s = |name: &str| -> Result<Vec<Vec<u32>>, SwarmyError> {
        lists(name)?
            .iter()
            .map(|series| {
                Ok(series
                    .u32()?
                    .iter()
                    .map(Option::unwrap_or_default)
                    .collect())
            })
            .collect()
    };
    let list_i32s = |name: &str| -> Result<Vec<Vec<i32>>, SwarmyError> {
        lists(name)?
            .iter()
            .map(|series| {
                Ok(series
                    .i32()?
                    .iter()
                    .map(Option::unwrap_or_default)
                    .collect())
            })
            .collect()
    };
    let list_f32s = |name: &str| -> Result<Vec<Vec<f32>>, SwarmyError> {
        lists(name)?
            .iter()
            .map(|series| {
                Ok(series
                    .f32()?
                    .iter()
                    .map(Option::unwrap_or_default)
                    .collect())
            })
            .collect()
    };
    let ids = crate::data::u64_values(rows, "ext_fs_id")?;
    let names = crate::data::str_values(rows, "name")?;
    let energy_ability = list_strs("energy_ability")?;
    let energy_caster = list_strs("energy_caster")?;
    let energy_uses = list_u32s("energy_uses")?;
    let energy_minutes = list_f32s("energy_caster_minutes")?;
    let energy_possible = list_f32s("energy_possible")?;
    let (won, matchup, map_title) = (bools("won")?, strs("matchup")?, strs("map_title")?);
    let duration = u32s("duration_seconds")?;
    let opening = list_strs("opening")?;
    let (first_army, favourite) = (strs("first_army_unit")?, strs("favourite_army_unit")?);
    let bm_minute = list_u32s("benchmark_minute")?;
    let bm_workers = list_i32s("benchmark_workers")?;
    let bm_supply = list_i32s("benchmark_supply")?;
    let bm_minerals = list_i32s("benchmark_minerals_rate")?;
    let bm_vespene = list_i32s("benchmark_vespene_rate")?;
    let bm_army = list_i32s("benchmark_army_value")?;
    let pi_structure = list_strs("production_idle_structure")?;
    let pi_structures = list_u32s("production_idle_structures")?;
    let pi_seconds = list_u32s("production_idle_seconds")?;
    let (scout_unit, scout_seconds) = (strs("first_scout_unit")?, u32s("first_scout_seconds")?);
    let first_upgrade = strs("first_upgrade")?;
    let (kb_unit, kb_count) = (list_strs("killed_by_unit")?, list_u32s("killed_by_count")?);
    let (a10_unit, a10_count) = (
        list_strs("army_at_10_unit")?,
        list_u32s("army_at_10_count")?,
    );
    let (was_behind, proxy, base_trade) =
        (bools("was_behind")?, bools("proxy")?, bools("base_trade")?);
    let rating: Vec<Option<i32>> = column("rating")?.i32()?.iter().collect();
    let supply_blocked = f32s("supply_blocked_seconds")?;
    let floating = f32s("floating_seconds")?;
    let avg_unspent = f32s("avg_unspent")?;
    let (injects, chrono, mules) = (f32s("injects")?, f32s("chrono_boosts")?, f32s("mules")?);
    let cpm = f32s("commands_per_minute")?;
    let (lost, killed) = (f32s("resources_lost")?, f32s("resources_killed")?);
    let mining = f32s("mining_seconds_lost")?;
    let bases_at_10 = f32s("bases_at_10")?;
    let first_kill = u32s("first_kill_seconds")?;
    let counts = |units: &[String], counts: &[u32]| -> Vec<UnitCount> {
        units
            .iter()
            .zip(counts)
            .map(|(unit, count)| UnitCount {
                unit: unit.clone(),
                count: *count as usize,
            })
            .collect()
    };
    Ok((0..rows.height())
        .map(|idx| {
            let insight = PlayerGameInsight {
                won: won[idx],
                matchup: matchup[idx].clone().unwrap_or_default(),
                map_title: map_title[idx].clone().unwrap_or_default(),
                duration_seconds: duration[idx].unwrap_or_default(),
                opening: opening[idx].clone(),
                first_army_unit: first_army[idx].clone(),
                favourite_army_unit: favourite[idx].clone(),
                benchmarks: (0..bm_minute[idx].len())
                    .map(|b| Benchmark {
                        minute: bm_minute[idx][b],
                        workers: bm_workers[idx][b],
                        supply: bm_supply[idx][b],
                        minerals_rate: bm_minerals[idx][b],
                        vespene_rate: bm_vespene[idx][b],
                        army_value: bm_army[idx][b],
                    })
                    .collect(),
                production_idle: (0..pi_structure[idx].len())
                    .map(|p| ProductionIdle {
                        structure: pi_structure[idx][p].clone(),
                        structures: pi_structures[idx][p] as usize,
                        idle_seconds: pi_seconds[idx][p],
                    })
                    .collect(),
                first_scout_unit: scout_unit[idx].clone(),
                first_scout_seconds: scout_seconds[idx],
                first_upgrade: first_upgrade[idx].clone(),
                killed_by: counts(&kb_unit[idx], &kb_count[idx]),
                army_at_10: counts(&a10_unit[idx], &a10_count[idx]),
                was_behind: was_behind[idx],
                proxy: proxy[idx],
                base_trade: base_trade[idx],
                rating: rating[idx],
                supply_blocked_seconds: supply_blocked[idx],
                floating_seconds: floating[idx],
                avg_unspent: avg_unspent[idx],
                injects: injects[idx],
                chrono_boosts: chrono[idx],
                mules: mules[idx],
                commands_per_minute: cpm[idx],
                resources_lost: lost[idx],
                resources_killed: killed[idx],
                mining_seconds_lost: mining[idx],
                bases_at_10: bases_at_10[idx],
                first_kill_seconds: first_kill[idx],
                energy: (0..energy_ability[idx].len())
                    .map(|e| AbilityEfficiency {
                        ability: energy_ability[idx][e].clone(),
                        caster: energy_caster[idx][e].clone(),
                        uses: energy_uses[idx][e],
                        caster_minutes: energy_minutes[idx][e],
                        possible: energy_possible[idx][e],
                        ..Default::default()
                    })
                    .collect(),
            };
            ((ids[idx], names[idx].clone()), insight)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_writes_and_reads_the_insights() {
        let insight = PlayerGameInsight {
            won: true,
            matchup: "TvZ".to_string(),
            opening: vec!["Barracks".to_string(), "Factory".to_string()],
            benchmarks: vec![Benchmark {
                minute: 4,
                workers: 30,
                supply: 40,
                minerals_rate: 900,
                vespene_rate: 200,
                army_value: 500,
            }],
            killed_by: vec![UnitCount {
                unit: "Baneling".to_string(),
                count: 12,
            }],
            rating: Some(5200),
            energy: vec![AbilityEfficiency {
                ability: "Inject Larva".to_string(),
                caster: "Queen".to_string(),
                uses: 20,
                caster_minutes: 15.5,
                possible: 30.25,
                ..Default::default()
            }],
            first_kill_seconds: Some(250),
            injects: 3.,
            ..Default::default()
        };
        let rows = vec![
            (7, "Clem".to_string(), insight.clone()),
            (7, "Serral".to_string(), PlayerGameInsight::default()),
        ];
        let frame = insights_frame(&rows).unwrap();
        let read = decode_insights(&frame).unwrap();
        assert_eq!(read.get(&(7, "Clem".to_string())), Some(&insight));
        assert_eq!(
            read.get(&(7, "Serral".to_string())),
            Some(&PlayerGameInsight::default())
        );
    }
}

/// The average share of the possible injects, chrono boosts and MULEs until 10:00 in the games
/// of each patch (the closest balance version): `(sum of uses / possible, games)` by patch and
/// ability, from the rows of the days computed by this version.
pub type EnergyAverages = HashMap<(u32, String), (f32, usize)>;

fn compute_energy_averages(ipc_path: &str) -> Result<EnergyAverages, SwarmyError> {
    let scans = list_partitions(Path::new(ipc_path))?
        .into_iter()
        .filter(|day| {
            day_source(day).is_some_and(|source| {
                std::fs::read_to_string(day.join(INSIGHTS_SOURCE_FILE))
                    .ok()
                    .as_deref()
                    == Some(source.as_str())
            })
        })
        .map(|day| {
            LazyFrame::scan_ipc(
                PlRefPath::new(day.join(INSIGHTS_FILE).to_string_lossy().as_ref()),
                Default::default(),
                UnifiedScanArgs {
                    hive_options: HiveOptions::new_disabled(),
                    ..Default::default()
                },
            )
            .map(|scan| {
                scan.select([
                    col("ext_fs_id"),
                    col("energy_ability"),
                    col("energy_uses"),
                    col("energy_possible"),
                ])
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if scans.is_empty() {
        return Ok(HashMap::new());
    }
    let versions = crate::data::scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .select([col("ext_fs_id"), col("ext_protocol_version")])
        .unique(Some(cols(["ext_fs_id"])), UniqueKeepStrategy::Any);
    let rows = concat(scans, UnionArgs::default())?
        .explode(
            cols(["energy_ability", "energy_uses", "energy_possible"]),
            // The games without casters have empty lists, they have no row.
            ExplodeOptions {
                empty_as_null: false,
                keep_nulls: false,
            },
        )
        .filter(col("energy_possible").gt(lit(0f32)))
        .join(
            versions,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("ext_protocol_version"), col("energy_ability")])
        .agg([
            (col("energy_uses").cast(DataType::Float32) / col("energy_possible"))
                .sum()
                .alias("shares"),
            len().alias("games"),
        ])
        .collect()?;
    let mut averages: EnergyAverages = HashMap::new();
    let versions = crate::data::u32_values(&rows, "ext_protocol_version")?;
    let abilities = crate::data::str_values(&rows, "energy_ability")?;
    let shares = crate::data::f32_values(&rows, "shares")?;
    let games = crate::data::u32_values(&rows, "games")?;
    for idx in 0..rows.height() {
        let Ok(patch) = crate::replay_rerun::decisions::closest_balance_version(versions[idx])
        else {
            continue;
        };
        let entry = averages.entry((patch, abilities[idx].clone())).or_default();
        entry.0 += shares[idx];
        entry.1 += games[idx] as usize;
    }
    Ok(averages)
}

/// [`compute_energy_averages`], once per snapshot generation.
pub fn energy_averages_cached(
    ipc_path: &str,
) -> Result<std::sync::Arc<EnergyAverages>, SwarmyError> {
    type Key = (String, Option<std::time::SystemTime>);
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<HashMap<Key, std::sync::Arc<EnergyAverages>>>,
    > = std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (
        ipc_path.to_string(),
        crate::data::snapshot_generation(ipc_path),
    );
    if let Some(averages) = cache.lock().ok().and_then(|cache| cache.get(&key).cloned()) {
        return Ok(averages);
    }
    let averages = std::sync::Arc::new(compute_energy_averages(ipc_path)?);
    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, averages.clone());
    }
    Ok(averages)
}

/// Adds the patch averages to the energy used by the players of the replay.
pub fn annotate_energy(detail: &mut ReplayDetail, averages: &EnergyAverages) {
    let Ok(patch) =
        crate::replay_rerun::decisions::closest_balance_version(detail.protocol_version)
    else {
        return;
    };
    for efficiency in detail
        .players
        .iter_mut()
        .flat_map(|summary| summary.ability_efficiency.iter_mut())
    {
        if let Some((shares, games)) = averages.get(&(patch, efficiency.ability.clone())) {
            efficiency.patch_share = (*games > 0).then(|| shares / *games as f32);
            efficiency.patch_games = *games;
        }
    }
}
