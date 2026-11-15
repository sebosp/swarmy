#![doc = include_str!("bases/README.md")]

use crate::data::*;
use crate::replay_detail::data::EXPANSION_TOWNHALLS;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The townhalls the players of the game made, the first one of each player at game loop 0:
/// `player_name`, `unit_type_name`, `x`, `y` (f32, map units) and `game_seconds` (u32); a
/// townhall born again (a Command Center landing) counts once, when it was first born; by time,
/// player and position.
pub fn plan(source: &dyn SnapshotSource, ext_fs_id: u64) -> Result<LazyFrame, SwarmyError> {
    let townhalls = Series::new("townhalls".into(), EXPANSION_TOWNHALLS);
    Ok(source
        .table(UNIT_BORN_IPC)?
        .filter(
            is_in_replays(&[ext_fs_id])
                .and(col("player_name").is_not_null())
                .and(col("unit_type_name").is_in(lit(townhalls).implode(false), false)),
        )
        .sort_by_exprs(
            [col("ext_replay_loop"), col("unit_tag_index")],
            Default::default(),
        )
        .with_column(
            (int_range(lit(0), len(), 1, DataType::UInt32) + lit(1u32))
                .over([col("unit_tag_index"), col("unit_tag_recycle")])?
                .alias("birth"),
        )
        .filter(col("birth").eq(lit(1u32)))
        .select([
            col("player_name"),
            col("unit_type_name"),
            col("x").cast(DataType::Float32),
            col("y").cast(DataType::Float32),
            game_seconds(),
        ])
        .sort_by_exprs(
            [col("game_seconds"), col("player_name"), col("x"), col("y")],
            Default::default(),
        ))
}

/// The bases as the view draws them.
pub fn shape(frame: DataFrame) -> Result<Vec<PlayerBase>, SwarmyError> {
    Ok(str_values(&frame, "player_name")?
        .into_iter()
        .zip(str_values(&frame, "unit_type_name")?)
        .zip(f32_values(&frame, "x")?)
        .zip(f32_values(&frame, "y")?)
        .zip(u32_values(&frame, "game_seconds")?)
        .map(|((((player, unit_type), x), y), seconds)| PlayerBase {
            player,
            unit_type,
            x,
            y,
            seconds,
        })
        .collect())
}

#[cfg(test)]
mod tests;
