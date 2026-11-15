use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::replay_detail::test_games::patch_games;

const README: &str = include_str!("README.md");

#[test]
fn the_games_of_the_version() {
    let snapshot = patch_games();
    let games = shape(plan(&snapshot.source(), 95_248).unwrap().collect().unwrap()).unwrap();
    assert_eq!(games, [1, 2, 3, 4, 5, 6]);
    let other = shape(plan(&snapshot.source(), 96_000).unwrap().collect().unwrap()).unwrap();
    assert_eq!(other, [7]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = patch_games();
    let plan = plan(&snapshot.source(), 95_248).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
