use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::usage_over_time::test_games::with_abilities;

const README: &str = include_str!("README.md");

#[test]
fn the_games_wins_and_uses_of_the_ability_per_month() {
    let snapshot = with_abilities();
    let uses = shape(
        plan(
            &snapshot.source(),
            TimeGrain::Month,
            "SpawnLarva.Execute",
            "Zerg",
        )
        .unwrap()
        .collect()
        .unwrap(),
    )
    .unwrap();
    let rows: Vec<(&str, (u64, u64, u64))> = uses.iter().map(|(p, v)| (p.as_str(), *v)).collect();
    // Game 1: Bob, 20 uses, lost; game 5: Alice, 15 uses, won.
    assert_eq!(rows, [("2026-01", (1, 0, 20)), ("2026-02", (1, 1, 15))]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = with_abilities();
    let plan = plan(
        &snapshot.source(),
        TimeGrain::Month,
        "SpawnLarva.Execute",
        "Zerg",
    )
    .unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
