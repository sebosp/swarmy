#![doc = include_str!("expansion_averages/README.md")]

use super::upgrade_averages::averages;
use crate::data::*;
use crate::replay_detail::data::EXPANSION_TOWNHALLS;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The average game second the nth expansion of each townhall type finishes in the games, 1 being
/// the first townhall after the initial one: `unit_type_name`, `number` (u32), `game_seconds`
/// (u32) and `samples` (u32), with at least `MIN_SAMPLES`; by type and number.
pub fn plan(source: &dyn SnapshotSource, game_ids: &[u64]) -> Result<LazyFrame, SwarmyError> {
    let townhalls = Series::new("townhalls".into(), EXPANSION_TOWNHALLS);
    let born = source
        .table(UNIT_BORN_IPC)?
        .filter(
            is_in_replays(game_ids)
                .and(col("ext_replay_loop").gt(lit(0i64)))
                .and(col("unit_type_name").is_in(lit(townhalls).implode(false), false)),
        )
        .select([
            col("ext_fs_id"),
            col("control_player_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            col("unit_type_name"),
            game_seconds(),
        ]);
    Ok(
        averages_of(born)?
            .sort_by_exprs([col("unit_type_name"), col("number")], Default::default()),
    )
}

/// The averages from the townhalls born (`ext_fs_id`, `control_player_id`, `unit_tag_index`,
/// `unit_tag_recycle`, `unit_type_name`, `game_seconds`): the first time each townhall appears is
/// when it finished, a Command Center landing again is not counted; the townhalls of a player are
/// numbered by that time.
pub fn averages_of(townhalls: LazyFrame) -> Result<LazyFrame, SwarmyError> {
    let finished = townhalls
        .group_by([
            col("ext_fs_id"),
            col("control_player_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
        ])
        .agg([col("unit_type_name").first(), col("game_seconds").min()])
        .sort_by_exprs(
            [
                col("ext_fs_id"),
                col("control_player_id"),
                col("game_seconds"),
                col("unit_tag_index"),
            ],
            Default::default(),
        )
        .with_column(
            (int_range(lit(0), len(), 1, DataType::UInt32) + lit(1u32))
                .over([col("ext_fs_id"), col("control_player_id")])?
                .alias("number"),
        );
    Ok(averages(finished, &["unit_type_name", "number"]))
}

/// By townhall type and number: the average finish in game seconds and the samples.
pub type ExpansionAverages = std::collections::HashMap<(String, usize), (u32, usize)>;

/// By townhall type and number.
pub fn shape(frame: DataFrame) -> Result<ExpansionAverages, SwarmyError> {
    Ok(str_values(&frame, "unit_type_name")?
        .into_iter()
        .zip(u32_values(&frame, "number")?)
        .zip(u32_values(&frame, "game_seconds")?)
        .zip(u32_values(&frame, "samples")?)
        .map(|(((name, number), finished), samples)| {
            ((name, number as usize), (finished, samples as usize))
        })
        .collect())
}

#[cfg(test)]
mod tests;
