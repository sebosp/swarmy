use super::*;
use crate::ability_usage::game_uses::{TRACKED_ABILITIES_IPC, tracked_abilities};
use crate::data::fixtures::assert_sql_matches_source;
use crate::usage_over_time::test_games::with_abilities;

const README: &str = include_str!("README.md");

#[test]
fn the_players_with_enough_games_ranked_by_uses_per_game() {
    let snapshot = with_abilities();
    let frame = plan(&snapshot.source(), tracked_abilities())
        .unwrap()
        .collect()
        .unwrap();
    assert_eq!(frame.height(), 1);
    assert_eq!(u32_values(&frame, "patch").unwrap(), [0]);
    assert_eq!(str_values(&frame, "player_name").unwrap(), ["Bob"]);
    assert_eq!(
        str_values(&frame, "ability").unwrap(),
        ["SpawnLarva.Execute"]
    );
    assert_eq!(u32_values(&frame, "uses").unwrap(), [20]);
    assert_eq!(u32_values(&frame, "games").unwrap(), [4]);
    assert_eq!(
        frame.column("per_game").unwrap().f64().unwrap().get(0),
        Some(5.0)
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = with_abilities();
    let tracked = tracked_abilities();
    let source = snapshot
        .source()
        .with(TRACKED_ABILITIES_IPC, tracked.clone());
    let plan = plan(&snapshot.source(), tracked).unwrap();
    assert_sql_matches_source(README, &source, plan, false);
}
