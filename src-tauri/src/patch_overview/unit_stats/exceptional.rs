#![doc = include_str!("exceptional/README.md")]

use super::{games, kills_per_unit};
use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The player games at or above the `threshold` of kills per unit with a kill, with their game:
/// `ext_fs_id`, `player_name`, `born`, `kills`, `value_killed`, `kills_per_unit`, `date`,
/// `title`, `ext_fs_file_name` and `ext_protocol_version`; the best first, at most
/// [`EXCEPTIONAL_GAMES_LIMIT`].
pub fn plan(
    source: &dyn SnapshotSource,
    unit: &str,
    min_born: usize,
    threshold: f64,
) -> Result<LazyFrame, SwarmyError> {
    Ok(kills_per_unit::plan(source, unit, min_born)?
        .filter(
            col("kills_per_unit")
                .gt_eq(lit(threshold))
                .and(col("kills").gt(lit(0u32))),
        )
        .join(
            games::plan(source)?.select([
                col("ext_fs_id"),
                col("date"),
                col("title"),
                col("ext_fs_file_name"),
                col("ext_protocol_version"),
            ]),
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .sort_by_exprs(
            [
                col("kills_per_unit"),
                col("kills"),
                col("ext_fs_id"),
                col("player_name"),
            ],
            SortMultipleOptions::default().with_order_descending_multi([true, true, false, false]),
        )
        .limit(EXCEPTIONAL_GAMES_LIMIT as IdxSize))
}

pub fn shape(frame: DataFrame) -> Result<Vec<ExceptionalUnitGame>, SwarmyError> {
    let ids = u64_values(&frame, "ext_fs_id")?;
    let locations = str_values(&frame, "ext_fs_file_name")?;
    let titles = str_values(&frame, "title")?;
    let dates = str_values(&frame, "date")?;
    let versions = u32_values(&frame, "ext_protocol_version")?;
    let names = str_values(&frame, "player_name")?;
    let born = u32_values(&frame, "born")?;
    let kills = u32_values(&frame, "kills")?;
    let values = u64_values(&frame, "value_killed")?;
    Ok((0..frame.height())
        .map(|idx| ExceptionalUnitGame {
            ext_fs_id: ids[idx],
            replay_location: locations[idx].clone(),
            map_title: titles[idx].clone(),
            date: dates[idx].clone(),
            protocol_version: versions[idx],
            player_name: names[idx].clone(),
            born: born[idx] as usize,
            kills: kills[idx] as usize,
            value_killed: values[idx] as u32,
        })
        .collect())
}

#[cfg(test)]
mod tests;
