use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_newest_version_first_with_its_games() {
    let versions = shape(
        plan(&alice_and_friends().source())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let rows: Vec<(&str, usize)> = versions
        .iter()
        .map(|v| (v.label.as_str(), v.games))
        .collect();
    assert_eq!(rows, [("96000", 3), ("95248", 3)]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
