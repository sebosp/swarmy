use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_rows_of_the_players_of_the_race() {
    let snapshot = alice_and_friends();
    let games = shape(plan(&snapshot.source(), "Zerg").unwrap().collect().unwrap()).unwrap();
    // Bob in games 1, 4, 5 and 6, Dave in 3 and 5, Alice in 5.
    assert_eq!(games, 7);
    let protoss = shape(
        plan(&snapshot.source(), "Protoss")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    // Carol in games 2 and 5, the computer in game 4.
    assert_eq!(protoss, 3);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), "Zerg").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
