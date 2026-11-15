use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::patch_overview::unit_stats::test_games::marine_makers;

const README: &str = include_str!("README.md");

/// The threshold of the example of `kills_per_unit`, 1.25 + 2 * 1.837.
const THRESHOLD: f64 = 4.924;

#[test]
fn the_players_above_the_threshold_with_their_game() {
    let snapshot = marine_makers();
    let games = shape(
        plan(
            &snapshot.source(),
            "Marine",
            EXCEPTIONAL_MIN_BORN,
            THRESHOLD,
        )
        .unwrap()
        .collect()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(games.len(), 1);
    let game = &games[0];
    assert_eq!(game.player_name, "Fiona");
    assert_eq!(game.ext_fs_id, 6);
    assert_eq!((game.born, game.kills, game.value_killed), (10, 50, 2_500));
    assert_eq!(game.date, "2026-01-20");
    assert_eq!(game.map_title, "Pylon LE");
    assert_eq!(game.replay_location, "6.SC2Replay");
    assert_eq!(game.protocol_version, 95_248);
}

#[test]
fn a_threshold_of_zero_takes_everyone_with_a_kill_the_best_first() {
    let snapshot = marine_makers();
    let games = shape(
        plan(&snapshot.source(), "Marine", EXCEPTIONAL_MIN_BORN, 0.)
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let names: Vec<&str> = games.iter().map(|g| g.player_name.as_str()).collect();
    // Fiona first, then the five at 0.5 by game.
    assert_eq!(names, ["Fiona", "Alice", "Bea", "Cleo", "Dora", "Eve"]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = marine_makers();
    let plan = plan(
        &snapshot.source(),
        "Marine",
        EXCEPTIONAL_MIN_BORN,
        THRESHOLD,
    )
    .unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
