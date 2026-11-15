#![doc = include_str!("maps/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The maps of the snapshot: `title` and `games`, the most played first.
pub fn plan(source: &dyn SnapshotSource) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .table(DETAILS_IPC)?
        .group_by([col("title")])
        .agg([col("ext_fs_id").n_unique().alias("games")])
        .sort_by_exprs(
            [col("games"), col("title")],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        ))
}

/// The maps as [`GamesWins`] for the map field, no wins.
pub fn shape(frame: DataFrame) -> Result<Vec<GamesWins>, SwarmyError> {
    Ok(str_values(&frame, "title")?
        .into_iter()
        .zip(u32_values(&frame, "games")?)
        .map(|(label, games)| GamesWins {
            label,
            games: games as usize,
            wins: 0,
        })
        .collect())
}

#[cfg(test)]
mod tests;
