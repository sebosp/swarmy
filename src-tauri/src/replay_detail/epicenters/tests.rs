use super::*;
use crate::data::fixtures::assert_sql_matches_source;
use crate::replay_detail::test_games::{fight, fight_engagements};
use crate::replay_detail::valued_units::{VALUED_UNITS_IPC, valued_units};

const README: &str = include_str!("README.md");

#[test]
fn the_center_and_the_spread_of_the_deaths_of_each_engagement() {
    let snapshot = fight();
    let windows = engagement_windows(&fight_engagements()).unwrap();
    let epicenters = shape(
        plan(&snapshot.source(), 1, windows, valued_units().unwrap())
            .unwrap()
            .collect()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        epicenters,
        [
            (
                0,
                Epicenter {
                    x: 50.,
                    y: 50.,
                    radius: 2.,
                    deaths: 4,
                }
            ),
            (
                1,
                Epicenter {
                    x: 10.,
                    y: 90.,
                    radius: 0.,
                    deaths: 1,
                }
            ),
        ]
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = fight();
    let windows = engagement_windows(&fight_engagements()).unwrap();
    let valued = valued_units().unwrap();
    let source = snapshot
        .source()
        .with(ENGAGEMENTS_IPC, windows.clone())
        .with(VALUED_UNITS_IPC, valued.clone());
    let plan = plan(&snapshot.source(), 1, windows, valued).unwrap();
    assert_sql_matches_source(README, &source, plan, false);
}
