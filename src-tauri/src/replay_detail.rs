//! The details of a single replay: build orders, expansions, kills, decisions and messages.

pub mod bases;
pub mod creep;
pub mod data;
pub mod death_density;
pub mod deaths;
pub mod epicenters;
pub mod focus;
pub mod lessons;
pub mod map;
pub mod navigation;
pub mod opening_answers;
pub mod patch_timings;
pub mod report;
pub mod valued_units;
pub mod win_chance;
pub mod winner_benchmarks;
use crate::data::QueryMeta;
use crate::host::SwarmyHost;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::{try_query_replay_detail, try_query_replay_detail_quick};
use swarmy_common::*;

/// The queries of the snapshot a replay is annotated with, for the debug links: the timings of
/// its patch, the winners of its matchup, the answers to its openings, the bases of its players,
/// the epicenters of its engagements and its deaths on the map.
pub const QUERIES: [QueryMeta; 9] = [
    patch_timings::games::META,
    patch_timings::upgrade_averages::META,
    patch_timings::expansion_averages::META,
    winner_benchmarks::medians::META,
    opening_answers::answers::META,
    bases::META,
    epicenters::META,
    deaths::META,
    death_density::META,
];

pub async fn query_replay_detail(
    host: &dyn SwarmyHost,
    ext_fs_id: u64,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(try_query_replay_detail(
                app_config.replay_path,
                &app_config.cache_path,
                ext_fs_id,
            ))
    })
    .await
}

/// The replay without the comparisons with the other games of the snapshot, shown first while
/// [`query_replay_detail`] computes them.
pub async fn query_replay_detail_quick(
    host: &dyn SwarmyHost,
    ext_fs_id: u64,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_replay_detail_quick(
            app_config.replay_path,
            &app_config.cache_path,
            ext_fs_id,
        ))
    })
    .await
}

/// The games the tests of the queries of the annotations share.
#[cfg(test)]
pub mod test_games {
    use crate::data::fixtures::{Born, Death, Game, Minute, Player, Snapshot, Upgrade};
    use swarmy_common::Engagement;

    /// The ids of the games of protocol version 95248 in [`patch_games`].
    pub const PATCH_GAMES: [u64; 6] = [1, 2, 3, 4, 5, 6];

    /// A 1v1 on a 100 by 100 map with a fight in the middle, for the bases and the epicenters.
    /// Alice (Terran) starts at (30, 30), lands a Command Center at (30, 60) at 120 s that lifts
    /// and lands again at 400 s; Bob (Zerg) starts at (70, 70) and finishes a Hatchery at
    /// (70, 40) at 100 s. From 300 s to 310 s Alice loses two Marines at (48, 50) and (52, 50)
    /// and Bob two Zerglings at (50, 48) and (50, 52), a Larva of Bob dies at (70, 70), a Drone
    /// becomes a building and a Zergling a Baneling (`cause` morphed) and Alice cancels a Supply
    /// Depot (`cancelled`), none of them losses; at 450 s an SCV of Alice dies at home and at
    /// 500 s an Overlord of Bob at (10, 90).
    pub fn fight() -> Snapshot {
        Snapshot::new().game(Game {
            id: 1,
            map_size: (100, 100),
            players: vec![
                Player {
                    start: Some((30., 30.)),
                    born: vec![
                        Born::at(120, "CommandCenter", 20, (30., 60.)),
                        Born::at(400, "CommandCenter", 20, (30., 60.)),
                    ],
                    deaths: vec![
                        Death::new(300, 48, 50, "Marine", 1, "killed"),
                        Death::new(305, 52, 50, "Marine", 2, "killed"),
                        Death::new(306, 40, 40, "SupplyDepot", 9, "cancelled"),
                        Death::new(450, 30, 30, "SCV", 3, "killed"),
                    ],
                    ..Player::won("Alice", "Terran")
                },
                Player {
                    start: Some((70., 70.)),
                    born: vec![Born::at(100, "Hatchery", 40, (70., 40.))],
                    deaths: vec![
                        Death::new(302, 50, 48, "Zergling", 4, "killed"),
                        Death::new(310, 50, 52, "Zergling", 5, "killed"),
                        Death::new(308, 60, 60, "Zergling", 10, "morphed"),
                        Death::new(305, 70, 70, "Larva", 6, "killed"),
                        Death::new(305, 70, 70, "Drone", 7, "morphed"),
                        Death::new(500, 10, 90, "Overlord", 8, "killed"),
                    ],
                    ..Player::lost("Bob", "Zerg")
                },
            ],
            ..Default::default()
        })
    }

    /// The engagements the analysis of the replay of [`fight`] gives: the fight of 300 s to
    /// 310 s and the Overlord at 500 s.
    pub fn fight_engagements() -> Vec<Engagement> {
        [(300, 310), (500, 500)]
            .into_iter()
            .map(|(start_seconds, end_seconds)| Engagement {
                start_seconds,
                end_seconds,
                ..Default::default()
            })
            .collect()
    }

    /// Seven 1v1 games on Pylon LE. Games 1 to 6 are of protocol version 95248, game 7 of 96000.
    /// In games 1 to 5 a human Terran beat a human Zerg; in game 6 Bob (Zerg) beat the
    /// computer; in game 7 Alice2 beat Bob without a third tech structure.
    ///
    /// | Game | Terran | Zerg | Terran: 2nd CC, upgrades, workers at 4:00 / 6:00 | Zerg: 2nd Hatchery |
    /// |---|---|---|---|---|
    /// | 1 | Alice | Bob | CC 120 s (lands again at 400 s), Weapons 1 at 300 s, Stimpack 200 s, a spray; 20 / 36 | 100 s |
    /// | 2 | Bea | Carl | CC 130 s, Weapons 1 at 310 s, Stimpack 220 s; 21 / 37 | 110 s |
    /// | 3 | Cleo | Dan | CC 140 s, Weapons 1 at 320 s; 22 / 38 | 120 s |
    /// | 4 | Dora | Eli | CC 150 s; 23 / 39 | 130 s |
    /// | 5 | Eve | Bob | none; 30 / 50 | 140 s, 3rd at 300 s |
    /// | 6 | A.I. (lost) | Bob (won) | | |
    /// | 7 | Alice2 | Bob | Barracks only | |
    ///
    /// The Terran open Barracks (60 s), Factory (120 s), Starport (180 s); the Zerg Spawning Pool
    /// (70 s), Roach Warren (130 s), Evolution Chamber (190 s).
    pub fn patch_games() -> Snapshot {
        let terran_opening = |tag: u32| {
            vec![
                Born::new(60, "Barracks", tag),
                Born::new(120, "Factory", tag + 1),
                Born::new(180, "Starport", tag + 2),
            ]
        };
        let zerg_opening = |tag: u32| {
            vec![
                Born::new(70, "SpawningPool", tag),
                Born::new(130, "RoachWarren", tag + 1),
                Born::new(190, "EvolutionChamber", tag + 2),
            ]
        };
        let minutes = |workers_4: i32, workers_6: i32| {
            vec![
                Minute::new(4, workers_4, 30, (500, 200), 400),
                Minute::new(6, workers_6, 44, (700, 400), 900),
            ]
        };
        let terran = |name: &'static str,
                      second_cc: Option<u32>,
                      upgrades: Vec<Upgrade>,
                      workers: (i32, i32)| {
            let mut born = terran_opening(10);
            if let Some(seconds) = second_cc {
                born.push(Born::new(seconds, "CommandCenter", 20));
            }
            Player {
                born,
                upgrades,
                minutes: minutes(workers.0, workers.1),
                ..Player::won(name, "Terran")
            }
        };
        let zerg = |name: &'static str, second_hatchery: u32| Player {
            born: {
                let mut born = zerg_opening(30);
                born.push(Born::new(second_hatchery, "Hatchery", 40));
                born
            },
            minutes: minutes(18, 34),
            ..Player::lost(name, "Zerg")
        };
        let game = |id: u64, players: Vec<Player>| Game {
            id,
            played_at: "2026-01-15 20:30:00",
            players,
            ..Default::default()
        };
        let weapons = |seconds: u32| Upgrade::new(seconds, "TerranInfantryWeaponsLevel1");
        Snapshot::new()
            .game(game(
                1,
                vec![
                    {
                        let mut alice = terran(
                            "Alice",
                            Some(120),
                            vec![
                                weapons(300),
                                Upgrade::new(200, "Stimpack"),
                                Upgrade::new(0, "SprayTerran"),
                            ],
                            (20, 36),
                        );
                        // The Command Center lands again: born once more with its tag.
                        alice.born.push(Born::new(400, "CommandCenter", 20));
                        alice
                    },
                    zerg("Bob", 100),
                ],
            ))
            .game(game(
                2,
                vec![
                    terran(
                        "Bea",
                        Some(130),
                        vec![weapons(310), Upgrade::new(220, "Stimpack")],
                        (21, 37),
                    ),
                    zerg("Carl", 110),
                ],
            ))
            .game(game(
                3,
                vec![
                    terran("Cleo", Some(140), vec![weapons(320)], (22, 38)),
                    zerg("Dan", 120),
                ],
            ))
            .game(game(
                4,
                vec![
                    terran("Dora", Some(150), vec![], (23, 39)),
                    zerg("Eli", 130),
                ],
            ))
            .game(game(
                5,
                vec![terran("Eve", None, vec![], (30, 50)), {
                    let mut bob = zerg("Bob", 140);
                    bob.born.push(Born::new(300, "Hatchery", 41));
                    bob
                }],
            ))
            .game(game(
                6,
                vec![
                    Player {
                        computer: true,
                        born: terran_opening(10),
                        minutes: minutes(10, 20),
                        ..Player::lost("A.I. 1 (Very Easy)", "Terran")
                    },
                    Player {
                        born: zerg_opening(30),
                        ..Player::won("Bob", "Zerg")
                    },
                ],
            ))
            .game(Game {
                id: 7,
                protocol_version: 96_000,
                played_at: "2026-02-10 20:30:00",
                players: vec![
                    Player {
                        born: vec![Born::new(60, "Barracks", 10)],
                        ..Player::won("Alice2", "Terran")
                    },
                    Player::lost("Bob", "Zerg"),
                ],
                ..Default::default()
            })
    }
}

/// The most costly lesson of the Main Player Name of Config in a replay file, for the message of
/// a new replay. None when the player is not in the replay or the game had nothing costly.
pub fn try_replay_lesson(
    replay_location: &str,
    main_player_name: &str,
) -> Result<Option<Lesson>, SwarmyError> {
    if main_player_name.is_empty() {
        return Ok(None);
    }
    let detail = data::analyze_replay_cached(replay_location)?;
    Ok(detail
        .players
        .iter()
        .find(|summary| summary.player.name.rsplit("<sp/>").next() == Some(main_player_name))
        .and_then(|summary| summary.lessons.first().cloned()))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn query_replay_lesson(
    app_handle: tauri::AppHandle,
    replay_location: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_replay_lesson(
            &replay_location,
            &app_config.main_player_name,
        ))
    })
    .await
}

/// The game as a single HTML page to share, and its map title.
pub async fn render_replay_report(
    host: &dyn SwarmyHost,
    ext_fs_id: u64,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(
            try_query_replay_detail(app_config.replay_path, &app_config.cache_path, ext_fs_id)
                .map(|detail| (report::render_report(&detail), detail.map_title.clone())),
        )
    })
    .await
}

/// Saves the game as a single HTML page to share, the folder and name are chosen in the dialog of
/// the system. The message is the file written, empty when the dialog was cancelled.
#[tauri::command(rename_all = "snake_case")]
pub async fn export_replay_report(
    app_handle: tauri::AppHandle,
    ext_fs_id: u64,
) -> Result<ApiResponse, SwarmyError> {
    use tauri_plugin_dialog::DialogExt;
    let app_config = load_stored_app_settings(&app_handle)?;
    let rendered = if app_config.is_remote() {
        crate::remote::call(
            &app_config,
            "render_replay_report",
            serde_json::json!({ "ext_fs_id": ext_fs_id }),
        )
        .await?
    } else {
        render_replay_report(&app_handle, ext_fs_id).await?
    };
    if !rendered.meta.success {
        return Ok(rendered);
    }
    let (html, map_title): (String, String) = serde_json::from_str(&rendered.message)?;
    let file_name = format!(
        "swarmy {} {ext_fs_id}.html",
        map_title.replace(['/', '\\', ':'], " ")
    );
    let dialog = app_handle
        .dialog()
        .file()
        .set_title("Save the game report")
        .set_file_name(file_name)
        .add_filter("HTML page", &["html"]);
    let picked = tokio::task::spawn_blocking(move || dialog.blocking_save_file()).await?;
    let Some(path) = picked else {
        return Ok(ApiResponseBuilder::new()
            .with_success()
            .with_message(String::new())
            .build());
    };
    let path = path
        .into_path()
        .map_err(|err| SwarmyError::Other(format!("Unsupported file: {err}")))?;
    std::fs::write(&path, html)?;
    Ok(ApiResponseBuilder::new()
        .with_success()
        .with_message(path.display().to_string())
        .build())
}

/// The newest replay, or the one before or after the current one, of the locked map and players.
pub async fn query_adjacent_replay(
    host: &dyn SwarmyHost,
    query: AdjacentReplayQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(navigation::try_adjacent_replay(
            app_config.replay_path,
            query,
        ))
    })
    .await
}
