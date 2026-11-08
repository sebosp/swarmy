//! The use of an ability or unit, and the games of a race, per day, month or year: how the players
//! adapt to the patches. Read from the usage tables of the aggregations, a row per period,
//! protocol version, race, player and ability or unit.

use crate::data::*;
use crate::host::SwarmyHost;
use crate::replay_rerun::decisions::{base_unit_name, closest_balance_version};
use crate::run_query;
use crate::settings::load_stored_app_settings;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

/// The games of the players of a race in a period, and its 1v1 games against other races.
#[derive(Debug, Default, Clone, Copy)]
struct RaceGames {
    games: u64,
    versus_games: u64,
    versus_wins: u64,
}

/// The games of the players of the race in each period.
fn race_games(
    ipc_path: &str,
    grain: TimeGrain,
    race: &str,
) -> Result<BTreeMap<String, RaceGames>, SwarmyError> {
    let games = scan_usage(ipc_path, grain, PLAYER_USAGE_IPC)?
        .filter(race_name_expr("player_race").eq(lit(race)))
        .group_by([col("period")])
        .agg(
            ["games", "versus_games", "versus_wins"]
                .map(|name| col(name).cast(DataType::UInt64).sum()),
        )
        .collect()?;
    let [counts, versus_games, versus_wins] = ["games", "versus_games", "versus_wins"]
        .map(|name| u64_values(&games, name).unwrap_or_default());
    Ok(str_values(&games, "period")?
        .into_iter()
        .enumerate()
        .map(|(idx, period)| {
            (
                period,
                RaceGames {
                    games: counts[idx],
                    versus_games: versus_games[idx],
                    versus_wins: versus_wins[idx],
                },
            )
        })
        .collect())
}

/// The first period of each patch; the protocol versions of a patch are its closest balance data.
fn patch_starts(ipc_path: &str, grain: TimeGrain) -> Result<Vec<PatchStart>, SwarmyError> {
    let versions = scan_usage(ipc_path, grain, PLAYER_USAGE_IPC)?
        .group_by([col("ext_protocol_version")])
        .agg([col("period").min()])
        .collect()?;
    let mut starts: BTreeMap<u32, String> = BTreeMap::new();
    for (version, period) in u32_values(&versions, "ext_protocol_version")?
        .into_iter()
        .zip(str_values(&versions, "period")?)
    {
        let start = starts
            .entry(closest_balance_version(version)?)
            .or_insert_with(|| period.clone());
        if period < *start {
            *start = period;
        }
    }
    let mut starts: Vec<PatchStart> = starts
        .into_iter()
        .map(|(patch, period)| PatchStart { period, patch })
        .collect();
    starts.sort_by(|a, b| a.period.cmp(&b.period).then(a.patch.cmp(&b.patch)));
    Ok(starts)
}

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

pub fn try_query_usage_over_time(
    replay_path: String,
    query: UsageOverTimeQuery,
) -> Result<UsageOverTime, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let grain = query.grain;
    let points = match &query.subject {
        UsageSubject::Race(race) => race_games(&ipc_path, grain, race)?
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
            let race_rows = race_games(&ipc_path, grain, tracked.race)?;
            let rows = scan_usage(&ipc_path, grain, ABILITY_USAGE_IPC)?
                .filter(
                    col("ability")
                        .eq(lit(ability.as_str()))
                        .and(race_name_expr("player_race").eq(lit(tracked.race))),
                )
                .group_by([col("period")])
                .agg([
                    col("games").cast(DataType::UInt64).sum(),
                    col("wins").cast(DataType::UInt64).sum(),
                    col("uses").cast(DataType::UInt64).sum(),
                ])
                .collect()?;
            let mut used: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
            let (games, wins, uses) = (
                u64_values(&rows, "games")?,
                u64_values(&rows, "wins")?,
                u64_values(&rows, "uses")?,
            );
            for (idx, period) in str_values(&rows, "period")?.into_iter().enumerate() {
                used.insert(period, (games[idx], wins[idx], uses[idx]));
            }
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
            // The games of the race that makes the unit are the base.
            let races = scan_usage(&ipc_path, TimeGrain::Year, UNIT_USAGE_IPC)?
                .filter(col("unit").eq(lit(base_unit_name(unit))))
                .group_by([race_name_expr("player_race").alias("race")])
                .agg([col("born").cast(DataType::UInt64).sum()])
                .sort(
                    ["born"],
                    SortMultipleOptions::default().with_order_descending(true),
                )
                .limit(1)
                .collect()?;
            let race = str_values(&races, "race")?
                .into_iter()
                .next()
                .unwrap_or_default();
            let race_rows = race_games(&ipc_path, grain, &race)?;
            let rows = scan_usage(&ipc_path, grain, UNIT_USAGE_IPC)?
                .filter(col("unit").eq(lit(base_unit_name(unit))))
                .group_by([col("period")])
                .agg(
                    ["games", "wins", "born", "died", "kills", "value_killed"]
                        .map(|name| col(name).cast(DataType::UInt64).sum()),
                )
                .collect()?;
            let periods = str_values(&rows, "period")?;
            let [games, wins, born, died, kills, values] =
                ["games", "wins", "born", "died", "kills", "value_killed"]
                    .map(|name| u64_values(&rows, name).unwrap_or_default());
            let by_period: BTreeMap<&String, usize> = periods
                .iter()
                .enumerate()
                .map(|(idx, period)| (period, idx))
                .collect();
            race_rows
                .into_iter()
                .map(|(period, games_of_race)| match by_period.get(&period) {
                    Some(idx) => UsagePoint {
                        player_games: games_of_race.games,
                        games_used: games[*idx],
                        wins: wins[*idx],
                        uses: born[*idx],
                        died: died[*idx],
                        kills: kills[*idx],
                        value_killed: values[*idx],
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
        patches: patch_starts(&ipc_path, grain)?,
    })
}

pub async fn query_usage_over_time(
    host: &dyn SwarmyHost,
    query: UsageOverTimeQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .process_result(try_query_usage_over_time(app_config.replay_path, query))
    })
    .await
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
