use super::*;
use crate::data::fixtures::assert_sql_matches_plan_in_order;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

/// The filters of the README's example.
fn example_query() -> ReplayListQuery {
    ReplayListQuery {
        map_title: "pylon".to_string(),
        player_name: "alice".to_string(),
        matchup: "TvZ".to_string(),
        min_date: chrono::NaiveDate::from_ymd_opt(2026, 2, 1),
        ..ReplayListQuery::newest_first()
    }
}

fn games_of(query: &ReplayListQuery) -> Vec<ReplayListEntry> {
    let snapshot = alice_and_friends();
    shape(
        plan(&snapshot.source(), &snapshot.groups_map(), query, None)
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn the_filters_of_the_example_keep_two_games_the_newest_first() {
    let games = games_of(&example_query());
    let ids: Vec<u64> = games.iter().map(|game| game.ext_fs_id).collect();
    assert_eq!(ids, [6, 3]);
    assert_eq!(games[0].player_list, ["Bob", "Alice2"]);
    assert_eq!(games[0].player_results, ["Win", "Loss"]);
    assert_eq!(games[0].player_races, ["Zerg", "Terran"]);
    assert_eq!(games[0].duration, 600);
    assert_eq!(games[0].replay_file_name, "6.SC2Replay");
    assert_eq!(games[1].replay_date.to_string(), "2026-02-10");
}

#[test]
fn without_filters_every_game_once_sorted_by_date_and_the_team_game_has_four_players() {
    let games = games_of(&ReplayListQuery::newest_first());
    let ids: Vec<u64> = games.iter().map(|game| game.ext_fs_id).collect();
    assert_eq!(ids, [6, 5, 4, 3, 2, 1]);
    assert_eq!(games[1].player_list.len(), 4);
}

#[test]
fn a_matchup_keeps_the_one_against_ones_only_and_the_notes_filter_keeps_its_replays() {
    let games = games_of(&ReplayListQuery {
        matchup: "ZvT".to_string(),
        ..ReplayListQuery::newest_first()
    });
    // TvZ and ZvT are one matchup; game 5 is a 2v2, game 2 PvT, game 4 PvZ.
    let ids: Vec<u64> = games.iter().map(|game| game.ext_fs_id).collect();
    assert_eq!(ids, [6, 3, 1]);
    let snapshot = alice_and_friends();
    let only_game_1: HashSet<String> = ["0000".to_string()].into_iter().collect();
    let kept = plan(
        &snapshot.source(),
        &snapshot.groups_map(),
        &ReplayListQuery::newest_first(),
        Some(&only_game_1),
    )
    .unwrap()
    .collect()
    .unwrap();
    // The fixtures give every replay the sha256 "0000".
    assert_eq!(kept.height(), 6);
    let none: HashSet<String> = HashSet::new();
    let kept = plan(
        &snapshot.source(),
        &snapshot.groups_map(),
        &ReplayListQuery::newest_first(),
        Some(&none),
    )
    .unwrap()
    .collect()
    .unwrap();
    assert_eq!(kept.height(), 0);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(
        &snapshot.source(),
        &snapshot.groups_map(),
        &example_query(),
        None,
    )
    .unwrap();
    assert_sql_matches_plan_in_order(README, &snapshot, plan);
}
