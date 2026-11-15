#![doc = include_str!("protocol_versions/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The protocol versions of the games of the snapshot: `ext_protocol_version` and `games` (u32),
/// the newest version first.
pub fn plan(source: &dyn SnapshotSource) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .table(DETAILS_IPC)?
        .group_by([col("ext_protocol_version")])
        .agg([col("ext_fs_id").n_unique().alias("games")])
        .sort(
            ["ext_protocol_version"],
            SortMultipleOptions::default().with_order_descending(true),
        ))
}

/// The versions as [`GamesWins`] for the dropdown, the label is the version.
pub fn shape(frame: DataFrame) -> Result<Vec<GamesWins>, SwarmyError> {
    Ok(u32_values(&frame, "ext_protocol_version")?
        .into_iter()
        .zip(u32_values(&frame, "games")?)
        .map(|(version, games)| GamesWins {
            label: version.to_string(),
            games: games as usize,
            wins: 0,
        })
        .collect())
}

#[cfg(test)]
mod tests;
