use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

fn example_query() -> MapStatsQuery {
    MapStatsQuery {
        player_name: "alice".to_string(),
        ..Default::default()
    }
}

#[test]
fn the_maps_of_the_games_of_a_player_the_most_played_first() {
    let snapshot = alice_and_friends();
    let maps = shape(
        plan(&snapshot.source(), &example_query())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let rows: Vec<(&str, u32, String, String)> = maps
        .iter()
        .map(|map| {
            (
                map.title.as_str(),
                map.num_games,
                map.min_date.to_string(),
                map.max_date.to_string(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            // Games 1, 3 (AliceEU), 5 and 6 (Alice2).
            (
                "Pylon LE",
                4,
                "2026-01-15".to_string(),
                "2026-03-01".to_string()
            ),
            (
                "Ephemeron LE",
                1,
                "2026-02-03".to_string(),
                "2026-02-03".to_string()
            ),
        ]
    );
    assert_eq!(maps[0].cache_handles, "pylon-v1");
}

#[test]
fn without_a_filter_every_game_counts_once() {
    let snapshot = alice_and_friends();
    let maps = shape(
        plan(&snapshot.source(), &MapStatsQuery::default())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let games: Vec<u32> = maps.iter().map(|map| map.num_games).collect();
    assert_eq!(games, [5, 1]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), &example_query()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
