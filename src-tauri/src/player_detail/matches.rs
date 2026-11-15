#![doc = include_str!("matches/README.md")]

use super::player_rows;
use crate::data::*;
use polars::prelude::*;
use std::collections::HashMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The games of the player against each opponent, a row per opponent: `ext_fs_id`, `date`,
/// `map_title`, `race`, `result`, `protocol_version`, `played_as`, `opponent`, `opponent_race`.
/// The newest first.
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    player: &str,
) -> Result<LazyFrame, SwarmyError> {
    let rows = player_rows::plan(source, groups)?;
    let mine = rows
        .clone()
        .filter(col("player").eq(lit(player)))
        .sort_by_exprs(
            [col("ext_fs_id"), col("player_name")],
            SortMultipleOptions::default(),
        )
        .unique_stable(Some(cols(["ext_fs_id"])), UniqueKeepStrategy::First)
        .select([
            col("ext_fs_id"),
            col("ext_datetime").dt().to_string("%Y-%m-%d").alias("date"),
            col("title").alias("map_title"),
            race_name_expr("player_race").alias("race"),
            col("player_result").alias("result"),
            col("ext_protocol_version").alias("protocol_version"),
            col("name").alias("played_as"),
            col("player_team_id"),
        ]);
    let others = rows.filter(col("player").neq(lit(player))).select([
        col("ext_fs_id"),
        col("player").alias("opponent"),
        race_name_expr("player_race").alias("opponent_race"),
        col("player_team_id").alias("opponent_team_id"),
    ]);
    Ok(mine
        .join(
            others,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        // Teammates are not opponents; a game without another player keeps its row.
        .filter(
            col("opponent_team_id")
                .is_null()
                .or(col("opponent_team_id").neq(col("player_team_id"))),
        )
        .select([
            col("ext_fs_id"),
            col("date"),
            col("map_title"),
            col("race"),
            col("result"),
            col("protocol_version"),
            col("played_as"),
            col("opponent"),
            col("opponent_race"),
        ])
        .sort_by_exprs(
            [col("date"), col("ext_fs_id"), col("opponent")],
            SortMultipleOptions::default().with_order_descending_multi([true, true, false]),
        ))
}

pub fn shape(frame: DataFrame) -> Result<Vec<PlayerMatch>, SwarmyError> {
    let ids = u64_values(&frame, "ext_fs_id")?;
    let dates = str_values(&frame, "date")?;
    let maps = str_values(&frame, "map_title")?;
    let races = str_values(&frame, "race")?;
    let results = str_values(&frame, "result")?;
    let versions = u32_values(&frame, "protocol_version")?;
    let played_as = str_values(&frame, "played_as")?;
    let opponents = str_values(&frame, "opponent")?;
    let opponent_races = str_values(&frame, "opponent_race")?;
    Ok((0..frame.height())
        .map(|row| PlayerMatch {
            ext_fs_id: ids[row],
            date: dates[row].clone(),
            map_title: maps[row].clone(),
            race: races[row].clone(),
            opponent: opponents[row].clone(),
            opponent_race: opponent_races[row].clone(),
            won: results[row] == "Win",
            tie: results[row] == "Tie",
            played_as: played_as[row].clone(),
            protocol_version: versions[row],
        })
        .collect())
}

#[cfg(test)]
mod tests;
