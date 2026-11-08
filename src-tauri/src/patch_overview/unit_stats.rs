//! How a unit was used in the games of the snapshot: the units made, died and their kills, by
//! month, and the games in which the unit was exceptionally effective.

use crate::data::*;
use crate::replay_rerun::decisions::{base_unit_name, latest_balance_units, unit_cost};
use polars::prelude::*;
use swarmy_common::*;

/// The unit names that are the same unit, i.e. `RoachBurrowed` is a Roach.
pub fn unit_variants(unit_name: &str) -> Vec<String> {
    const VARIANTS: [&str; 7] = [
        "SiegeTankSieged",
        "VikingAssault",
        "LiberatorAG",
        "ThorAP",
        "WarpPrismPhasing",
        "ObserverSiegeMode",
        "OverseerSiegeMode",
    ];
    let mut names = vec![unit_name.to_string(), format!("{unit_name}Burrowed")];
    names.extend(
        VARIANTS
            .iter()
            .filter(|variant| base_unit_name(variant) == unit_name)
            .map(|variant| variant.to_string()),
    );
    names
}

/// A single value of the first row of a column, 0 when the frame is empty.
fn first_u64(df: &DataFrame, name: &str) -> Result<u64, SwarmyError> {
    Ok(df
        .column(name)?
        .cast(&DataType::UInt64)?
        .u64()?
        .get(0)
        .unwrap_or_default())
}

fn first_f64(df: &DataFrame, name: &str) -> Result<f64, SwarmyError> {
    Ok(df
        .column(name)?
        .cast(&DataType::Float64)?
        .f64()?
        .get(0)
        .unwrap_or_default())
}

pub fn try_query_unit_snapshot_stats(
    replay_path: String,
    unit_name: String,
) -> Result<UnitSnapshotStats, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    let balance = latest_balance_units()?;
    let cost = unit_cost(balance, &unit_name);

    // The units made by each player in each game, the kills and the units lost.
    let unit_rows = scan_snapshot_ipc(&ipc_path, GAME_UNITS_IPC)?
        .filter(col("unit").eq(lit(base_unit_name(&unit_name))));
    let players = unit_rows
        .clone()
        .filter(col("born").gt(lit(0u32)))
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("born").cast(DataType::UInt64),
            col("kills").cast(DataType::UInt64),
            col("value_killed").cast(DataType::UInt64),
        ])
        .with_column(
            (col("kills").cast(DataType::Float64) / col("born").cast(DataType::Float64))
                .alias("kills_per_unit"),
        )
        .collect()?
        .lazy();

    let totals = players
        .clone()
        .select([
            col("born").sum(),
            col("kills").sum(),
            col("value_killed").sum(),
            col("ext_fs_id").n_unique().alias("games_with_unit"),
        ])
        .collect()?;
    let died = unit_rows
        .select([col("died").cast(DataType::UInt64).sum()])
        .collect()?;

    // One row per game with its month, title, date, etc.
    let details = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?;
    let has_protocol_version = details
        .clone()
        .collect_schema()?
        .contains("ext_protocol_version");
    let protocol_version = if has_protocol_version {
        col("ext_protocol_version").first()
    } else {
        lit(0u32).alias("ext_protocol_version")
    };
    let games = details
        .group_by([col("ext_fs_id")])
        .agg([
            col("ext_datetime")
                .first()
                .dt()
                .to_string("%Y-%m")
                .alias("month"),
            col("ext_datetime")
                .first()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("date"),
            col("title").first(),
            col("ext_fs_file_name").first(),
            protocol_version,
        ])
        .collect()?
        .lazy();
    let total_games = games.clone().select([len().alias("games")]).collect()?;

    let born_by_game = players
        .clone()
        .group_by([col("ext_fs_id")])
        .agg([col("born").sum()]);
    let by_month_df = games
        .clone()
        .join(
            born_by_game,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .group_by([col("month")])
        .agg([
            len().cast(DataType::UInt64).alias("games"),
            col("born")
                .is_not_null()
                .cast(DataType::UInt64)
                .sum()
                .alias("games_with_unit"),
            col("born").fill_null(lit(0u64)).sum().alias("born"),
        ])
        .sort(["month"], Default::default())
        .collect()?;
    let months = str_values(&by_month_df, "month")?;
    let month_games = u64_values(&by_month_df, "games")?;
    let month_games_with_unit = u64_values(&by_month_df, "games_with_unit")?;
    let month_born = u64_values(&by_month_df, "born")?;
    let by_month = (0..by_month_df.height())
        .map(|idx| UnitMonthUsage {
            month: months[idx].clone(),
            games: month_games[idx] as usize,
            games_with_unit: month_games_with_unit[idx] as usize,
            born: month_born[idx] as usize,
        })
        .collect();

    // The kills per unit of the players that made a few of the unit, the games two standard
    // deviations above the mean are exceptional.
    let compared = players.filter(col("born").gt_eq(lit(EXCEPTIONAL_MIN_BORN as u64)));
    let kills_per_unit_stats = compared
        .clone()
        .select([
            col("kills_per_unit").mean().alias("mean"),
            col("kills_per_unit").std(1).fill_null(lit(0.)).alias("std"),
        ])
        .collect()?;
    let kills_per_unit_mean = first_f64(&kills_per_unit_stats, "mean")?;
    let kills_per_unit_threshold =
        kills_per_unit_mean + 2. * first_f64(&kills_per_unit_stats, "std")?;
    let exceptional_df = compared
        .filter(
            col("kills_per_unit")
                .gt_eq(lit(kills_per_unit_threshold))
                .and(col("kills").gt(lit(0u64))),
        )
        .join(
            games,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .sort(
            ["kills_per_unit", "kills"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .limit(EXCEPTIONAL_GAMES_LIMIT as IdxSize)
        .collect()?;
    let ext_fs_ids = u64_values(&exceptional_df, "ext_fs_id")?;
    let locations = str_values(&exceptional_df, "ext_fs_file_name")?;
    let titles = str_values(&exceptional_df, "title")?;
    let dates = str_values(&exceptional_df, "date")?;
    let versions = u32_values(&exceptional_df, "ext_protocol_version")?;
    let player_names = str_values(&exceptional_df, "player_name")?;
    let borns = u64_values(&exceptional_df, "born")?;
    let kills = u64_values(&exceptional_df, "kills")?;
    let values_killed = u64_values(&exceptional_df, "value_killed")?;
    let exceptional = (0..exceptional_df.height())
        .map(|idx| ExceptionalUnitGame {
            ext_fs_id: ext_fs_ids[idx],
            replay_location: locations[idx].clone(),
            map_title: titles[idx].clone(),
            date: dates[idx].clone(),
            protocol_version: versions[idx],
            player_name: player_names[idx].clone(),
            born: borns[idx] as usize,
            kills: kills[idx] as usize,
            value_killed: values_killed[idx] as u32,
        })
        .collect();

    Ok(UnitSnapshotStats {
        unit_name,
        games: first_u64(&total_games, "games")? as usize,
        games_with_unit: first_u64(&totals, "games_with_unit")? as usize,
        born: first_u64(&totals, "born")? as usize,
        died: first_u64(&died, "died")? as usize,
        kills: first_u64(&totals, "kills")? as usize,
        value_killed: first_u64(&totals, "value_killed")? as u32,
        unit_cost: cost.minerals + cost.vespene,
        by_month,
        kills_per_unit_mean,
        kills_per_unit_threshold,
        exceptional,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_lists_the_unit_variants() {
        assert_eq!(unit_variants("Roach"), vec!["Roach", "RoachBurrowed"]);
        assert!(unit_variants("SiegeTank").contains(&"SiegeTankSieged".to_string()));
        assert!(unit_variants("VikingFighter").contains(&"VikingAssault".to_string()));
    }
}

#[cfg(test)]
mod snapshot_tests {
    /// Runs against a real snapshot: `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test -- --ignored`
    #[test]
    #[ignore]
    fn it_queries_a_snapshot() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let start = std::time::Instant::now();
        let stats = super::try_query_unit_snapshot_stats(replay_path, "Roach".to_string()).unwrap();
        eprintln!("{:?}", start.elapsed());
        eprintln!(
            "games {} with {} born {} died {} kills {} value {} cost {} mean {:.2} thr {:.2}",
            stats.games,
            stats.games_with_unit,
            stats.born,
            stats.died,
            stats.kills,
            stats.value_killed,
            stats.unit_cost,
            stats.kills_per_unit_mean,
            stats.kills_per_unit_threshold
        );
        for month in stats.by_month.iter().take(5) {
            eprintln!("{month:?}");
        }
        for game in stats.exceptional.iter().take(5) {
            eprintln!("{game:?}");
        }
        assert!(stats.born > 0);
        assert!(stats.games_with_unit <= stats.games);
    }
}
