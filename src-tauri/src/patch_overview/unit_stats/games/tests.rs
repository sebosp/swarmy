use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::alice_and_friends;

const README: &str = include_str!("README.md");

#[test]
fn one_row_per_game_with_its_month_day_map_and_file() {
    let snapshot = alice_and_friends();
    let frame = plan(&snapshot.source()).unwrap().collect().unwrap();
    assert_eq!(frame.height(), 6);
    assert_eq!(u64_values(&frame, "ext_fs_id").unwrap(), [1, 2, 3, 4, 5, 6]);
    assert_eq!(str_values(&frame, "month").unwrap()[0], "2026-01");
    assert_eq!(str_values(&frame, "date").unwrap()[0], "2026-01-15");
    assert_eq!(str_values(&frame, "title").unwrap()[1], "Ephemeron LE");
    assert_eq!(
        str_values(&frame, "ext_fs_file_name").unwrap()[0],
        "1.SC2Replay"
    );
    assert_eq!(
        u32_values(&frame, "ext_protocol_version").unwrap()[2],
        96_000
    );
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = alice_and_friends();
    let plan = plan(&snapshot.source()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
