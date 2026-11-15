//! Progress: the workers at 06:00, the income at 08:00, the seconds supply blocked and the army at
//! 10:00 of each of the last games of a player, and their goals. Each statistic is a query file
//! of this directory (AGENTS.md, "A query is a file"); this file runs them and puts the answer
//! together.

pub mod games;
pub mod metrics;

use crate::data::{IpcSnapshot, QueryMeta};
use crate::host::SwarmyHost;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use std::collections::BTreeMap;
use swarmy_common::*;

/// The queries of the view, for the debug links of the panels.
pub const QUERIES: [QueryMeta; 2] = [games::META, metrics::META];

const GOALS_KEY: &str = "progress_goals";

/// The name the view follows from a player name as stored in a replay: without the clan tag,
/// as the Main Player Name of Config is typed.
pub fn name_of(player_name: &str) -> &str {
    crate::player_detail::name_without_clan(player_name)
}

/// The last games of the player with their metrics, and the goals.
pub fn try_query_progress(
    replay_path: &str,
    player: &str,
    goals: BTreeMap<String, f64>,
) -> Result<ProgressReport, SwarmyError> {
    let mut report = ProgressReport {
        player: player.to_string(),
        goals,
        ..Default::default()
    };
    if player.is_empty() {
        return Ok(report);
    }
    let source = IpcSnapshot::open(replay_path)?;
    report.games = games::shape(games::plan(&source, player)?.collect()?)?;
    let mut values = metrics::shape(metrics::plan(&source, player)?.collect()?)?;
    for game in &mut report.games {
        if let Some(values) = values.remove(&game.ext_fs_id) {
            game.values = values;
        }
    }
    Ok(report)
}

fn read_goals(host: &dyn SwarmyHost) -> Result<BTreeMap<String, f64>, SwarmyError> {
    Ok(host
        .read_store()?
        .remove(GOALS_KEY)
        .and_then(|goals| serde_json::from_value(goals).ok())
        .unwrap_or_default())
}

/// The progress of `player_name`, the Main Player Name of the Config of the app that asks: with a
/// remote backend, the Config of the backend does not have it. Empty for the backend's own.
pub async fn query_progress(
    host: &dyn SwarmyHost,
    player_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let goals = read_goals(host)?;
    let player_name = if player_name.trim().is_empty() {
        app_config.main_player_name
    } else {
        player_name.trim().to_string()
    };
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(try_query_progress(
                &app_config.replay_path,
                &player_name,
                goals,
            ))
    })
    .await
}

/// Saves the goals of the progress, by metric key, a goal of 0 is removed.
pub async fn set_progress_goals(
    host: &dyn SwarmyHost,
    goals: BTreeMap<String, f64>,
) -> Result<ApiResponse, SwarmyError> {
    let goals: BTreeMap<String, f64> = goals.into_iter().filter(|(_, goal)| *goal > 0.).collect();
    host.write_store(GOALS_KEY, serde_json::to_value(&goals)?)?;
    Ok(ApiResponseBuilder::new()
        .with_success()
        .with_message("Goals saved".to_string())
        .build())
}

/// The games the tests of the queries of this view share, the ones of their READMEs.
#[cfg(test)]
pub mod test_games {
    use crate::data::fixtures::{Game, Player, Snapshot, Stat};

    /// Alice's three games on Pylon LE: against Bob (won, with stats), against the computer
    /// (won, no stats), and with Bob against Carol and Dave (lost, no stats).
    pub fn alice_progress() -> Snapshot {
        Snapshot::new()
            .game(Game {
                id: 1,
                played_at: "2026-01-15 20:30:00",
                players: vec![
                    Player {
                        stats: vec![
                            Stat {
                                game_loop: 5000,
                                food_made: 30,
                                food_used: 30,
                                workers: 25,
                                ..Default::default()
                            },
                            Stat {
                                game_loop: 7900,
                                food_made: 44,
                                food_used: 46,
                                workers: 40,
                                ..Default::default()
                            },
                            Stat {
                                game_loop: 8000,
                                food_made: 60,
                                food_used: 48,
                                workers: 42,
                                ..Default::default()
                            },
                            Stat {
                                game_loop: 10500,
                                food_made: 90,
                                food_used: 70,
                                minerals_rate: 1200,
                                vespene_rate: 300,
                                ..Default::default()
                            },
                            Stat {
                                game_loop: 13200,
                                food_made: 120,
                                food_used: 100,
                                army_minerals: 1500,
                                army_vespene: 400,
                                ..Default::default()
                            },
                        ],
                        ..Player::won("Alice", "Terran")
                    },
                    Player::lost("Bob", "Zerg"),
                ],
                ..Default::default()
            })
            .game(Game {
                id: 2,
                played_at: "2026-01-20 20:30:00",
                players: vec![
                    Player::won("Alice", "Terran"),
                    Player {
                        computer: true,
                        ..Player::lost("A.I. 1 (Very Easy)", "Protoss")
                    },
                ],
                ..Default::default()
            })
            .game(Game {
                id: 3,
                played_at: "2026-02-01 20:30:00",
                players: vec![
                    Player {
                        team: 1,
                        ..Player::lost("Alice", "Zerg")
                    },
                    Player {
                        team: 1,
                        ..Player::lost("Bob", "Zerg")
                    },
                    Player {
                        team: 2,
                        ..Player::won("Carol", "Protoss")
                    },
                    Player {
                        team: 2,
                        ..Player::won("Dave", "Zerg")
                    },
                ],
                ..Default::default()
            })
    }
}
