use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_other_accounts_with_a_name_like_the_players() {
    let snapshot = alice_and_friends();
    let similar = shape(
        plan(&snapshot.source(), &snapshot.groups_map(), "Alice")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(similar.len(), 1);
    assert_eq!(similar[0].names, ["Alice2"]);
    assert_eq!(similar[0].games, 1);
    assert_eq!(similar[0].grouped_as, "");
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), &snapshot.groups_map(), "Alice").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
