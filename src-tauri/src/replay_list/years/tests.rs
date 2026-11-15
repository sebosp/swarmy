use super::*;
use crate::data::fixtures::{Game, Snapshot, assert_sql_matches_plan};
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_years_with_games_the_newest_first() {
    let snapshot = alice_and_friends().game(Game {
        id: 7,
        played_at: "2024-06-01 12:00:00",
        ..Default::default()
    });
    let years = shape(plan(&snapshot.source()).unwrap().collect().unwrap()).unwrap();
    assert_eq!(years, [2026, 2024]);
    assert_eq!(
        shape(
            plan(&Snapshot::new().game(Game::default()).source())
                .unwrap()
                .collect()
                .unwrap()
        )
        .unwrap(),
        [2026]
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
