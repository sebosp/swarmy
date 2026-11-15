#![doc = include_str!("metrics/README.md")]

use super::games;
use crate::data::*;
use crate::replay_rerun::viewer::GAME_LOOPS_PER_SECOND;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The seconds between two stats of a player, 160 game loops.
pub const STATS_INTERVAL_SECONDS: f64 = 160. / GAME_LOOPS_PER_SECOND;

/// The stats of a minute are the last ones of the seconds before it.
pub const STATS_WINDOW_SECONDS: f64 = 15.;

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

/// The four numbers of each of the recent games of the player ([`games`]): `workers_6` (the
/// workers at 06:00), `income_8` (minerals and vespene per minute at 08:00), `army_10` (the
/// resources in the army at 10:00) and `supply_blocked` (the seconds supply blocked), all
/// Float64, by `ext_fs_id` and `player_id`.
pub fn plan(source: &dyn SnapshotSource, player: &str) -> Result<LazyFrame, SwarmyError> {
    let mine = games::plan(source, player)?
        .filter(col("player_id").is_not_null())
        .select([col("ext_fs_id"), col("player_id")]);
    let stats = source.table(STATS_IPC)?.join(
        mine.clone(),
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
    let mut metrics = mine;
    for values in measured {
        metrics = metrics.join(
            values,
            [col("ext_fs_id"), col("player_id")],
            [col("ext_fs_id"), col("player_id")],
            JoinArgs::new(JoinType::Left),
        );
    }
    Ok(metrics
        // A game without a supply block has no rows of it: 0 seconds.
        .with_column(col("supply_blocked").fill_null(lit(0.0)))
        .sort(["ext_fs_id"], Default::default()))
}

/// The metrics by game, as [`ProgressGame::values`] (the metric keys of `PROGRESS_METRICS`);
/// a game without stats at a minute has no value for it.
pub fn shape(frame: DataFrame) -> Result<BTreeMap<u64, BTreeMap<String, f64>>, SwarmyError> {
    let ids = u64_values(&frame, "ext_fs_id")?;
    let mut values = BTreeMap::new();
    for metric in PROGRESS_METRICS {
        let column = frame.column(metric.key)?.f64()?;
        for (row, value) in column.iter().enumerate() {
            if let Some(value) = value {
                values
                    .entry(ids[row])
                    .or_insert_with(BTreeMap::new)
                    .insert(metric.key.to_string(), value);
            }
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests;
