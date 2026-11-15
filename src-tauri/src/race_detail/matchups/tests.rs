use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_one_vs_one_games_of_the_race_by_matchup() {
    let snapshot = alice_and_friends();
    let matchups = shape(plan(&snapshot.source(), "Zerg").unwrap().collect().unwrap()).unwrap();
    let rows: Vec<(&str, usize, usize)> = matchups
        .iter()
        .map(|m| (m.label.as_str(), m.games, m.wins))
        .collect();
    assert_eq!(rows, [("ZvP", 1, 1), ("ZvT", 3, 1)]);
}

#[test]
fn two_computers_with_the_same_name_are_a_one_vs_one() {
    use crate::data::fixtures::{Game, Player, Snapshot};
    let snapshot = Snapshot::new().game(Game {
        players: vec![
            Player {
                computer: true,
                ..Player::won("A.I. 1 (Very Easy)", "Zerg")
            },
            Player {
                computer: true,
                ..Player::lost("A.I. 1 (Very Easy)", "Protoss")
            },
        ],
        ..Default::default()
    });
    let matchups = shape(plan(&snapshot.source(), "Zerg").unwrap().collect().unwrap()).unwrap();
    assert_eq!(matchups.len(), 1);
    assert_eq!(
        (
            matchups[0].label.as_str(),
            matchups[0].games,
            matchups[0].wins
        ),
        ("ZvP", 1, 1)
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), "Zerg").unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
