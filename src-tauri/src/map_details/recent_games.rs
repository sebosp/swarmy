#![doc = include_str!("recent_games/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The games shown, by date.
pub const RECENT_GAMES: u32 = 20;

/// The players of the newest games on a map, a row per player: `ext_fs_id`, `date`,
/// `player_team_id`, `player_name`, `player_result`, `player_race`, `file_name`, `sha256`,
/// `game_seconds`; the newest game first, within a game the teams then the names.
pub fn plan(source: &dyn SnapshotSource, map_title: &str) -> Result<LazyFrame, SwarmyError> {
    let newest = source
        .table(DETAILS_IPC)?
        .filter(col("title").eq(lit(map_title)))
        .group_by([col("ext_fs_id")])
        .agg([col("ext_datetime").min().alias("played_at")])
        .sort_by_exprs(
            [col("played_at"), col("ext_fs_id")],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .limit(RECENT_GAMES);
    let durations = source
        .table(GAME_PLAYERS_IPC)?
        .group_by([col("ext_fs_id")])
        .agg([col("game_seconds").min()]);
    Ok(source
        .table(DETAILS_IPC)?
        .filter(col("player_observe").eq(lit(0u8)))
        .join(
            newest,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            durations,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .select([
            col("ext_fs_id"),
            col("played_at").dt().to_string("%Y-%m-%d").alias("date"),
            col("player_team_id"),
            col("player_name"),
            col("player_result"),
            col("player_race"),
            col("ext_fs_file_name").alias("file_name"),
            col("ext_fs_sha256").alias("sha256"),
            col("game_seconds"),
        ])
        .sort_by_exprs(
            [
                col("date"),
                col("ext_fs_id"),
                col("player_team_id"),
                col("player_name"),
            ],
            SortMultipleOptions::default().with_order_descending_multi([true, true, false, false]),
        ))
}

/// The rows grouped into games, a [`ReplayListEntry`] per game with its players in the order of
/// the rows; the personal fields (rating, tags, notes) are empty, the view does not show them.
pub fn shape(frame: DataFrame, map_title: &str) -> Result<Vec<ReplayListEntry>, SwarmyError> {
    let ids = u64_values(&frame, "ext_fs_id")?;
    let dates = str_values(&frame, "date")?;
    let names = str_values(&frame, "player_name")?;
    let results = str_values(&frame, "player_result")?;
    let races = str_values(&frame, "player_race")?;
    let files = str_values(&frame, "file_name")?;
    let shas = str_values(&frame, "sha256")?;
    let seconds: Vec<Option<u32>> = frame.column("game_seconds")?.u32()?.iter().collect();
    let mut games: Vec<ReplayListEntry> = vec![];
    for row in 0..frame.height() {
        if games.last().is_none_or(|game| game.ext_fs_id != ids[row]) {
            let file = std::path::Path::new(&files[row]);
            games.push(ReplayListEntry {
                ext_fs_id: ids[row],
                map_title: map_title.to_string(),
                replay_date: chrono::NaiveDate::parse_from_str(&dates[row], "%Y-%m-%d")
                    .map_err(|err| SwarmyError::Other(format!("date {:?}: {err}", dates[row])))?,
                replay_location: files[row].clone(),
                replay_file_name: file
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
                sha256_sum: shas[row].clone(),
                replay_sha256: shas[row].clone(),
                duration: seconds[row].unwrap_or_default() as i32,
                ..Default::default()
            });
        }
        let game = games.last_mut().expect("pushed above");
        game.player_list.push(names[row].clone());
        game.player_results.push(results[row].clone());
        game.player_races.push(normalize_race(&races[row]));
    }
    Ok(games)
}

#[cfg(test)]
mod tests;
