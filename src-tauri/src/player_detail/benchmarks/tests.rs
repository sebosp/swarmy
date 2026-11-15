use super::*;
use crate::data::fixtures::assert_sql_matches_plan;
use crate::player_detail::test_games::with_minutes;

const README: &str = include_str!("README.md");

#[test]
fn the_average_economy_of_each_matchup_at_the_benchmark_minutes() {
    let snapshot = with_minutes();
    let table = shape(plan(&snapshot.source()).unwrap().collect().unwrap()).unwrap();
    let mut keys: Vec<&(String, u32)> = table.keys().collect();
    keys.sort();
    let keys: Vec<(&str, u32)> = keys.iter().map(|(m, min)| (m.as_str(), *min)).collect();
    // Game 1 (Alice TvZ Bob) and game 6 (Alice2 TvZ Bob) have minute 4 and 6 samples; game 3
    // (AliceEU TvZ Dave) only minute 4; game 4 (Bob ZvP the computer) minute 4; game 2 (Carol
    // PvT Alice) none; game 5 is a 2v2.
    assert_eq!(
        keys,
        [
            ("PvZ", 4),
            ("TvZ", 4),
            ("TvZ", 6),
            ("ZvP", 4),
            ("ZvT", 4),
            ("ZvT", 6),
        ]
    );
    let tvz4 = &table[&("TvZ".to_string(), 4)];
    assert_eq!(tvz4.games, 3);
    // Alice 20, AliceEU 18, Alice2 16.
    assert_eq!(tvz4.workers, 18.);
    assert_eq!(tvz4.income, (800. + 700. + 600.) / 3.);
    let zvt4 = &table[&("ZvT".to_string(), 4)];
    assert_eq!(zvt4.games, 3);
    // Bob 22, Dave 24, Bob 20.
    assert_eq!(zvt4.workers, 22.);
    assert_eq!(table[&("ZvT".to_string(), 6)].games, 2);
    assert_eq!(table[&("ZvP".to_string(), 4)].supply, 30.);
}

#[test]
fn the_readme_sql_gives_the_same_frame() {
    let snapshot = with_minutes();
    let plan = plan(&snapshot.source()).unwrap();
    assert_sql_matches_plan(README, &snapshot, plan);
}
