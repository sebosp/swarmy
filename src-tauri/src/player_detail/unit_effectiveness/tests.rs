use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_units_of_every_game_and_account_of_the_player_add_up() {
    let snapshot = alice_and_friends();
    let frame = plan(&snapshot.source(), &snapshot.groups_map(), "Alice")
        .unwrap()
        .collect()
        .unwrap();
    assert_eq!(
        str_values(&frame, "unit").unwrap(),
        ["Marauder", "Marine", "Zergling"]
    );
    // Marines: 20 in game 1 and 10 in game 2.
    assert_eq!(u32_values(&frame, "born").unwrap(), [4, 30, 30]);
    assert_eq!(u32_values(&frame, "kills").unwrap(), [6, 20, 10]);
    assert_eq!(
        u64_values(&frame, "value_killed").unwrap(),
        [600, 1000, 500]
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), &snapshot.groups_map(), "Alice").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
