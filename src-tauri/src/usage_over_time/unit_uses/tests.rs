use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_units_made_per_month_with_their_games_and_kills() {
    let snapshot = alice_and_friends();
    let uses = shape(
        plan(&snapshot.source(), TimeGrain::Month, "Zergling")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(uses.len(), 1);
    assert_eq!(
        uses["2026-02"],
        UnitUse {
            games: 1,
            wins: 1,
            born: 30,
            died: 0,
            kills: 10,
            value_killed: 500,
        }
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), TimeGrain::Month, "Zergling").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
