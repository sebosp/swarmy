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
            seconds: 300,
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
            cache_handles: "pylon-v2",
            players: vec![
                Player::won("Alice", "Terran"),
                Player::lost("Carol", "Protoss"),
                Player::observer("Caster"),
            ],
            ..Default::default()
        })
}

#[test]
fn four_games_between_two_dates_with_their_mean_length() {
    let summary = shape(
        plan(&pylon_games().source(), "Pylon LE")
            .unwrap()
            .collect()
            .unwrap(),
        "Pylon LE",
    )
    .unwrap();
    assert_eq!(
        summary,
        GamesSummary {
            games: 4,
            min_date: "2026-01-15".to_string(),
            max_date: "2026-01-19".to_string(),
            // (600 + 900 + 300 + 600) / 4
            avg_duration_seconds: 600,
            cache_handles: "pylon-v2".to_string(),
        }
    );
}

#[test]
fn a_map_without_games_is_an_error() {
    let frame = plan(&pylon_games().source(), "Nowhere")
        .unwrap()
        .collect()
        .unwrap();
    let err = shape(frame, "Nowhere").unwrap_err();
    assert!(
        err.to_string().contains("No games found for map Nowhere"),
        "{err}"
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = pylon_games();
    let plan = plan(&snapshot.source(), "Pylon LE").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
