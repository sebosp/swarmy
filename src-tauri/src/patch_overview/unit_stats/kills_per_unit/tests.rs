use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::patch_overview::unit_stats::test_games::marine_makers;

const README: &str = include_str!("README.md");

#[test]
fn the_kills_per_unit_of_the_players_that_made_enough_and_the_threshold() {
    let snapshot = marine_makers();
    let frame = plan(&snapshot.source(), "Marine", EXCEPTIONAL_MIN_BORN)
        .unwrap()
        .collect()
        .unwrap();
    // Gina made 2 Marines in game 7: not compared.
    assert_eq!(frame.height(), 6);
    let kills_per_unit: Vec<f64> = frame
        .column("kills_per_unit")
        .unwrap()
        .f64()
        .unwrap()
        .iter()
        .flatten()
        .collect();
    assert_eq!(kills_per_unit, [0.5, 0.5, 0.5, 0.5, 0.5, 5.0]);
    let threshold = threshold(&frame).unwrap();
    assert!((threshold.mean - 1.25).abs() < 1e-9);
    assert!((threshold.threshold - 4.924).abs() < 1e-3, "{threshold:?}");
}

#[test]
fn nobody_made_enough_gives_no_rows_and_a_zero_threshold() {
    let snapshot = marine_makers();
    let frame = plan(&snapshot.source(), "Colossus", EXCEPTIONAL_MIN_BORN)
        .unwrap()
        .collect()
        .unwrap();
    assert_eq!(frame.height(), 0);
    assert_eq!(threshold(&frame).unwrap(), Threshold::default());
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = marine_makers();
    let plan = plan(&snapshot.source(), "Marine", EXCEPTIONAL_MIN_BORN).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
