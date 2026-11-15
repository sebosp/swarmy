use super::*;
use crate::data::fixtures::{Game, Player, Snapshot, assert_sql_matches_plan};

const README: &str = include_str!("README.md");

/// The games of the README's example.
fn pylon_games() -> Snapshot {
    Snapshot::new()
        .game(Game {
            id: 1,
            played_at: "2026-01-15 20:30:00",
            ..Default::default()
        })
        .game(Game {
            id: 2,
            played_at: "2026-01-16 20:30:00",
            seconds: 900,
            players: vec![
                Player::won("Carol", "Protoss"),
                Player::lost("Alice", "Terran"),
            ],
            ..Default::default()
        })
        .game(Game {
            id: 3,
            played_at: "2026-01-17 20:30:00",
            players: vec![Player::won("Bob", "Zerg"), Player::lost("Dave", "Zerg")],
            ..Default::default()
        })
        .game(Game {
            id: 4,
            map: "Ephemeron LE",
            played_at: "2026-01-18 20:30:00",
            players: vec![
                Player::won("Alice", "Terran"),
                Player::lost("Carol", "Protoss"),
            ],
            ..Default::default()
        })
        .game(Game {
            id: 5,
            played_at: "2026-01-19 20:30:00",
            players: vec![
                Player::won("Alice", "Terran"),
                Player::lost("Carol", "Protoss"),
                Player::observer("Caster"),
            ],
            ..Default::default()
        })
}

fn recent_games_of(snapshot: &Snapshot) -> Vec<ReplayListEntry> {
    shape(
        plan(&snapshot.source(), "Pylon LE")
            .unwrap()
            .collect()
            .unwrap(),
        "Pylon LE",
    )
    .unwrap()
}

#[test]
fn the_newest_games_first_with_their_players_without_the_observers() {
    let games = recent_games_of(&pylon_games());
    let ids: Vec<u64> = games.iter().map(|game| game.ext_fs_id).collect();
    assert_eq!(ids, [5, 3, 2, 1]);
    assert_eq!(games[0].player_list, ["Alice", "Carol"]);
    assert_eq!(games[0].player_results, ["Win", "Loss"]);
    assert_eq!(games[0].player_races, ["Terran", "Protoss"]);
    assert_eq!(games[0].replay_date.to_string(), "2026-01-19");
    assert_eq!(games[0].map_title, "Pylon LE");
    assert_eq!(games[0].replay_file_name, "5.SC2Replay");
    assert_eq!(games[0].duration, 600);
    // Carol is team 1 in game 2: first.
    assert_eq!(games[2].player_list, ["Carol", "Alice"]);
    assert_eq!(games[2].duration, 900);
}

#[test]
fn at_most_twenty_games() {
    let mut snapshot = Snapshot::new();
    for id in 1..=25u64 {
        snapshot = snapshot.game(Game {
            id,
            ..Default::default()
        });
    }
    let games = recent_games_of(&snapshot);
    assert_eq!(games.len(), RECENT_GAMES as usize);
    // The same day: the highest ids are the newest.
    assert_eq!(games[0].ext_fs_id, 25);
    assert_eq!(games[19].ext_fs_id, 6);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = pylon_games();
    let plan = plan(&snapshot.source(), "Pylon LE").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
