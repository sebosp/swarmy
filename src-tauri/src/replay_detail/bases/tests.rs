use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::replay_detail::test_games::fight;

const README: &str = include_str!("README.md");

#[test]
fn the_townhalls_of_the_players_once_each() {
    let snapshot = fight();
    let bases = shape(plan(&snapshot.source(), 1).unwrap().collect().unwrap()).unwrap();
    let base = |player: &str, unit_type: &str, x: f32, y: f32, seconds: u32| PlayerBase {
        player: player.to_string(),
        unit_type: unit_type.to_string(),
        x,
        y,
        seconds,
    };
    assert_eq!(
        bases,
        [
            base("Alice", "CommandCenter", 30., 30., 0),
            base("Bob", "Hatchery", 70., 70., 0),
            base("Bob", "Hatchery", 70., 40., 100),
            // Landing again at 400 s is not a new base.
            base("Alice", "CommandCenter", 30., 60., 120),
        ]
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = fight();
    let plan = plan(&snapshot.source(), 1).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
