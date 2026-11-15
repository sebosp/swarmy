use super::*;
use crate::data::fixtures::assert_sql_matches_source;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_army_units_of_the_race_with_the_games_they_were_made_in() {
    let snapshot = alice_and_friends();
    let units = shape(
        plan(&snapshot.source(), "Zerg", race_units("Zerg").unwrap())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        units,
        vec![RaceUnitStats {
            unit: "Zergling".to_string(),
            games: 1,
            born: 30,
            kills: 10,
            wins: 1,
        }]
    );
    // The Terran units of Alice are not Zerg units.
    let terran = shape(
        plan(&snapshot.source(), "Terran", race_units("Terran").unwrap())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let names: Vec<&str> = terran.iter().map(|unit| unit.unit.as_str()).collect();
    assert_eq!(names, ["Marine", "Marauder"]);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let units = race_units("Zerg").unwrap();
    let source = snapshot.source().with(RACE_UNITS_IPC, units.clone());
    let plan = plan(&snapshot.source(), "Zerg", units).unwrap();
    assert_sql_matches_source(README, &source, plan, false);
}
