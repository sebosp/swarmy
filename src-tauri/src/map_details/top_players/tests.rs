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

fn games_wins(label: &str, games: usize, wins: usize) -> GamesWins {
    GamesWins {
        label: label.to_string(),
        games,
        wins,
    }
}

#[test]
fn the_most_games_first_then_the_wins_then_the_name() {
    let players = shape(
        plan(&pylon_games().source(), "Pylon LE")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        players,
        vec![
            // Games 1, 2 and 5; won 1 and 5.
            games_wins("Alice", 3, 2),
            // Games 1 and 3; won 3. Before Carol by name.
            games_wins("Bob", 2, 1),
            // Games 2 and 5; won 2.
            games_wins("Carol", 2, 1),
            games_wins("Dave", 1, 0),
        ]
    );
}

#[test]
fn only_ten_players_are_listed() {
    let names = [
        "P01", "P02", "P03", "P04", "P05", "P06", "P07", "P08", "P09", "P10", "P11", "P12",
    ];
    let mut snapshot = Snapshot::new();
    for (index, pair) in names.chunks(2).enumerate() {
        snapshot = snapshot.game(Game {
            id: index as u64 + 1,
            players: vec![
                Player::won(pair[0], "Terran"),
                Player::lost(pair[1], "Zerg"),
            ],
            ..Default::default()
        });
    }
    let players = shape(
        plan(&snapshot.source(), "Pylon LE")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(players.len(), TOP_PLAYERS as usize);
    // Every player has one game: the winners (1 win) come first, then by name.
    assert_eq!(players[0].label, "P01");
    assert_eq!(players[5].label, "P11");
    assert_eq!(players[6].label, "P02");
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = pylon_games();
    let plan = plan(&snapshot.source(), "Pylon LE").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
