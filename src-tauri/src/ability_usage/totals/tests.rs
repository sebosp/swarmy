use super::*;
use crate::ability_usage::game_uses::{TRACKED_ABILITIES_IPC, tracked_abilities};
use crate::data::fixtures::assert_sql_matches_source;
use crate::usage_over_time::test_games::with_abilities;

const README: &str = include_str!("README.md");

#[test]
fn the_games_used_the_uses_and_the_games_of_the_race_per_patch() {
    let snapshot = with_abilities();
    let frame = plan(&snapshot.source(), tracked_abilities())
        .unwrap()
        .collect()
        .unwrap();
    let rows: Vec<(u32, String, String, u32, u32, u32)> = u32_values(&frame, "patch")
        .unwrap()
        .into_iter()
        .zip(str_values(&frame, "race").unwrap())
        .zip(str_values(&frame, "ability").unwrap())
        .zip(u32_values(&frame, "games_used").unwrap())
        .zip(u32_values(&frame, "uses").unwrap())
        .zip(u32_values(&frame, "player_games").unwrap())
        .map(
            |(((((patch, race), ability), games_used), uses), player_games)| {
                (patch, race, ability, games_used, uses, player_games)
            },
        )
        .collect();
    let row = |patch: u32, race: &str, ability: &str, used: u32, uses: u32, games: u32| {
        (
            patch,
            race.to_string(),
            ability.to_string(),
            used,
            uses,
            games,
        )
    };
    // Terran player games: Alice 1, Alice 2, AliceEU 3, Alice2 6 (4; 2 of patch 95248).
    // Zerg player games: Bob 1, Dave 3, Bob 4, Alice 5, Bob 5, Dave 5, Bob 6 (7; 4 of 95248).
    assert_eq!(
        rows,
        [
            row(0, "Terran", "CalldownMULE.Execute", 1, 5, 4),
            row(0, "Zerg", "SpawnLarva.Execute", 2, 35, 7),
            row(95_248, "Terran", "CalldownMULE.Execute", 1, 5, 2),
            row(95_248, "Zerg", "SpawnLarva.Execute", 2, 35, 4),
        ]
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
