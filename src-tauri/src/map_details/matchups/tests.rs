use super::*;
use crate::data::fixtures::{Game, Player, Snapshot, assert_sql_matches_plan};

const README: &str = include_str!("README.md");

/// The games of the README's example.
fn pylon_games() -> Snapshot {
    Snapshot::new()
        .game(Game {
            id: 1,
            ..Default::default()
        })
        .game(Game {
            id: 2,
            players: vec![
                Player::won("Carol", "Protoss"),
                Player::lost("Alice", "Terran"),
            ],
            ..Default::default()
        })
        .game(Game {
            id: 3,
            players: vec![Player::won("Bob", "Zerg"), Player::lost("Dave", "Zerg")],
            ..Default::default()
        })
        .game(Game {
            id: 4,
            map: "Ephemeron LE",
            players: vec![
                Player::won("Alice", "Terran"),
                Player::lost("Carol", "Protoss"),
            ],
            ..Default::default()
        })
        .game(Game {
            id: 5,
            players: vec![
                Player::won("Alice", "Terran"),
                Player::lost("Carol", "Protoss"),
                Player::observer("Caster"),
            ],
            ..Default::default()
        })
}

fn matchups_of(snapshot: &Snapshot) -> Vec<GamesWins> {
    shape(
        plan(&snapshot.source(), "Pylon LE")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap()
}

fn games_wins(label: &str, games: usize, wins: usize) -> GamesWins {
    GamesWins {
        label: label.to_string(),
        games,
        wins,
    }
}

#[test]
fn the_races_are_sorted_and_the_wins_are_of_the_first_race() {
    assert_eq!(
        matchups_of(&pylon_games()),
        vec![
            // Game 2 (Carol won as Protoss) and game 5 (Carol lost): 1 of 2.
            games_wins("PvT", 2, 1),
            // Game 1, Alice won as Terran against a Zerg.
            games_wins("TvZ", 1, 1),
            // Game 3, a mirror: a game, never a win.
            games_wins("ZvZ", 1, 0),
        ]
    );
}

#[test]
fn localized_race_names_are_the_same_matchup() {
    let snapshot = Snapshot::new().game(Game {
        players: vec![
            Player::won("Alice", "Terraner"),
            Player::lost("Bob", "저그"),
        ],
        ..Default::default()
    });
    assert_eq!(matchups_of(&snapshot), vec![games_wins("TvZ", 1, 1)]);
}

#[test]
fn team_games_and_arcade_races_have_no_matchup() {
    let snapshot = Snapshot::new()
        .game(Game {
            id: 1,
            players: vec![
                Player::won("Alice", "Terran"),
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
                    ..Player::lost("Dave", "Protoss")
                },
            ],
            ..Default::default()
        })
        .game(Game {
            id: 2,
            players: vec![
                Player::won("Alice", "Terran"),
                Player::lost("Bob", "Primal Zerg"),
            ],
            ..Default::default()
        });
    assert_eq!(matchups_of(&snapshot), vec![]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = pylon_games();
    let plan = plan(&snapshot.source(), "Pylon LE").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
