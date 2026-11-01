//! The progress of the main player of Config: the workers at 06:00, the income at 08:00, the
//! seconds supply blocked and the army at 10:00 of each of their last games, and their goals.

use crate::data::*;
use crate::replay_rerun::viewer::GAME_LOOPS_PER_SECOND;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;
use tauri_plugin_store::StoreBuilder;

/// The last games of the player shown.
pub const PROGRESS_GAMES: u32 = 200;

/// The control of the computer players in the details.
const COMPUTER_CONTROL: u8 = 3;

/// The seconds between two stats of a player, 160 game loops.
const STATS_INTERVAL_SECONDS: f64 = 160. / GAME_LOOPS_PER_SECOND;

/// The stats of a minute are the last ones of the seconds before it.
const STATS_WINDOW_SECONDS: f64 = 15.;

const GOALS_KEY: &str = "progress_goals";

/// The name without the clan tag, i.e. `&lt;chezs&gt;<sp/>Sazed` is `Sazed`.
fn name_without_clan(column: &str) -> Expr {
    col(column).str().split(lit("<sp/>")).list().last()
}

/// The last stats of each game and player in the seconds before the minute.
fn stats_at(stats: &LazyFrame, minute: u32, value: Expr, alias: &str) -> LazyFrame {
    let end = minute as f64 * 60. * GAME_LOOPS_PER_SECOND;
    let start = end - STATS_WINDOW_SECONDS * GAME_LOOPS_PER_SECOND;
    stats
        .clone()
        .filter(
            col("ext_replay_loop")
                .gt(lit(start as i64))
                .and(col("ext_replay_loop").lt_eq(lit(end as i64))),
        )
        .sort(["ext_replay_loop"], Default::default())
        .group_by([col("ext_fs_id"), col("player_id")])
        .agg([value.last().cast(DataType::Float64).alias(alias)])
}

pub fn try_query_progress(
    replay_path: String,
    player: String,
    goals: BTreeMap<String, f64>,
) -> Result<ProgressReport, SwarmyError> {
    let mut report = ProgressReport {
        player: player.clone(),
        goals,
        ..Default::default()
    };
    if player.is_empty() {
        return Ok(report);
    }
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let details = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?.with_columns([
        // The tracker player id is the position of the player in the details.
        (int_range(lit(0), len(), 1, DataType::UInt32) + lit(1u32))
            .over([col("ext_fs_id")])?
            .cast(DataType::UInt8)
            .alias("player_id"),
        name_without_clan("player_name").alias("name"),
    ]);
    let own = details
        .clone()
        .filter(col("name").eq(lit(player.as_str())))
        .sort(
            ["ext_datetime"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .unique_stable(Some(cols(["ext_fs_id"])), UniqueKeepStrategy::First)
        .limit(PROGRESS_GAMES)
        .select([
            col("ext_fs_id"),
            col("player_id"),
            col("player_team_id").alias("team_id"),
            race_name_expr("player_race").alias("race"),
            col("player_result").alias("result"),
            col("title"),
            col("ext_datetime")
                .dt()
                .to_string("%Y-%m-%d %H:%M")
                .alias("date"),
        ])
        .collect()?;
    let game_ids = u64_values(&own, "ext_fs_id")?;
    let opponents = details
        .filter(is_in_replays(&game_ids))
        .join(
            own.clone()
                .lazy()
                .select([col("ext_fs_id"), col("team_id")]),
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("player_team_id").neq(col("team_id")))
        .group_by([col("ext_fs_id")])
        .agg([
            col("player_name").alias("opponents"),
            col("player_control")
                .eq(lit(COMPUTER_CONTROL))
                .any(true)
                .alias("vs_computer"),
        ]);
    let stats = scan_snapshot_ipc(&ipc_path, STATS_IPC)?
        .filter(is_in_replays(&game_ids))
        .join(
            own.clone()
                .lazy()
                .select([col("ext_fs_id"), col("player_id")]),
            [col("ext_fs_id"), col("player_id")],
            [col("ext_fs_id"), col("player_id")],
            JoinArgs::new(JoinType::Inner),
        );
    let blocked = stats
        .clone()
        .filter(
            col("food_made")
                .gt(lit(0))
                .and(col("food_used").gt_eq(col("food_made")))
                .and(col("food_made").lt(lit(200))),
        )
        .group_by([col("ext_fs_id"), col("player_id")])
        .agg([
            (len().cast(DataType::Float64) * lit(STATS_INTERVAL_SECONDS)).alias("supply_blocked"),
        ]);
    let measured = [
        stats_at(&stats, 6, col("workers_active_count"), "workers_6"),
        stats_at(
            &stats,
            8,
            col("minerals_collection_rate") + col("vespene_collection_rate"),
            "income_8",
        ),
        stats_at(
            &stats,
            10,
            col("minerals_used_current_army") + col("vespene_used_current_army"),
            "army_10",
        ),
        blocked,
    ];
    let mut games = own.lazy().join(
        opponents,
        [col("ext_fs_id")],
        [col("ext_fs_id")],
        JoinArgs::new(JoinType::Left),
    );
    for values in measured {
        games = games.join(
            values,
            [col("ext_fs_id"), col("player_id")],
            [col("ext_fs_id"), col("player_id")],
            JoinArgs::new(JoinType::Left),
        );
    }
    let games = games.sort(["date"], Default::default()).collect()?;
    let ids = u64_values(&games, "ext_fs_id")?;
    let (dates, titles) = (str_values(&games, "date")?, str_values(&games, "title")?);
    let (races, results) = (str_values(&games, "race")?, str_values(&games, "result")?);
    let opponents = games.column("opponents")?.list()?.clone();
    let vs_computer = games.column("vs_computer")?.bool()?.clone();
    let metric_values: Vec<(&str, Float64Chunked)> = PROGRESS_METRICS
        .iter()
        .map(|metric| -> Result<_, SwarmyError> {
            let values = games.column(metric.key)?.f64()?.clone();
            Ok((metric.key, values))
        })
        .collect::<Result<_, _>>()?;
    for idx in 0..games.height() {
        let names = opponents
            .get_as_series(idx)
            .map(|series| -> Result<Vec<String>, SwarmyError> {
                Ok(series.str()?.iter().flatten().map(str::to_string).collect())
            })
            .transpose()?
            .unwrap_or_default();
        let mut values = BTreeMap::new();
        for (key, column) in &metric_values {
            match (column.get(idx), *key) {
                (Some(value), _) => {
                    values.insert(key.to_string(), value);
                }
                // A game without a supply block has no rows of it.
                (None, "supply_blocked") => {
                    values.insert(key.to_string(), 0.);
                }
                _ => {}
            }
        }
        report.games.push(ProgressGame {
            ext_fs_id: ids[idx],
            date: dates[idx].clone(),
            map_title: titles[idx].clone(),
            race: races[idx].clone(),
            result: results[idx].clone(),
            opponents: names,
            vs_computer: vs_computer.get(idx).unwrap_or(false),
            values,
        });
    }
    Ok(report)
}

fn read_goals(app_handle: &tauri::AppHandle) -> Result<BTreeMap<String, f64>, SwarmyError> {
    let store = StoreBuilder::new(app_handle, "settings.json").build()?;
    let _ = store.reload();
    Ok(store
        .get(GOALS_KEY)
        .and_then(|goals| serde_json::from_value(goals).ok())
        .unwrap_or_default())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn query_progress(app_handle: tauri::AppHandle) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    let goals = read_goals(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_progress(
            app_config.replay_path,
            app_config.main_player_name,
            goals,
        ))
    })
    .await
}

/// Saves the goals of the progress, by metric key, a goal of 0 is removed.
#[tauri::command(rename_all = "snake_case")]
pub async fn set_progress_goals(
    app_handle: tauri::AppHandle,
    goals: BTreeMap<String, f64>,
) -> Result<ApiResponse, SwarmyError> {
    let goals: BTreeMap<String, f64> = goals.into_iter().filter(|(_, goal)| *goal > 0.).collect();
    let store = StoreBuilder::new(&app_handle, "settings.json").build()?;
    let _ = store.reload();
    store.set(GOALS_KEY, serde_json::to_value(&goals)?);
    store.save()?;
    Ok(ApiResponseBuilder::new()
        .with_success()
        .with_message("Goals saved".to_string())
        .build())
}

#[cfg(test)]
mod snapshot_tests {
    /// `SWARMY_TEST_REPLAY_PATH=~/SC2Replays SWARMY_TEST_PLAYER=Sazed cargo test progress_of -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn progress_of_a_player() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let player = std::env::var("SWARMY_TEST_PLAYER").unwrap();
        let start = std::time::Instant::now();
        let report = super::try_query_progress(replay_path, player, Default::default()).unwrap();
        eprintln!("{:?} {} games", start.elapsed(), report.games.len());
        for game in report.games.iter().rev().take(5) {
            eprintln!("{game:?}");
        }
        assert!(!report.games.is_empty());
    }
}
