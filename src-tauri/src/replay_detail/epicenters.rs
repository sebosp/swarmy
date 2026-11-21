#![doc = include_str!("epicenters/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The table name of the [`engagement_windows`] frame, for the SQL of the README.
pub const ENGAGEMENTS_IPC: &str = "engagements.arrow";

/// The engagements of the replay (`replay_detail::data`) as a frame: `engagement` (u32, the
/// position in the list), `start_seconds` and `end_seconds` (u32). A frame parameter of
/// [`plan`].
pub fn engagement_windows(engagements: &[Engagement]) -> Result<DataFrame, SwarmyError> {
    let positions: Vec<u32> = (0..engagements.len() as u32).collect();
    let starts: Vec<u32> = engagements.iter().map(|e| e.start_seconds).collect();
    let ends: Vec<u32> = engagements.iter().map(|e| e.end_seconds).collect();
    Ok(df!(
        "engagement" => positions,
        "start_seconds" => starts,
        "end_seconds" => ends,
    )?)
}

/// Where the engagements of the game happened: for each engagement with deaths in the snapshot,
/// `engagement` (u32), `x` and `y` (f32, the mean position of the deaths of its seconds, in map
/// units), `radius` (f32, the root mean square distance of the deaths to that center) and
/// `deaths` (u32); by engagement. `valued_units` is `replay_detail::valued_units::valued_units`:
/// the deaths that cost resources and are losses (`cause` killed or own_player), a Drone
/// morphing into a structure does not count.
pub fn plan(
    source: &dyn SnapshotSource,
    ext_fs_id: u64,
    windows: DataFrame,
    valued_units: DataFrame,
) -> Result<LazyFrame, SwarmyError> {
    let deaths = source
        .table(UNIT_DIED_IPC)?
        .filter(is_in_replays(&[ext_fs_id]).and(is_loss_row()))
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
        ]);
    let center = |axis: &str| col(axis).mean();
    Ok(windows
        .lazy()
        .cross_join(deaths, None)
        .filter(
            col("game_seconds")
                .gt_eq(col("start_seconds"))
                .and(col("game_seconds").lt_eq(col("end_seconds"))),
        )
        .group_by([col("engagement")])
        .agg([
            center("x").cast(DataType::Float32).alias("x"),
            center("y").cast(DataType::Float32).alias("y"),
            ((col("x") - center("x")).pow(2) + (col("y") - center("y")).pow(2))
                .mean()
                .sqrt()
                .cast(DataType::Float32)
                .alias("radius"),
            len().alias("deaths"),
        ])
        .sort_by_exprs([col("engagement")], Default::default()))
}

/// The epicenters by the position of their engagement in the list.
pub fn shape(frame: DataFrame) -> Result<Vec<(usize, Epicenter)>, SwarmyError> {
    Ok(u32_values(&frame, "engagement")?
        .into_iter()
        .zip(f32_values(&frame, "x")?)
        .zip(f32_values(&frame, "y")?)
        .zip(f32_values(&frame, "radius")?)
        .zip(u32_values(&frame, "deaths")?)
        .map(|((((engagement, x), y), radius), deaths)| {
            (
                engagement as usize,
                Epicenter {
                    x,
                    y,
                    radius,
                    deaths,
                },
            )
        })
        .collect())
}

#[cfg(test)]
mod tests;
