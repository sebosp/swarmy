//! Player: the games of a player across the snapshot, their results, opponents, units and
//! accounts, and the insights computed by Optimize. Each statistic is a query file of this
//! directory (AGENTS.md, "A query is a file"); this file runs them and puts the answer together.
//! `head_to_head` (two players side by side) and `insights` read the analysis of the games.

pub mod accounts;
pub mod benchmarks;
pub mod game_insights;
pub mod game_results;
pub mod head_to_head;
pub mod insights;
pub mod matches;
pub mod opponents;
pub mod player_rows;
pub mod players;
pub mod similar_accounts;
pub mod snapshot_insights;
pub mod unit_effectiveness;

use crate::data::{IpcSnapshot, QueryMeta, resolve_player};
use crate::host::SwarmyHost;
use crate::replay_rerun::decisions::latest_balance_units;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use head_to_head::try_query_head_to_head;
use std::collections::HashMap;
use swarmy_common::*;

/// The queries of the view, for the debug links of the panels.
pub const QUERIES: [QueryMeta; 8] = [
    player_rows::META,
    benchmarks::META,
    game_results::META,
    matches::META,
    opponents::META,
    unit_effectiveness::META,
    accounts::META,
    similar_accounts::META,
];

/// The players change their clan tags, they are identified by the name without it.
pub fn name_without_clan(player_name: &str) -> &str {
    player_name.rsplit("<sp/>").next().unwrap_or(player_name)
}

pub async fn query_player_detail(
    host: &dyn SwarmyHost,
    player_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let groups = crate::personal_db::read_player_groups(host);
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(try_query_player_detail(
                &app_config.replay_path,
                &player_name,
                &groups,
            ))
    })
    .await
}

/// The games of a player: of its accounts (`groups`, by account, from the personal database), or
/// of the account of the name; `player_name` is a player or a name one of its accounts played as.
pub fn try_query_player_detail(
    replay_path: &str,
    player_name: &str,
    groups: &HashMap<String, String>,
) -> Result<PlayerDetail, SwarmyError> {
    let source = IpcSnapshot::open(replay_path)?;
    let rows = player_rows::plan(&source, groups)?;
    let player = resolve_player(&rows, name_without_clan(player_name))?;
    let results = game_results::shape(game_results::plan(&source, groups, &player)?.collect()?)?;
    let matches = matches::shape(matches::plan(&source, groups, &player)?.collect()?)?;
    let opponents = opponents::shape(opponents::plan(&source, groups, &player)?.collect()?)?;
    let units = unit_effectiveness::shape(
        unit_effectiveness::plan(&source, groups, &player)?.collect()?,
        latest_balance_units()?,
    )?;
    let accounts = accounts::shape(
        accounts::plan(&source, groups, &player)?.collect()?,
        accounts::ACCOUNTS_SHOWN,
    )?;
    let similar_accounts =
        similar_accounts::shape(similar_accounts::plan(&source, groups, &player)?.collect()?)?;
    let insights = insights::player_insights(source.ipc_path(), &player, &matches)?;
    Ok(PlayerDetail {
        player_name: player,
        games: results.games,
        wins: results.wins,
        ties: results.ties,
        races: results.races,
        by_month: results.by_month,
        by_version: results.by_version,
        units,
        opponents,
        matches,
        insights,
        accounts,
        similar_accounts,
    })
}

/// The players of the snapshot, for the player fields.
pub async fn list_players(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let groups = crate::personal_db::read_player_groups(host);
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources([QuerySource::from(players::META)])
            .process_result(try_list_players(&app_config.replay_path, &groups))
    })
    .await
}

pub fn try_list_players(
    replay_path: &str,
    groups: &HashMap<String, String>,
) -> Result<Vec<GamesWins>, SwarmyError> {
    let source = IpcSnapshot::open(replay_path)?;
    players::shape(players::plan(&source, groups)?.collect()?)
}

pub async fn query_head_to_head(
    host: &dyn SwarmyHost,
    player_name: String,
    opponent_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let groups = crate::personal_db::read_player_groups(host);
    let query = HeadToHeadQuery {
        player_name,
        opponent_name,
    };
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_head_to_head(
            app_config.replay_path,
            query,
            groups,
        ))
    })
    .await
}

/// The games the tests of the queries of this view share, the ones of their READMEs.
#[cfg(test)]
pub mod test_games {
    use crate::data::fixtures::{Game, Player, Snapshot, Unit};

    /// Alice (account 2-1-1001, once with a clan tag, and 2-1-777 as AliceEU grouped under her),
    /// Bob, Carol, Dave, a computer and Alice2 (a name like hers, another account):
    ///
    /// | Game | Day | Map | Version | Players |
    /// |---|---|---|---|---|
    /// | 1 | 2026-01-15 | Pylon LE | 95248 | `&lt;TL&gt;<sp/>Alice` (Terran) beats Bob (Zerg) |
    /// | 2 | 2026-02-03 | Ephemeron LE | 95248 | Carol (Protoss) beats Alice (Terran) |
    /// | 3 | 2026-02-10 | Pylon LE | 96000 | AliceEU (Terran) ties Dave (Zerg) |
    /// | 4 | 2026-02-15 | Pylon LE | 96000 | Bob (Zerg) beats A.I. 1 (Very Easy) |
    /// | 5 | 2026-02-20 | Pylon LE | 95248 | Alice (Zerg) and Bob (Zerg) beat Carol (Protoss) and Dave (Zerg) |
    /// | 6 | 2026-03-01 | Pylon LE | 96000 | Bob (Zerg) beats Alice2 (Terran) |
    pub fn alice_and_friends() -> Snapshot {
        let alice = |name: &'static str, race: &'static str, result: &'static str| Player {
            account: 1001,
            result,
            ..Player::won(name, race)
        };
        Snapshot::new()
            .game(Game {
                id: 1,
                played_at: "2026-01-15 20:30:00",
                players: vec![
                    Player {
                        units: vec![Unit::new("Marine", 20, 15, 750)],
                        ..alice("&lt;TL&gt;<sp/>Alice", "Terran", "Win")
                    },
                    Player::lost("Bob", "Zerg"),
                ],
                ..Default::default()
            })
            .game(Game {
                id: 2,
                map: "Ephemeron LE",
                played_at: "2026-02-03 20:30:00",
                players: vec![
                    Player::won("Carol", "Protoss"),
                    Player {
                        units: vec![
                            Unit::new("Marine", 10, 5, 250),
                            Unit::new("Marauder", 4, 6, 600),
                        ],
                        ..alice("Alice", "Terran", "Loss")
                    },
                ],
                ..Default::default()
            })
            .game(Game {
                id: 3,
                played_at: "2026-02-10 20:30:00",
                protocol_version: 96_000,
                players: vec![
                    Player {
                        account: 777,
                        result: "Tie",
                        ..Player::won("AliceEU", "Terran")
                    },
                    Player {
                        result: "Tie",
                        ..Player::lost("Dave", "Zerg")
                    },
                ],
                ..Default::default()
            })
            .game(Game {
                id: 4,
                played_at: "2026-02-15 20:30:00",
                protocol_version: 96_000,
                players: vec![
                    Player::won("Bob", "Zerg"),
                    Player {
                        computer: true,
                        ..Player::lost("A.I. 1 (Very Easy)", "Protoss")
                    },
                ],
                ..Default::default()
            })
            .game(Game {
                id: 5,
                played_at: "2026-02-20 20:30:00",
                players: vec![
                    Player {
                        team: 1,
                        units: vec![Unit::new("Zergling", 30, 10, 500)],
                        ..alice("Alice", "Zerg", "Win")
                    },
                    Player {
                        team: 1,
                        ..Player::won("Bob", "Zerg")
                    },
                    Player {
                        team: 2,
                        ..Player::lost("Carol", "Protoss")
                    },
                    Player {
                        team: 2,
                        ..Player::lost("Dave", "Zerg")
                    },
                ],
                ..Default::default()
            })
            .game(Game {
                id: 6,
                played_at: "2026-03-01 20:30:00",
                protocol_version: 96_000,
                players: vec![Player::won("Bob", "Zerg"), Player::lost("Alice2", "Terran")],
                ..Default::default()
            })
            .group("2-1-777", "Alice")
    }

    /// `alice_and_friends` with the economy at the benchmark minutes of the 1v1 games 1, 3, 4 and
    /// 6 (`data::fixtures::Minute`: minute, workers, supply, (minerals, vespene) per minute, army
    /// value); game 2 has none and game 5 is a 2v2.
    ///
    /// | Game | Player | Minute 4 | Minute 6 |
    /// |---|---|---|---|
    /// | 1 | Alice (T) | 20 workers, 28 supply, 500 + 300, 400 army | 30, 44, 700 + 400, 900 |
    /// | 1 | Bob (Z) | 22, 30, 600 + 200, 200 | 34, 50, 900 + 300, 600 |
    /// | 3 | AliceEU (T) | 18, 26, 450 + 250, 350 | |
    /// | 3 | Dave (Z) | 24, 32, 650 + 150, 100 | |
    /// | 4 | Bob (Z) | 20, 30, 600 + 200, 300 | |
    /// | 4 | A.I. (P) | 12, 18, 300 + 0, 0 | |
    /// | 6 | Alice2 (T) | 16, 24, 400 + 200, 300 | 26, 40, 600 + 300, 800 |
    /// | 6 | Bob (Z) | 20, 28, 550 + 250, 250 | 32, 46, 850 + 350, 500 |
    pub fn with_minutes() -> Snapshot {
        use crate::data::fixtures::Minute;
        let mut snapshot = alice_and_friends();
        let minutes = |game: u64, name: &str| -> Vec<Minute> {
            match (game, name) {
                (1, "&lt;TL&gt;<sp/>Alice") => vec![
                    Minute::new(4, 20, 28, (500, 300), 400),
                    Minute::new(6, 30, 44, (700, 400), 900),
                ],
                (1, "Bob") => vec![
                    Minute::new(4, 22, 30, (600, 200), 200),
                    Minute::new(6, 34, 50, (900, 300), 600),
                ],
                (3, "AliceEU") => vec![Minute::new(4, 18, 26, (450, 250), 350)],
                (3, "Dave") => vec![Minute::new(4, 24, 32, (650, 150), 100)],
                (4, "Bob") => vec![Minute::new(4, 20, 30, (600, 200), 300)],
                (4, "A.I. 1 (Very Easy)") => vec![Minute::new(4, 12, 18, (300, 0), 0)],
                (6, "Alice2") => vec![
                    Minute::new(4, 16, 24, (400, 200), 300),
                    Minute::new(6, 26, 40, (600, 300), 800),
                ],
                (6, "Bob") => vec![
                    Minute::new(4, 20, 28, (550, 250), 250),
                    Minute::new(6, 32, 46, (850, 350), 500),
                ],
                _ => vec![],
            }
        };
        for game in &mut snapshot.games {
            for player in &mut game.players {
                player.minutes = minutes(game.id, player.name);
            }
        }
        snapshot
    }
}
