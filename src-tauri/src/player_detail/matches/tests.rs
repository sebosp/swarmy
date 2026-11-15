use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn one_row_per_opponent_the_newest_first_without_the_teammates() {
    let snapshot = alice_and_friends();
    let matches = shape(
        plan(&snapshot.source(), &snapshot.groups_map(), "Alice")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let rows: Vec<(u64, &str, &str, bool)> = matches
        .iter()
        .map(|m| {
            (
                m.ext_fs_id,
                m.opponent.as_str(),
                m.played_as.as_str(),
                m.won,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (5, "Carol", "Alice", true),
            (5, "Dave", "Alice", true),
            (3, "Dave", "AliceEU", false),
            (2, "Carol", "Alice", false),
            (1, "Bob", "Alice", true),
        ]
    );
    assert!(matches[2].tie);
    assert_eq!(matches[0].race, "Zerg");
    assert_eq!(matches[0].opponent_race, "Protoss");
    assert_eq!(matches[4].map_title, "Pylon LE");
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), &snapshot.groups_map(), "Alice").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
