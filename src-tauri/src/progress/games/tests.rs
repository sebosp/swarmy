use super::*;
use crate::data::fixtures::assert_sql_matches_plan_in_order;
use crate::progress::test_games::alice_progress;

const README: &str = include_str!("README.md");

#[test]
fn the_games_of_the_player_the_oldest_first_with_their_opponents() {
    let snapshot = alice_progress();
    let games = shape(
        plan(&snapshot.source(), "Alice")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let rows: Vec<(u64, &str, Vec<&str>, bool)> = games
        .iter()
        .map(|game| {
            (
                game.ext_fs_id,
                game.result.as_str(),
                game.opponents.iter().map(String::as_str).collect(),
                game.vs_computer,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (1, "Win", vec!["Bob"], false),
            (2, "Win", vec!["A.I. 1 (Very Easy)"], true),
            (3, "Loss", vec!["Carol", "Dave"], false),
        ]
    );
    assert_eq!(games[0].date, "2026-01-15 20:30");
    assert_eq!(games[2].race, "Zerg");
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_progress();
    let plan = plan(&snapshot.source(), "Alice").unwrap();
    assert_sql_matches_plan_in_order(README, &snapshot, plan);
}
