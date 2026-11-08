//! The creep of the 1v1 between a Zerg and a Terran or a Protoss: the creep tumors the Zerg placed
//! (the Queens and the tumors spread them) and the ones the opponent killed (scans, detectors,
//! Oracle revelations, Observers, then the army), compared with the games of the matchup.
//!
//! A tumor that lands is a `CreepTumorBurrowed` born in the tracker events, the tumors killed are
//! its deaths with the opponent as the killer.

use crate::data::*;
use polars::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

/// The unit of a creep tumor that spreads creep.
const CREEP_TUMOR: &str = "CreepTumorBurrowed";

/// The minutes of the timeline and of the averages.
const MAX_MINUTES: usize = 30;

/// The creep of a game, per minute (not cumulative).
#[derive(Debug, Clone, Default)]
struct CreepGame {
    zerg: String,
    opponent: String,
    matchup: String,
    seconds: u32,
    placed: Vec<u32>,
    cleaned: Vec<u32>,
}

impl CreepGame {
    /// The full minutes the game lasted.
    fn full_minutes(&self) -> usize {
        (self.seconds / 60) as usize
    }

    fn cumulative(values: &[u32], minute: usize) -> u32 {
        values.iter().take(minute).sum()
    }
}

/// The averages of the games of a matchup.
#[derive(Debug, Clone, Default)]
struct MatchupAverages {
    games: u32,
    placed_per_minute: f32,
    cleaned_per_minute: f32,
    /// By the end of each minute, of the games that lasted that long.
    placed: Vec<f32>,
    cleaned: Vec<f32>,
}

#[derive(Debug, Default)]
pub struct CreepStats {
    games: HashMap<u64, CreepGame>,
    averages: HashMap<String, MatchupAverages>,
}

impl CreepStats {
    /// The creep of a game against the averages of its matchup.
    pub fn summary(&self, ext_fs_id: u64) -> Option<CreepSummary> {
        let game = self.games.get(&ext_fs_id)?;
        let averages = self.averages.get(&game.matchup)?;
        let minutes = game.full_minutes().min(MAX_MINUTES);
        Some(CreepSummary {
            zerg: game.zerg.clone(),
            opponent: game.opponent.clone(),
            matchup: game.matchup.clone(),
            game_seconds: game.seconds,
            placed: game.placed.iter().sum(),
            cleaned: game.cleaned.iter().sum(),
            avg_placed_per_minute: averages.placed_per_minute,
            avg_cleaned_per_minute: averages.cleaned_per_minute,
            games: averages.games,
            timeline: (1..=minutes)
                .map(|minute| CreepMinute {
                    minute: minute as u32,
                    placed: CreepGame::cumulative(&game.placed, minute),
                    cleaned: CreepGame::cumulative(&game.cleaned, minute),
                    avg_placed: averages.placed.get(minute - 1).copied().unwrap_or_default(),
                    avg_cleaned: averages
                        .cleaned
                        .get(minute - 1)
                        .copied()
                        .unwrap_or_default(),
                })
                .collect(),
        })
    }
}

/// The averages of the games of a matchup.
fn averages_of(games: &[&CreepGame]) -> MatchupAverages {
    let rate = |values: &dyn Fn(&CreepGame) -> u32| {
        let rates: Vec<f32> = games
            .iter()
            .filter(|game| game.seconds >= 60)
            .map(|game| values(game) as f32 / (game.seconds as f32 / 60.))
            .collect();
        rates.iter().sum::<f32>() / rates.len().max(1) as f32
    };
    let by_minute = |values: &dyn Fn(&CreepGame) -> &Vec<u32>| {
        (1..=MAX_MINUTES)
            .map(|minute| {
                let lasted: Vec<u32> = games
                    .iter()
                    .filter(|game| game.full_minutes() >= minute)
                    .map(|game| CreepGame::cumulative(values(game), minute))
                    .collect();
                lasted.iter().sum::<u32>() as f32 / lasted.len().max(1) as f32
            })
            .collect()
    };
    MatchupAverages {
        games: games.len() as u32,
        placed_per_minute: rate(&|game| game.placed.iter().sum()),
        cleaned_per_minute: rate(&|game| game.cleaned.iter().sum()),
        placed: by_minute(&|game| &game.placed),
        cleaned: by_minute(&|game| &game.cleaned),
    }
}

/// The count per game, player (or killer id) and minute of the tumors of a frame.
fn counts_by_minute(
    frame: LazyFrame,
    who: &str,
) -> Result<HashMap<(u64, String), Vec<u32>>, SwarmyError> {
    let counts = frame
        .with_column((game_seconds() / lit(60u32)).alias("minute"))
        .group_by([col("ext_fs_id"), col(who), col("minute")])
        .agg([len().cast(DataType::UInt32).alias("tumors")])
        .collect()?;
    let ids = u64_values(&counts, "ext_fs_id")?;
    let who_values = str_values(&counts, who)?;
    let minutes = u32_values(&counts, "minute")?;
    let tumors = u32_values(&counts, "tumors")?;
    let mut out: HashMap<(u64, String), Vec<u32>> = HashMap::new();
    for idx in 0..counts.height() {
        let minute = minutes[idx] as usize;
        let values = out.entry((ids[idx], who_values[idx].clone())).or_default();
        if values.len() <= minute {
            values.resize(minute + 1, 0);
        }
        values[minute] += tumors[idx];
    }
    Ok(out)
}

pub fn compute_creep_stats(ipc_path: &str) -> Result<CreepStats, SwarmyError> {
    // The 1v1 between a Zerg and a Terran or a Protoss.
    let players = snapshot_players(ipc_path)?
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("player_id").cast(DataType::String).alias("player_id"),
            col("race"),
        ])
        .collect()?;
    let ids = u64_values(&players, "ext_fs_id")?;
    let names = str_values(&players, "player_name")?;
    let player_ids = str_values(&players, "player_id")?;
    let races = str_values(&players, "race")?;
    let mut by_game: HashMap<u64, Vec<usize>> = HashMap::new();
    for (idx, id) in ids.iter().enumerate() {
        by_game.entry(*id).or_default().push(idx);
    }
    let durations = game_durations(ipc_path)?.collect()?;
    let durations: HashMap<u64, u32> = u64_values(&durations, "ext_fs_id")?
        .into_iter()
        .zip(u32_values(&durations, "game_seconds")?)
        .collect();
    let placed = counts_by_minute(
        scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
            .filter(col("unit_type_name").eq(lit(CREEP_TUMOR)))
            .select([col("ext_fs_id"), col("player_name"), col("ext_replay_loop")]),
        "player_name",
    )?;
    let cleaned = counts_by_minute(
        scan_snapshot_ipc(ipc_path, UNIT_DIED_IPC)?
            .filter(
                col("unit_died_name")
                    .eq(lit(CREEP_TUMOR))
                    .and(col("killer_player_id").is_not_null()),
            )
            .select([
                col("ext_fs_id"),
                col("killer_player_id")
                    .cast(DataType::String)
                    .alias("killer"),
                col("ext_replay_loop"),
            ]),
        "killer",
    )?;
    let mut games = HashMap::new();
    for (ext_fs_id, rows) in by_game {
        let [first, second] = rows.as_slice() else {
            continue;
        };
        let (zerg, opponent) = match (races[*first].as_str(), races[*second].as_str()) {
            ("Zerg", "Terran" | "Protoss") => (*first, *second),
            ("Terran" | "Protoss", "Zerg") => (*second, *first),
            _ => continue,
        };
        let Some(seconds) = durations.get(&ext_fs_id).copied() else {
            continue;
        };
        games.insert(
            ext_fs_id,
            CreepGame {
                zerg: names[zerg].clone(),
                opponent: names[opponent].clone(),
                matchup: matchup_label("Zerg", &races[opponent]),
                seconds,
                placed: placed
                    .get(&(ext_fs_id, names[zerg].clone()))
                    .cloned()
                    .unwrap_or_default(),
                cleaned: cleaned
                    .get(&(ext_fs_id, player_ids[opponent].clone()))
                    .cloned()
                    .unwrap_or_default(),
            },
        );
    }
    let mut by_matchup: HashMap<String, Vec<&CreepGame>> = HashMap::new();
    for game in games.values() {
        by_matchup
            .entry(game.matchup.clone())
            .or_default()
            .push(game);
    }
    let averages = by_matchup
        .into_iter()
        .map(|(matchup, games)| (matchup, averages_of(&games)))
        .collect();
    Ok(CreepStats { games, averages })
}

type CacheKey = (String, Option<std::time::SystemTime>);

/// The creep of the games of the snapshot, computed once per snapshot.
pub fn creep_stats_cached(ipc_path: &str) -> Result<Arc<CreepStats>, SwarmyError> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Arc<CreepStats>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (ipc_path.to_string(), snapshot_generation(ipc_path));
    if let Some(stats) = cache.lock().ok().and_then(|cache| cache.get(&key).cloned()) {
        return Ok(stats);
    }
    let stats = Arc::new(compute_creep_stats(ipc_path)?);
    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, stats.clone());
    }
    Ok(stats)
}

/// The creep of the replay, a 1v1 of a Zerg against a Terran or a Protoss.
pub fn annotate(detail: &mut ReplayDetail, stats: &CreepStats) {
    detail.creep = stats.summary(detail.ext_fs_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(matchup: &str, seconds: u32, placed: Vec<u32>, cleaned: Vec<u32>) -> CreepGame {
        CreepGame {
            zerg: "Z".to_string(),
            opponent: "T".to_string(),
            matchup: matchup.to_string(),
            seconds,
            placed,
            cleaned,
        }
    }

    #[test]
    fn it_compares_the_creep_with_the_matchup() {
        let mut games = HashMap::new();
        // 4 minutes: 2, 4, 6 then 8 tumors, 1 cleaned in the third minute.
        games.insert(1, game("ZvT", 240, vec![0, 2, 2, 2, 2], vec![0, 0, 1]));
        // 2 minutes: 4 then 4 tumors.
        games.insert(2, game("ZvT", 120, vec![2, 2], vec![]));
        let averages = averages_of(&games.values().collect::<Vec<_>>());
        // 8/4 and 4/2 tumors per minute.
        assert_eq!(averages.placed_per_minute, 2.);
        assert_eq!(averages.games, 2);
        // By 02:00 both games, (0 + 2) and (2 + 2); by 03:00 only the first one, 0 + 2 + 2.
        assert_eq!(averages.placed[1], 3.);
        assert_eq!(averages.placed[2], 4.);
        let stats = CreepStats {
            games,
            averages: HashMap::from([("ZvT".to_string(), averages)]),
        };
        let summary = stats.summary(1).unwrap();
        assert_eq!(summary.placed, 8);
        assert_eq!(summary.cleaned, 1);
        assert_eq!(summary.timeline.len(), 4);
        assert_eq!(summary.timeline[2].placed, 4);
        assert_eq!(summary.timeline[2].cleaned, 1);
        assert_eq!(summary.placed_per_minute(), 2.);
        assert!(stats.summary(3).is_none());
    }
}

#[cfg(test)]
mod snapshot_tests {
    /// Runs against a real snapshot: `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test -- --ignored`
    #[test]
    #[ignore]
    fn it_computes_the_creep_of_a_snapshot() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let ipc_path = format!("{}/{}", replay_path, swarmy_common::IPC_DIR);
        let start = std::time::Instant::now();
        let stats = super::compute_creep_stats(&ipc_path).unwrap();
        eprintln!("{:?} {} games", start.elapsed(), stats.games.len());
        for (matchup, averages) in &stats.averages {
            eprintln!(
                "{matchup}: {} games, {:.2} placed and {:.2} cleaned per minute, by 10:00 {:.1} placed {:.1} cleaned",
                averages.games,
                averages.placed_per_minute,
                averages.cleaned_per_minute,
                averages.placed[9],
                averages.cleaned[9],
            );
        }
        let (id, _) = stats.games.iter().next().unwrap();
        eprintln!("{:?}", stats.summary(*id));
        assert!(!stats.averages.is_empty());
    }
}
