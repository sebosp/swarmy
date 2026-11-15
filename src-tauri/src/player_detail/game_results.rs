#![doc = include_str!("game_results/README.md")]

use super::player_rows;
use crate::data::*;
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap};
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The games of the player, one row each: `ext_fs_id`, `date` (`YYYY-MM-DD`), `month`
/// (`YYYY-MM`), `race`, `result`, `protocol_version` and `played_as` (the name of the account in
/// that game, without the clan tag). The oldest first.
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    player: &str,
) -> Result<LazyFrame, SwarmyError> {
    Ok(player_rows::plan(source, groups)?
        .filter(col("player").eq(lit(player)))
        // A player with two accounts in one game is one side of it, the first by name.
        .sort_by_exprs(
            [col("ext_fs_id"), col("player_name")],
            SortMultipleOptions::default(),
        )
        .unique_stable(Some(cols(["ext_fs_id"])), UniqueKeepStrategy::First)
        .select([
            col("ext_fs_id"),
            col("ext_datetime").dt().to_string("%Y-%m-%d").alias("date"),
            col("ext_datetime").dt().to_string("%Y-%m").alias("month"),
            race_name_expr("player_race").alias("race"),
            col("player_result").alias("result"),
            col("ext_protocol_version").alias("protocol_version"),
            col("name").alias("played_as"),
        ]))
}

/// The results of the Player view: the header counts and the games and wins by race, by month
/// and by protocol version.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameResults {
    pub games: usize,
    pub wins: usize,
    pub ties: usize,
    /// The most played race first.
    pub races: Vec<GamesWins>,
    /// By `YYYY-MM`, the oldest first.
    pub by_month: Vec<GamesWins>,
    /// By protocol version, the oldest first.
    pub by_version: Vec<GamesWins>,
}

/// Counts a game under a label.
pub fn add_game(counts: &mut BTreeMap<String, GamesWins>, label: String, won: bool) {
    let entry = counts.entry(label.clone()).or_insert_with(|| GamesWins {
        label,
        ..Default::default()
    });
    entry.games += 1;
    entry.wins += won as usize;
}

pub fn shape(frame: DataFrame) -> Result<GameResults, SwarmyError> {
    let races_column = str_values(&frame, "race")?;
    let months = str_values(&frame, "month")?;
    let results = str_values(&frame, "result")?;
    let versions = u32_values(&frame, "protocol_version")?;
    let mut shaped = GameResults {
        games: frame.height(),
        ..Default::default()
    };
    let mut races = BTreeMap::new();
    let mut by_month = BTreeMap::new();
    let mut by_version = BTreeMap::new();
    for row in 0..frame.height() {
        let won = results[row] == "Win";
        shaped.wins += won as usize;
        shaped.ties += (results[row] == "Tie") as usize;
        add_game(&mut races, races_column[row].clone(), won);
        add_game(&mut by_month, months[row].clone(), won);
        add_game(&mut by_version, versions[row].to_string(), won);
    }
    shaped.races = races.into_values().collect();
    shaped
        .races
        .sort_by_key(|race| std::cmp::Reverse(race.games));
    shaped.by_month = by_month.into_values().collect();
    shaped.by_version = by_version.into_values().collect();
    shaped
        .by_version
        .sort_by_key(|version| version.label.parse::<u32>().unwrap_or_default());
    Ok(shaped)
}

#[cfg(test)]
mod tests;
