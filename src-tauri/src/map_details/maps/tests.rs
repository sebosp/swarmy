use super::*;
use crate::data::fixtures::{Game, Player, Snapshot, assert_sql_matches_plan};

const README: &str = include_str!("README.md");

/// The games of the README's example.
fn games() -> Snapshot {
    Snapshot::new()
        .game(Game {
            id: 1,
            ..Default::default()
        })
        .game(Game {
            id: 2,
            map: "Ephemeron LE",
            ..Default::default()
        })
        .game(Game {
            id: 3,
            ..Default::default()
        })
        .game(Game {
            id: 4,
            players: vec![
                Player::won("Alice", "Terran"),
                Player::lost("Bob", "Zerg"),
                Player::observer("Caster"),
            ],
            ..Default::default()
        })
}

#[test]
fn the_most_played_map_first_and_a_game_counted_once() {
    let maps = shape(plan(&games().source()).unwrap().collect().unwrap()).unwrap();
    assert_eq!(
        maps,
        vec![
            GamesWins {
                label: "Pylon LE".to_string(),
                games: 3,
                wins: 0,
            },
            GamesWins {
                label: "Ephemeron LE".to_string(),
                games: 1,
                wins: 0,
            },
        ]
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = games();
    let plan = plan(&snapshot.source()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
