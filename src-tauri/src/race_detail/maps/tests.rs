use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_games_of_the_race_by_map_and_matchup_and_the_minimum_of_the_table() {
    let snapshot = alice_and_friends();
    let frame = plan(&snapshot.source(), "Zerg").unwrap().collect().unwrap();
    assert_eq!(
        str_values(&frame, "title").unwrap(),
        ["Pylon LE", "Pylon LE"]
    );
    assert_eq!(str_values(&frame, "matchup").unwrap(), ["ZvP", "ZvT"]);
    assert_eq!(u32_values(&frame, "games").unwrap(), [1, 3]);
    assert_eq!(i32_values(&frame, "wins").unwrap(), [1, 1]);
    // Four games on the map: fewer than the five the table needs.
    assert!(shape(frame).unwrap().is_empty());
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), "Zerg").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
