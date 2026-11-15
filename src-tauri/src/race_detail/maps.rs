#![doc = include_str!("maps/README.md")]

use super::one_vs_one;
use crate::data::*;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The non-mirror 1v1 games of the race by map and matchup: `title`, `matchup`, `games` (u32) and
/// `wins` (Int32), by map and matchup.
pub fn plan(source: &dyn SnapshotSource, race: &str) -> Result<LazyFrame, SwarmyError> {
    Ok(one_vs_one(source, race)?
        .filter(super::not_mirror(race))
        .group_by([col("title"), super::matchup_expr(race)])
        .agg(super::games_wins())
        .sort(["title", "matchup"], Default::default()))
}

/// The maps with at least [`RACE_MAP_MIN_GAMES`] games, the best win rate first.
pub fn shape(frame: DataFrame) -> Result<Vec<RaceMapStats>, SwarmyError> {
    let mut by_map: BTreeMap<String, Vec<GamesWins>> = BTreeMap::new();
    for (title, matchup) in str_values(&frame, "title")?
        .into_iter()
        .zip(super::games_wins_rows(&frame, "matchup")?)
    {
        by_map.entry(title).or_default().push(matchup);
    }
    let mut maps: Vec<RaceMapStats> = by_map
        .into_iter()
        .map(|(map_title, matchups)| RaceMapStats {
            map_title,
            matchups,
        })
        .filter(|map| map.games() >= RACE_MAP_MIN_GAMES)
        .collect();
    maps.sort_by(|a, b| {
        b.win_percent()
            .total_cmp(&a.win_percent())
            .then(b.games().cmp(&a.games()))
    });
    Ok(maps)
}

#[cfg(test)]
mod tests;
