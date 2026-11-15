use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_most_games_first_and_a_player_is_their_accounts() {
    let snapshot = alice_and_friends();
    let players = shape(
        plan(&snapshot.source(), &snapshot.groups_map())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let labels: Vec<(&str, usize, usize)> = players
        .iter()
        .map(|p| (p.label.as_str(), p.games, p.wins))
        .collect();
    assert_eq!(
        labels,
        [
            // Games 1, 2, 3 (as AliceEU) and 5; won 1 and 5.
            ("Alice", 4, 2),
            // Games 1, 4, 5 and 6; won 4, 5 and 6.
            ("Bob", 4, 3),
            ("Carol", 2, 1),
            ("Dave", 2, 0),
            ("A.I. 1 (Very Easy)", 1, 0),
            ("Alice2", 1, 0),
        ]
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), &snapshot.groups_map()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
