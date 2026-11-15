#![doc = include_str!("upgrade_averages/README.md")]

use crate::data::*;
use crate::replay_detail::data::cosmetic_upgrade_expr;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The averages with fewer samples are not shown.
pub const MIN_SAMPLES: usize = 3;

/// The average game second each upgrade finishes in the games: `name`, `game_seconds` (u32) and
/// `samples` (u32), the upgrades finished at least [`MIN_SAMPLES`] times; by name.
pub fn plan(source: &dyn SnapshotSource, game_ids: &[u64]) -> Result<LazyFrame, SwarmyError> {
    let upgrades = source
        .table(UPGRADES_IPC)?
        .filter(
            is_in_replays(game_ids)
                .and(col("ext_replay_loop").gt(lit(0i64)))
                .and(cosmetic_upgrade_expr("name").not()),
        )
        .select([col("name"), game_seconds()]);
    Ok(averages(upgrades, &["name"]).sort(["name"], Default::default()))
}

/// The average seconds and the samples of each group, the groups with fewer than [`MIN_SAMPLES`]
/// are left out.
pub fn averages(query: LazyFrame, by: &[&str]) -> LazyFrame {
    query
        .group_by(by.iter().map(|name| col(*name)).collect::<Vec<_>>())
        .agg([
            col("game_seconds").mean().cast(DataType::UInt32),
            len().alias("samples"),
        ])
        .filter(col("samples").gt_eq(lit(MIN_SAMPLES as u32)))
}

/// By upgrade name.
pub fn shape(
    frame: DataFrame,
) -> Result<std::collections::HashMap<String, PatchAverage>, SwarmyError> {
    Ok(str_values(&frame, "name")?
        .into_iter()
        .zip(u32_values(&frame, "game_seconds")?)
        .zip(u32_values(&frame, "samples")?)
        .map(|((name, seconds), samples)| {
            (
                name,
                PatchAverage {
                    seconds,
                    samples: samples as usize,
                },
            )
        })
        .collect())
}

#[cfg(test)]
mod tests;
