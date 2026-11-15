use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_accounts_with_their_names_the_most_games_first() {
    let snapshot = alice_and_friends();
    let accounts = shape(
        plan(&snapshot.source(), &snapshot.groups_map(), "Alice")
            .unwrap()
            .collect()
            .unwrap(),
        ACCOUNTS_SHOWN,
    )
    .unwrap();
    assert_eq!(
        accounts,
        vec![
            PlayerAccount {
                account: "2-1-1001".to_string(),
                // Games 1 (with the clan tag), 2 and 5.
                names: vec!["Alice".to_string()],
                games: 3,
                grouped_as: String::new(),
            },
            PlayerAccount {
                account: "2-1-777".to_string(),
                names: vec!["AliceEU".to_string()],
                games: 1,
                grouped_as: "Alice".to_string(),
            },
        ]
    );
}

#[test]
fn the_limit_counts_accounts_not_names() {
    let snapshot = alice_and_friends();
    let accounts = shape(
        plan(&snapshot.source(), &snapshot.groups_map(), "Alice")
            .unwrap()
            .collect()
            .unwrap(),
        1,
    )
    .unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account, "2-1-1001");
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), &snapshot.groups_map(), "Alice").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
