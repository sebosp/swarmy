use super::*;
use crate::data::fixtures::{Game, Player, Snapshot, assert_sql_matches_plan};

const README: &str = include_str!("README.md");

/// The games of the README's example.
fn games() -> Snapshot {
    Snapshot::new()
        .game(Game {
            id: 1,
            players: vec![
                Player {
                    account: 1001,
                    ..Player::won("&lt;TL&gt;<sp/>Alice", "Terran")
                },
                Player::lost("Bob", "Zerg"),
            ],
            ..Default::default()
        })
        .game(Game {
            id: 2,
            players: vec![
                Player::won("Carol", "Protoss"),
                Player {
                    account: 1001,
                    ..Player::lost("Alice", "Terran")
                },
            ],
            ..Default::default()
        })
        .game(Game {
            id: 3,
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
            players: vec![
                Player::won("Bob", "Zerg"),
                Player {
                    computer: true,
                    ..Player::lost("A.I. 1 (Very Easy)", "Protoss")
                },
            ],
            ..Default::default()
        })
        .group("2-1-777", "Alice")
}

fn rows_of(snapshot: &Snapshot) -> DataFrame {
    plan(&snapshot.source(), &snapshot.groups_map())
        .unwrap()
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("account"),
            col("name"),
            col("player"),
        ])
        .sort(["ext_fs_id", "player_name"], Default::default())
        .collect()
        .unwrap()
}

#[test]
fn an_account_is_one_player_whatever_the_clan_tag_or_the_group() {
    let rows = rows_of(&games());
    let names = str_values(&rows, "player_name").unwrap();
    let accounts = str_values(&rows, "account").unwrap();
    let players = str_values(&rows, "player").unwrap();
    let row = |name: &str| {
        let idx = names.iter().position(|n| n == name).unwrap();
        (accounts[idx].as_str(), players[idx].as_str())
    };
    // The clan tag is stripped, the account is named by its most played name.
    assert_eq!(row("&lt;TL&gt;<sp/>Alice"), ("2-1-1001", "Alice"));
    assert_eq!(row("Alice"), ("2-1-1001", "Alice"));
    // Grouped under Alice in the personal database.
    assert_eq!(row("AliceEU"), ("2-1-777", "Alice"));
    // A computer has no account.
    assert_eq!(row("A.I. 1 (Very Easy)"), ("", "A.I. 1 (Very Easy)"));
    assert_eq!(rows.height(), 8);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = games();
    let plan = plan(&snapshot.source(), &snapshot.groups_map()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
