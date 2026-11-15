#![doc = include_str!("maps/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The maps shown at most, the most played first.
pub const MAP_LIST_LIMIT: u32 = 1000;

/// The maps of the snapshot with their games, dates and cache handles, the most played first:
/// `title`, `map_info_sha256`, `min_date`, `max_date` (`YYYY-MM-DD`), `num_games` (u32) and
/// `cache_handles`. The map title and the player name of the query filter the games, by text
/// contained (ignoring case).
pub fn plan(source: &dyn SnapshotSource, query: &MapStatsQuery) -> Result<LazyFrame, SwarmyError> {
    // The observers watched the game, they are not its players.
    let mut details = source
        .table(DETAILS_IPC)?
        .filter(col("player_observe").eq(lit(0u8)));
    if !query.map_title.is_empty() {
        details = details.filter(
            col("title")
                .str()
                .to_lowercase()
                .str()
                .contains_literal(lit(query.map_title.to_lowercase())),
        );
    }
    if !query.player_name.is_empty() {
        details = details.filter(
            col("player_name")
                .str()
                .to_lowercase()
                .str()
                .contains_literal(lit(query.player_name.to_lowercase())),
        );
    }
    // One row per game: the map, its sha256 and cache handles are the same for every player. The
    // newest game first: its cache handles are the ones kept, the handles of the games of a map
    // differ (the mods and skins loaded) and the newest draw it best, as in Map Details.
    let games = details
        .group_by([col("ext_fs_id")])
        .agg([
            col("title").first(),
            col("map_info_sha256").first(),
            col("ext_datetime").min().alias("played_at"),
            col("cache_handles").first(),
        ])
        .sort_by_exprs(
            [col("played_at"), col("ext_fs_id")],
            SortMultipleOptions::default().with_order_descending_multi([true, true]),
        );
    Ok(games
        .group_by_stable([col("title"), col("map_info_sha256")])
        .agg([
            col("played_at")
                .min()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("min_date"),
            col("played_at")
                .max()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("max_date"),
            len().alias("num_games"),
            col("cache_handles").first(),
        ])
        .sort_by_exprs(
            [col("num_games"), col("title"), col("map_info_sha256")],
            SortMultipleOptions::default().with_order_descending_multi([true, false, false]),
        )
        .limit(MAP_LIST_LIMIT))
}

pub fn shape(frame: DataFrame) -> Result<Vec<MapStats>, SwarmyError> {
    let titles = str_values(&frame, "title")?;
    let shas = str_values(&frame, "map_info_sha256")?;
    let min_dates = str_values(&frame, "min_date")?;
    let max_dates = str_values(&frame, "max_date")?;
    let games = u32_values(&frame, "num_games")?;
    let handles = str_values(&frame, "cache_handles")?;
    let date = |value: &str| {
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|err| SwarmyError::Other(format!("date {value:?}: {err}")))
    };
    (0..frame.height())
        .map(|row| {
            Ok(MapStats {
                title: titles[row].clone(),
                map_info_sha256: shas[row].clone(),
                min_date: date(&min_dates[row])?,
                max_date: date(&max_dates[row])?,
                num_games: games[row],
                cache_handles: handles[row].clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
