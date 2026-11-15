use super::*;
use crate::data::fixtures::assert_sql_matches_source;
use crate::replay_detail::test_games::patch_games;

const README: &str = include_str!("README.md");

const TERRAN_OPENING: &str = "Barracks → Factory → Starport";
const ZERG_OPENING: &str = "SpawningPool → RoachWarren → EvolutionChamber";

#[test]
fn the_openings_and_how_often_they_won_against_the_enemy_opening() {
    let snapshot = patch_games();
    let answers = shape(
        plan(&snapshot.source(), tech_structures().unwrap())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(answers.len(), 2, "{answers:?}");
    let terran = &answers[&(
        "Terran".to_string(),
        "Zerg".to_string(),
        ZERG_OPENING.to_string(),
    )];
    assert_eq!(
        terran,
        // The five human games and the computer's loss in game 6.
        &[OpeningAnswer {
            opening: TERRAN_OPENING.to_string(),
            games: 6,
            wins: 5,
        }]
    );
    let zerg = &answers[&(
        "Zerg".to_string(),
        "Terran".to_string(),
        TERRAN_OPENING.to_string(),
    )];
    assert_eq!((zerg[0].games, zerg[0].wins), (6, 1));
}

#[test]
fn the_tech_structures_are_the_ones_of_the_openings() {
    let tech = tech_structures().unwrap();
    let units = str_values(&tech, "unit").unwrap();
    for unit in [
        "Barracks",
        "Factory",
        "Starport",
        "SpawningPool",
        "RoachWarren",
        "EvolutionChamber",
    ] {
        assert!(units.iter().any(|u| u == unit), "{unit}");
    }
    for not_tech in ["SupplyDepot", "Hatchery", "BarracksFlying"] {
        assert!(!units.iter().any(|u| u == not_tech), "{not_tech}");
    }
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = patch_games();
    let tech = tech_structures().unwrap();
    let source = snapshot.source().with(TECH_STRUCTURES_IPC, tech.clone());
    let plan = plan(&snapshot.source(), tech).unwrap();
    assert_sql_matches_source(README, &source, plan, false);
}
