use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::replay_detail::test_games::{PATCH_GAMES, patch_games};

const README: &str = include_str!("README.md");

#[test]
fn the_average_finish_of_the_nth_expansion_of_each_townhall() {
    let snapshot = patch_games();
    let averages = shape(
        plan(&snapshot.source(), &PATCH_GAMES)
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(averages.len(), 2, "{averages:?}");
    // The Command Center of game 1 landing again at 400 s is not another expansion.
    assert_eq!(averages[&("CommandCenter".to_string(), 1)], (135, 4));
    assert_eq!(averages[&("Hatchery".to_string(), 1)], (120, 5));
}

#[test]
fn it_numbers_the_expansions_once_per_townhall() {
    let townhalls = df!(
        "ext_fs_id" => [1u64, 1, 1, 1, 2, 3, 4],
        "control_player_id" => [Some(1u8), Some(1), Some(2), Some(1), Some(1), Some(1), Some(1)],
        "unit_tag_index" => [10u32, 11, 12, 10, 10, 10, 10],
        "unit_tag_recycle" => [1u32, 1, 1, 1, 1, 1, 1],
        "unit_type_name" => ["CommandCenter", "CommandCenter", "Nexus", "CommandCenter", "CommandCenter", "CommandCenter", "CommandCenter"],
        "game_seconds" => [100u32, 200, 110, 300, 120, 140, 150],
    )
    .unwrap();
    let expansions = averages_of(townhalls.lazy())
        .unwrap()
        .sort(["unit_type_name", "number"], Default::default())
        .collect()
        .unwrap();
    // The Command Center 10 landing again at 300 is not a new expansion, the second Command
    // Center and the Nexus have too few samples.
    assert_eq!(
        str_values(&expansions, "unit_type_name").unwrap(),
        ["CommandCenter"]
    );
    assert_eq!(u32_values(&expansions, "number").unwrap(), [1]);
    assert_eq!(u32_values(&expansions, "game_seconds").unwrap(), [127]);
    assert_eq!(u32_values(&expansions, "samples").unwrap(), [4]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = patch_games();
    let plan = plan(&snapshot.source(), &PATCH_GAMES).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
