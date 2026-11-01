//! What wins against an opening: in the 1v1 games of the snapshot, the openings of a race (its
//! first three tech structures, as the opening of the build order) and how often they won against
//! each opening of the enemy race, so that a replay shows the answers that won the most against
//! the opening of the opponent.

use super::data::{is_tech_structure, OPENING_STRUCTURES};
use crate::data::*;
use crate::replay_rerun::decisions::latest_balance_units;
use polars::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

/// The answers with fewer games are not shown.
pub const MIN_ANSWER_GAMES: u32 = 5;

/// The answers shown, the ones that won the most first.
pub const MAX_ANSWERS: usize = 5;

/// The openings are joined with this, as in Player Detail.
pub const OPENING_SEPARATOR: &str = " → ";

/// By race, enemy race and enemy opening: the openings of the race with their games and wins.
pub type AnswersByOpening = HashMap<(String, String, String), Vec<OpeningAnswer>>;

type CacheKey = (String, Option<std::time::SystemTime>);

fn cache() -> &'static Mutex<HashMap<CacheKey, Arc<AnswersByOpening>>> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Arc<AnswersByOpening>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

pub fn opening_answers_cached(ipc_path: &str) -> Result<Arc<AnswersByOpening>, SwarmyError> {
    let key = (ipc_path.to_string(), snapshot_generation(ipc_path));
    if let Some(answers) = cache().lock().unwrap().get(&key) {
        return Ok(answers.clone());
    }
    let answers = Arc::new(opening_answers(ipc_path)?);
    cache().lock().unwrap().insert(key, answers.clone());
    Ok(answers)
}

/// The tech structures of every race, see [`is_tech_structure`].
fn tech_structures() -> Result<Vec<String>, SwarmyError> {
    let balance = latest_balance_units()?;
    Ok(balance
        .keys()
        .filter(|name| is_tech_structure(name, balance))
        .cloned()
        .collect())
}

pub fn opening_answers(ipc_path: &str) -> Result<AnswersByOpening, SwarmyError> {
    let players = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .with_columns([
            (int_range(lit(0), len(), 1, DataType::UInt32) + lit(1u32))
                .over([col("ext_fs_id")])?
                .cast(DataType::UInt8)
                .alias("player_id"),
            race_name_expr("player_race").alias("race"),
        ])
        .with_column(len().over([col("ext_fs_id")])?.alias("players"))
        .filter(col("players").eq(lit(2u32)))
        .select([
            col("ext_fs_id"),
            col("player_id"),
            col("race"),
            col("player_result"),
        ]);
    let openings = scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
        .filter(
            col("ext_replay_loop")
                .gt(lit(0))
                .and(is_one_of("unit_type_name", &tech_structures()?)),
        )
        .select([
            col("ext_fs_id"),
            col("control_player_id").alias("player_id"),
            col("unit_type_name"),
            col("ext_replay_loop"),
        ])
        .sort(["ext_replay_loop"], Default::default())
        .unique_stable(
            Some(cols(["ext_fs_id", "player_id", "unit_type_name"])),
            UniqueKeepStrategy::First,
        )
        .group_by_stable([col("ext_fs_id"), col("player_id")])
        .agg([
            col("unit_type_name")
                .head(Some(OPENING_STRUCTURES))
                .str()
                .join(OPENING_SEPARATOR, true)
                .alias("opening"),
            len().alias("structures"),
        ])
        // The games that ended before the third tech structure are not compared.
        .filter(col("structures").gt_eq(lit(OPENING_STRUCTURES as u32)))
        .drop(cols(["structures"]));
    let played = players.join(
        openings,
        [col("ext_fs_id"), col("player_id")],
        [col("ext_fs_id"), col("player_id")],
        JoinArgs::new(JoinType::Inner),
    );
    let enemies = played.clone().select([
        col("ext_fs_id"),
        col("player_id").alias("enemy_id"),
        col("race").alias("enemy_race"),
        col("opening").alias("enemy_opening"),
    ]);
    let answers = played
        .join(
            enemies,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("player_id").neq(col("enemy_id")))
        .group_by([
            col("race"),
            col("enemy_race"),
            col("enemy_opening"),
            col("opening"),
        ])
        .agg([
            len().cast(DataType::UInt32).alias("games"),
            col("player_result")
                .eq(lit("Win"))
                .sum()
                .cast(DataType::UInt32)
                .alias("wins"),
        ])
        .filter(col("games").gt_eq(lit(MIN_ANSWER_GAMES)))
        .collect_with_engine(Engine::Streaming)?
        .unwrap_single();
    let mut result = AnswersByOpening::new();
    for (((((race, enemy_race), enemy_opening), opening), games), wins) in
        str_values(&answers, "race")?
            .into_iter()
            .zip(str_values(&answers, "enemy_race")?)
            .zip(str_values(&answers, "enemy_opening")?)
            .zip(str_values(&answers, "opening")?)
            .zip(u32_values(&answers, "games")?)
            .zip(u32_values(&answers, "wins")?)
    {
        result
            .entry((race, enemy_race, enemy_opening))
            .or_default()
            .push(OpeningAnswer {
                opening,
                games,
                wins,
            });
    }
    for answers in result.values_mut() {
        answers.sort_by(|a, b| {
            b.ranking()
                .total_cmp(&a.ranking())
                .then(b.games.cmp(&a.games))
        });
    }
    Ok(result)
}

/// Sets the answers to the opening of the enemy of each player of a 1v1, and the record of the
/// player's own opening against it.
pub fn annotate(detail: &mut ReplayDetail, answers: &AnswersByOpening) {
    if detail.players.len() != 2 {
        return;
    }
    let enemies: Vec<(String, String)> = detail
        .players
        .iter()
        .map(|summary| {
            (
                summary.player.race.clone(),
                summary.opening.join(OPENING_SEPARATOR),
            )
        })
        .collect();
    for (idx, summary) in detail.players.iter_mut().enumerate() {
        let (enemy_race, enemy_opening) = &enemies[1 - idx];
        if enemy_opening.is_empty() {
            continue;
        }
        let key = (
            summary.player.race.clone(),
            enemy_race.clone(),
            enemy_opening.clone(),
        );
        let Some(found) = answers.get(&key) else {
            continue;
        };
        let own = summary.opening.join(OPENING_SEPARATOR);
        summary.opening_answers = OpeningAnswers {
            enemy_opening: enemy_opening.clone(),
            answers: found.iter().take(MAX_ANSWERS).cloned().collect(),
            own: found.iter().find(|answer| answer.opening == own).cloned(),
        };
    }
}

#[cfg(test)]
mod snapshot_tests {
    /// `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test opening_answers_of -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn opening_answers_of_a_snapshot() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let ipc_path = format!("{}/{}", replay_path, swarmy_common::IPC_DIR);
        let start = std::time::Instant::now();
        let answers = super::opening_answers(&ipc_path).unwrap();
        eprintln!("{:?} {} enemy openings", start.elapsed(), answers.len());
        let mut keys: Vec<_> = answers.iter().collect();
        keys.sort_by_key(|(_, answers)| {
            std::cmp::Reverse(answers.iter().map(|a| a.games).sum::<u32>())
        });
        for (key, answers) in keys.iter().take(4) {
            eprintln!("{key:?} {:?}", &answers[..answers.len().min(3)]);
        }
        assert!(!answers.is_empty());
    }
}
