use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::progress::test_games::alice_progress;

const README: &str = include_str!("README.md");

#[test]
fn the_last_stats_before_each_minute_and_the_seconds_blocked() {
    let snapshot = alice_progress();
    let frame = plan(&snapshot.source(), "Alice")
        .unwrap()
        .collect()
        .unwrap();
    let values = shape(frame).unwrap();
    let game_1 = &values[&1];
    assert_eq!(game_1["workers_6"], 42.);
    assert_eq!(game_1["income_8"], 1500.);
    assert_eq!(game_1["army_10"], 1900.);
    // Two stats blocked, 160 loops each at 22.4 loops per second.
    assert!((game_1["supply_blocked"] - 2. * 160. / 22.4).abs() < 1e-9);
    // No stats: only the zero seconds blocked.
    assert_eq!(values[&2].len(), 1);
    assert_eq!(values[&2]["supply_blocked"], 0.);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_progress();
    let plan = plan(&snapshot.source(), "Alice").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
