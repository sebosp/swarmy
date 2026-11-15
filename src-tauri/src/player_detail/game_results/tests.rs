use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

fn games_wins(label: &str, games: usize, wins: usize) -> GamesWins {
    GamesWins {
        label: label.to_string(),
        games,
        wins,
    }
}

#[test]
fn one_row_per_game_with_the_name_played_as() {
    let snapshot = alice_and_friends();
    let frame = plan(&snapshot.source(), &snapshot.groups_map(), "Alice")
        .unwrap()
        .collect()
        .unwrap();
    assert_eq!(u64_values(&frame, "ext_fs_id").unwrap(), [1, 2, 3, 5]);
    assert_eq!(
        str_values(&frame, "played_as").unwrap(),
        ["Alice", "Alice", "AliceEU", "Alice"]
    );
    assert_eq!(
        str_values(&frame, "race").unwrap(),
        ["Terran", "Terran", "Terran", "Zerg"]
    );
}

#[test]
fn the_counts_by_race_month_and_version() {
    let snapshot = alice_and_friends();
    let results = shape(
        plan(&snapshot.source(), &snapshot.groups_map(), "Alice")
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        results,
        GameResults {
            games: 4,
            // Games 1 and 5.
            wins: 2,
            // Game 3.
            ties: 1,
            races: vec![games_wins("Terran", 3, 1), games_wins("Zerg", 1, 1)],
            by_month: vec![games_wins("2026-01", 1, 1), games_wins("2026-02", 3, 1)],
            by_version: vec![games_wins("95248", 3, 2), games_wins("96000", 1, 0)],
        }
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), &snapshot.groups_map(), "Alice").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
