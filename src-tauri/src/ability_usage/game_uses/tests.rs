use super::*;
use crate::data::fixtures::assert_sql_matches_source;
use crate::usage_over_time::test_games::with_abilities;

const README: &str = include_str!("README.md");

#[test]
fn every_use_under_its_patch_and_under_all_the_patches() {
    let snapshot = with_abilities();
    let frame = plan(&snapshot.source(), tracked_abilities())
        .unwrap()
        .collect()
        .unwrap();
    let rows: Vec<(u32, String, String, u64, String, u32)> = u32_values(&frame, "patch")
        .unwrap()
        .into_iter()
        .zip(str_values(&frame, "race").unwrap())
        .zip(str_values(&frame, "ability").unwrap())
        .zip(u64_values(&frame, "ext_fs_id").unwrap())
        .zip(str_values(&frame, "player_name").unwrap())
        .zip(u32_values(&frame, "uses").unwrap())
        .map(|(((((patch, race), ability), id), name), uses)| {
            (patch, race, ability, id, name, uses)
        })
        .collect();
    let row = |patch: u32, race: &str, ability: &str, id: u64, name: &str, uses: u32| {
        (
            patch,
            race.to_string(),
            ability.to_string(),
            id,
            name.to_string(),
            uses,
        )
    };
    assert_eq!(
        rows,
        [
            row(
                0,
                "Terran",
                "CalldownMULE.Execute",
                1,
                "&lt;TL&gt;<sp/>Alice",
                5
            ),
            row(0, "Zerg", "SpawnLarva.Execute", 1, "Bob", 20),
            row(0, "Zerg", "SpawnLarva.Execute", 5, "Alice", 15),
            row(
                95_248,
                "Terran",
                "CalldownMULE.Execute",
                1,
                "&lt;TL&gt;<sp/>Alice",
                5
            ),
            row(95_248, "Zerg", "SpawnLarva.Execute", 1, "Bob", 20),
            row(95_248, "Zerg", "SpawnLarva.Execute", 5, "Alice", 15),
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
