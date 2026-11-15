use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::replay_detail::test_games::patch_games;

const README: &str = include_str!("README.md");

#[test]
fn the_medians_of_the_winners_of_each_matchup_at_the_benchmark_minutes() {
    let snapshot = patch_games();
    let benchmarks = shape(plan(&snapshot.source(), 95_248).unwrap().collect().unwrap()).unwrap();
    assert_eq!(benchmarks.len(), 1, "{benchmarks:?}");
    let (medians, games) = &benchmarks[&("Terran".to_string(), "Zerg".to_string())];
    assert_eq!(*games, 5);
    assert_eq!(medians.len(), 2);
    assert_eq!(
        medians[0],
        Benchmark {
            minute: 4,
            workers: 22,
            supply: 30,
            minerals_rate: 500,
            vespene_rate: 200,
            army_value: 400,
        }
    );
    assert_eq!((medians[1].minute, medians[1].workers), (6, 38));
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = patch_games();
    let plan = plan(&snapshot.source(), 95_248).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
