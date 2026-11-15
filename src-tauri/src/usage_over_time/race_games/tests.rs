use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_games_of_the_race_per_month_with_the_one_vs_ones_against_other_races() {
    let snapshot = alice_and_friends();
    let games = shape(
        plan(&snapshot.source(), TimeGrain::Month, "Zerg")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let rows: Vec<(&str, u64, u64, u64)> = games
        .iter()
        .map(|(period, g)| (period.as_str(), g.games, g.versus_games, g.versus_wins))
        .collect();
    assert_eq!(
        rows,
        [
            // Game 1: Bob lost to a Terran.
            ("2026-01", 1, 1, 0),
            // Games 3 (Dave, tie), 4 (Bob beat the computer) and 5 (three Zerg in a 2v2).
            ("2026-02", 5, 2, 1),
            // Game 6: Bob beat a Terran.
            ("2026-03", 1, 1, 1),
        ]
    );
    let by_year = shape(
        plan(&snapshot.source(), TimeGrain::Year, "Zerg")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(by_year["2026"].games, 7);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), TimeGrain::Month, "Zerg").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
