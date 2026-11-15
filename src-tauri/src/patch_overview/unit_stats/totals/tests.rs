use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::patch_overview::unit_stats::test_games::marine_makers;

const README: &str = include_str!("README.md");

#[test]
fn the_games_units_deaths_and_kills_of_the_unit() {
    let snapshot = marine_makers();
    let totals = shape(
        plan(&snapshot.source(), "Marine")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        totals,
        Totals {
            games: 8,
            games_with_unit: 7,
            born: 62,
            died: 3,
            kills: 95,
            value_killed: 4_750,
        }
    );
}

#[test]
fn a_unit_never_made_gives_zeros_with_the_games_of_the_snapshot() {
    let snapshot = marine_makers();
    let totals = shape(
        plan(&snapshot.source(), "Colossus")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        totals,
        Totals {
            games: 8,
            ..Default::default()
        }
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = marine_makers();
    let plan = plan(&snapshot.source(), "Marine").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
