use super::*;
use crate::data::fixtures::assert_sql_matches_source;
use crate::replay_detail::test_games::fight;
use crate::replay_detail::valued_units::{VALUED_UNITS_IPC, valued_units};

const README: &str = include_str!("README.md");

#[test]
fn the_deaths_that_cost_resources_by_time() {
    let snapshot = fight();
    let deaths = shape(
        plan(&snapshot.source(), 1, valued_units().unwrap())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    let death = |x: f32, y: f32, seconds: u32| MapDeath { x, y, seconds };
    assert_eq!(
        deaths,
        [
            death(48., 50., 300),
            death(50., 48., 302),
            death(52., 50., 305),
            death(50., 52., 310),
            death(30., 30., 450),
            death(10., 90., 500),
        ]
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = fight();
    let valued = valued_units().unwrap();
    let source = snapshot.source().with(VALUED_UNITS_IPC, valued.clone());
    let plan = plan(&snapshot.source(), 1, valued).unwrap();
    assert_sql_matches_source(README, &source, plan, false);
}
