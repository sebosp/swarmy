use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_non_mirror_games_of_the_race_by_patch_the_oldest_first() {
    let snapshot = alice_and_friends();
    let by_patch = shape(plan(&snapshot.source(), "Zerg").unwrap().collect().unwrap()).unwrap();
    let rows: Vec<(usize, usize)> = by_patch.iter().map(|p| (p.games, p.wins)).collect();
    // Version 95248: game 1 (lost); version 96000: games 3 (tie), 4 and 6 (won).
    assert_eq!(rows, [(1, 0), (3, 2)]);
    let patches: Vec<u32> = by_patch.iter().map(|p| p.label.parse().unwrap()).collect();
    assert!(patches[0] < patches[1]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), "Zerg").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
