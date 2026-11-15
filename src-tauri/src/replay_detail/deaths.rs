#![doc = include_str!("deaths/README.md")]

use crate::data::*;
use crate::replay_rerun::decisions::MORPH_SOURCES;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// Where and when the units of the game died, the ones that cost resources: `x`, `y` (f32, map
/// units) and `game_seconds` (u32); by time and position. `valued_units` is
/// `replay_detail::valued_units::valued_units`: a Larva or a Broodling dying is not a loss, nor a
/// Drone morphing into a structure.
pub fn plan(
    source: &dyn SnapshotSource,
    ext_fs_id: u64,
    valued_units: DataFrame,
) -> Result<LazyFrame, SwarmyError> {
    let morph_sources = Series::new("morph_sources".into(), MORPH_SOURCES);
    Ok(source
        .table(UNIT_DIED_IPC)?
        .filter(
            is_in_replays(&[ext_fs_id]).and(
                col("unit_died_name")
                    .is_in(lit(morph_sources).implode(false), false)
                    .and(col("killer_unit_tag_index").is_null())
                    .not(),
            ),
        )
        .join(
            valued_units.lazy().select([col("variant")]),
            [col("unit_died_name")],
            [col("variant")],
            JoinArgs::new(JoinType::Inner),
        )
        .select([
            col("x").cast(DataType::Float32),
            col("y").cast(DataType::Float32),
            game_seconds(),
        ])
        .sort_by_exprs(
            [col("game_seconds"), col("x"), col("y")],
            Default::default(),
        ))
}

/// The deaths as the map of the view draws them.
pub fn shape(frame: DataFrame) -> Result<Vec<MapDeath>, SwarmyError> {
    Ok(f32_values(&frame, "x")?
        .into_iter()
        .zip(f32_values(&frame, "y")?)
        .zip(u32_values(&frame, "game_seconds")?)
        .map(|((x, y), seconds)| MapDeath { x, y, seconds })
        .collect())
}

#[cfg(test)]
mod tests;
