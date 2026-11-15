use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_race_that_made_the_most_of_the_unit() {
    let snapshot = alice_and_friends();
    let race = shape(
        plan(&snapshot.source(), "Zergling")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(race, "Zerg");
    let none = shape(
        plan(&snapshot.source(), "Colossus")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(none, "");
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), "Zergling").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
