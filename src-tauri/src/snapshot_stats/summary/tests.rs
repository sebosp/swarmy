use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_games_maps_and_dates_of_the_snapshot() {
    let snapshot = alice_and_friends();
    let summary = shape(plan(&snapshot.source()).unwrap().collect().unwrap()).unwrap();
    assert_eq!(
        summary,
        Summary {
            min_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            max_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            num_games: 6,
            num_maps: 2,
        }
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
