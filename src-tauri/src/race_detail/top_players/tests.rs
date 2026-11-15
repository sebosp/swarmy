use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_players_of_the_race_with_the_most_games_first() {
    let snapshot = alice_and_friends();
    let players = shape(plan(&snapshot.source(), "Zerg").unwrap().collect().unwrap()).unwrap();
    let rows: Vec<(&str, usize, usize)> = players
        .iter()
        .map(|p| (p.label.as_str(), p.games, p.wins))
        .collect();
    assert_eq!(rows, [("Bob", 4, 3), ("Dave", 2, 0), ("Alice", 1, 1)]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), "Zerg").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
