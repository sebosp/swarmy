use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::replay_detail::test_games::{PATCH_GAMES, patch_games};

const README: &str = include_str!("README.md");

#[test]
fn the_average_finish_of_the_upgrades_with_enough_samples() {
    let snapshot = patch_games();
    let averages = shape(
        plan(&snapshot.source(), &PATCH_GAMES)
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(averages.len(), 1, "{averages:?}");
    assert_eq!(
        averages["TerranInfantryWeaponsLevel1"],
        PatchAverage {
            seconds: 310,
            samples: 3,
        }
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = patch_games();
    let plan = plan(&snapshot.source(), &PATCH_GAMES).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
