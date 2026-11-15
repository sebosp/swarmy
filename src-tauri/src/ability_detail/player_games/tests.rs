use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::usage_over_time::test_games::with_abilities;

const README: &str = include_str!("README.md");

fn spawn_larva() -> &'static TrackedAbility {
    tracked_ability("SpawnLarva.Execute").unwrap()
}

#[test]
fn every_game_of_the_race_with_its_matchup_and_uses() {
    let snapshot = with_abilities();
    let games = shape(
        plan(&snapshot.source(), spawn_larva())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    /// Game, player, matchup, uses, first use, patch.
    type Row<'a> = (u64, &'a str, &'a str, usize, Option<u32>, u32);
    let rows: Vec<Row> = games
        .iter()
        .map(|g| {
            (
                g.game.ext_fs_id,
                g.game.player_name.as_str(),
                g.game.matchup.as_str(),
                g.game.uses,
                g.game.first_seconds,
                g.patch,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (1, "Bob", "ZvT", 20, Some(60), 95_248),
            (3, "Dave", "ZvT", 0, None, 95_841),
            (4, "Bob", "ZvP", 0, None, 95_841),
            (5, "Alice", "Team games", 15, Some(70), 95_248),
            (5, "Bob", "Team games", 0, None, 95_248),
            (5, "Dave", "Team games", 0, None, 95_248),
            (6, "Bob", "ZvT", 0, None, 95_841),
        ]
    );
    assert_eq!(games[0].game.date, "2026-01-15");
    assert_eq!(games[0].game.map_title, "Pylon LE");
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = with_abilities();
    let plan = plan(&snapshot.source(), spawn_larva()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
