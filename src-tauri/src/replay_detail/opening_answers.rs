//! What wins against an opening: in the 1v1 games of the snapshot, the openings of a race (its
//! first three tech structures, as the opening of the build order) and how often they won against
//! each opening of the enemy race, so that a replay shows the answers that won the most against
//! the opening of the opponent. The answers are the query file `answers.rs` (AGENTS.md, "A query
//! is a file"), computed once per snapshot generation.

pub mod answers;

use crate::data::{IpcSnapshot, QueryMeta, snapshot_generation};
use polars::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

pub use answers::{AnswersByOpening, MIN_ANSWER_GAMES, OPENING_SEPARATOR};

/// The answers shown, the ones that won the most first.
pub const MAX_ANSWERS: usize = 5;

/// The queries of the answers, for the debug links.
pub const QUERIES: [QueryMeta; 1] = [answers::META];

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

pub fn opening_answers(ipc_path: &str) -> Result<AnswersByOpening, SwarmyError> {
    let source = IpcSnapshot::at(ipc_path);
    // The structures of every game of the snapshot: streamed, not all in memory at once.
    let frame = answers::plan(&source, answers::tech_structures()?)?
        .collect_with_engine(Engine::Streaming)?
        .unwrap_single();
    answers::shape(frame)
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
