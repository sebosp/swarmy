//! The use of an ability or unit, and the games of a race, per day, month or year: how the players
//! adapt to the patches. Read from the usage tables of the aggregations, a row per period,
//! protocol version, race, player and ability or unit. Each statistic is a query file of this
//! directory (AGENTS.md, "A query is a file"); this file runs them and puts the points together.

pub mod ability_uses;
pub mod patch_starts;
pub mod race_games;
pub mod unit_race;
pub mod unit_uses;

use crate::data::{IpcSnapshot, QueryMeta, SnapshotSource};
use crate::host::SwarmyHost;
use crate::replay_rerun::decisions::base_unit_name;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use swarmy_common::*;

/// The queries of the view, for the debug links of the panels.
pub const QUERIES: [QueryMeta; 5] = [
    race_games::META,
    ability_uses::META,
    unit_uses::META,
    unit_race::META,
    patch_starts::META,
];

/// The next period after one of the grain.
fn next_period(period: &str, grain: TimeGrain) -> Option<String> {
    let mut parts = period.split('-').map(|part| part.parse::<i32>().ok());
    let year = parts.next()??;
    Some(match grain {
        TimeGrain::Year => format!("{}", year + 1),
        TimeGrain::Month => {
            let month = parts.next()??;
            if month == 12 {
                format!("{}-01", year + 1)
            } else {
                format!("{year}-{:02}", month + 1)
            }
        }
        TimeGrain::Day => {
            let date = chrono::NaiveDate::parse_from_str(period, "%Y-%m-%d").ok()?;
            date.succ_opt()?.format("%Y-%m-%d").to_string()
        }
    })
}

/// Adds the periods without games between the first and last ones: the axis is time, a chart that
/// skipped them would draw the changes steeper than they were.
fn with_empty_periods(points: Vec<UsagePoint>, grain: TimeGrain) -> Vec<UsagePoint> {
    let mut filled: Vec<UsagePoint> = Vec::with_capacity(points.len());
    for point in points {
        while let Some(next) = filled
            .last()
            .and_then(|last| next_period(&last.period, grain))
            .filter(|next| *next < point.period)
        {
            filled.push(UsagePoint {
                period: next,
                ..Default::default()
            });
        }
        filled.push(point);
    }
    filled
}

/// The games of the race in each period, run for the subject.
fn race_points(
    source: &dyn SnapshotSource,
    grain: TimeGrain,
    race: &str,
) -> Result<std::collections::BTreeMap<String, race_games::RaceGames>, SwarmyError> {
    race_games::shape(race_games::plan(source, grain, race)?.collect()?)
}

pub fn try_query_usage_over_time(
    replay_path: &str,
    query: &UsageOverTimeQuery,
) -> Result<UsageOverTime, SwarmyError> {
    let source = IpcSnapshot::open(replay_path)?;
    let grain = query.grain;
    let points = match &query.subject {
        UsageSubject::Race(race) => race_points(&source, grain, race)?
            .into_iter()
            .map(|(period, games)| UsagePoint {
                period,
                player_games: games.games,
                games_used: games.versus_games,
                wins: games.versus_wins,
                uses: games.games,
                ..Default::default()
            })
            .collect(),
        UsageSubject::Ability(ability) => {
            let tracked = tracked_ability(ability)
                .ok_or_else(|| SwarmyError::Other(format!("{ability} is not a tracked ability")))?;
            let race_rows = race_points(&source, grain, tracked.race)?;
            let used = ability_uses::shape(
                ability_uses::plan(&source, grain, ability, tracked.race)?.collect()?,
            )?;
            race_rows
                .into_iter()
                .map(|(period, games)| {
                    let (games_used, wins, uses) = used.get(&period).copied().unwrap_or_default();
                    UsagePoint {
                        period,
                        player_games: games.games,
                        games_used,
                        wins,
                        uses,
                        ..Default::default()
                    }
                })
                .collect()
        }
        UsageSubject::Unit(unit) => {
            let unit = base_unit_name(unit);
            // The games of the race that makes the unit are the base.
            let race = unit_race::shape(unit_race::plan(&source, unit)?.collect()?)?;
            let race_rows = race_points(&source, grain, &race)?;
            let used = unit_uses::shape(unit_uses::plan(&source, grain, unit)?.collect()?)?;
            race_rows
                .into_iter()
                .map(|(period, games_of_race)| match used.get(&period) {
                    Some(use_) => UsagePoint {
                        player_games: games_of_race.games,
                        games_used: use_.games,
                        wins: use_.wins,
                        uses: use_.born,
                        died: use_.died,
                        kills: use_.kills,
                        value_killed: use_.value_killed,
                        period,
                    },
                    None => UsagePoint {
                        period,
                        player_games: games_of_race.games,
                        ..Default::default()
                    },
                })
                .collect()
        }
    };
    Ok(UsageOverTime {
        points: with_empty_periods(points, grain),
        patches: patch_starts::shape(patch_starts::plan(&source, grain)?.collect()?)?,
    })
}

pub async fn query_usage_over_time(
    host: &dyn SwarmyHost,
    query: UsageOverTimeQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(try_query_usage_over_time(&app_config.replay_path, &query))
    })
    .await
}

/// The games the tests of the queries of this view share.
#[cfg(test)]
pub mod test_games {
    use crate::data::fixtures::{Ability, Snapshot};
    use crate::player_detail::test_games::alice_and_friends;

    /// `alice_and_friends` with Spawn Larva used by Bob in game 1 (20 times, from 0:60) and by
    /// Alice as Zerg in game 5 (15 times, from 1:10), and MULEs called down by Alice as Terran in
    /// game 1 (5 times, from 3:00).
    pub fn with_abilities() -> Snapshot {
        let mut snapshot = alice_and_friends();
        for game in &mut snapshot.games {
            for player in &mut game.players {
                match (game.id, player.name) {
                    (1, "&lt;TL&gt;<sp/>Alice") => {
                        player.abilities = vec![Ability::new("CalldownMULE.Execute", 5, 180)]
                    }
                    (1, "Bob") => {
                        player.abilities = vec![Ability::new("SpawnLarva.Execute", 20, 60)]
                    }
                    (5, "Alice") => {
                        player.abilities = vec![Ability::new("SpawnLarva.Execute", 15, 70)]
                    }
                    _ => {}
                }
            }
        }
        snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn periods(periods: &[&str], grain: TimeGrain) -> Vec<String> {
        let points = periods
            .iter()
            .map(|period| UsagePoint {
                period: period.to_string(),
                player_games: 1,
                ..Default::default()
            })
            .collect();
        with_empty_periods(points, grain)
            .into_iter()
            .map(|point| point.period)
            .collect()
    }

    #[test]
    fn it_adds_the_periods_without_games() {
        assert_eq!(
            periods(&["2025-11", "2026-02"], TimeGrain::Month),
            ["2025-11", "2025-12", "2026-01", "2026-02"]
        );
        assert_eq!(
            periods(&["2024-02-28", "2024-03-01"], TimeGrain::Day),
            ["2024-02-28", "2024-02-29", "2024-03-01"]
        );
        assert_eq!(
            periods(&["2024", "2026"], TimeGrain::Year),
            ["2024", "2025", "2026"]
        );
    }
}
