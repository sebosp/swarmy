use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::patch_overview::unit_stats::test_games::marine_makers;

const README: &str = include_str!("README.md");

#[test]
fn the_games_and_units_of_each_month_including_months_without_the_unit() {
    let snapshot = marine_makers();
    let months = shape(
        plan(&snapshot.source(), "Marine")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let rows: Vec<(&str, usize, usize, usize)> = months
        .iter()
        .map(|m| (m.month.as_str(), m.games, m.games_with_unit, m.born))
        .collect();
    assert_eq!(rows, [("2026-01", 6, 6, 60), ("2026-02", 2, 1, 2)]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = marine_makers();
    let plan = plan(&snapshot.source(), "Marine").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
