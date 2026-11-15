use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn the_first_month_of_each_version_and_patch() {
    let snapshot = alice_and_friends();
    let frame = plan(&snapshot.source(), TimeGrain::Month)
        .unwrap()
        .collect()
        .unwrap();
    assert_eq!(
        u32_values(&frame, "ext_protocol_version").unwrap(),
        [95_248, 96_000]
    );
    assert_eq!(
        str_values(&frame, "period").unwrap(),
        ["2026-01", "2026-02"]
    );
    let starts = shape(frame).unwrap();
    assert!(!starts.is_empty());
    assert_eq!(starts[0].period, "2026-01");
    assert!(
        starts
            .windows(2)
            .all(|pair| pair[0].period <= pair[1].period)
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source(), TimeGrain::Month).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
