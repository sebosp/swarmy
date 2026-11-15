use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_most_met_first_a_team_game_once_per_enemy() {
    let snapshot = alice_and_friends();
    let opponents = shape(
        plan(&snapshot.source(), &snapshot.groups_map(), "Alice")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let rows: Vec<(&str, usize, usize)> = opponents
        .iter()
        .map(|o| (o.opponent.as_str(), o.games, o.wins))
        .collect();
    assert_eq!(rows, [("Carol", 2, 1), ("Dave", 2, 1), ("Bob", 1, 1)]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), &snapshot.groups_map(), "Alice").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
