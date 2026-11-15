#![doc = include_str!("games/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The last games of the player shown.
pub const PROGRESS_GAMES: u32 = 200;

/// The control of the computer players in the details.
pub const COMPUTER_CONTROL: u8 = 3;

/// The last [`PROGRESS_GAMES`] games of the player (by the name without the clan tag), the oldest
/// first: `ext_fs_id`, `player_id` (the tracker id, for the stats), `team_id`, `race`, `result`,
/// `title`, `date` (`YYYY-MM-DD HH:MM`), `opponents` (list of names) and `computers` (Int32, the
/// computer players among the opponents).
pub fn plan(source: &dyn SnapshotSource, player: &str) -> Result<LazyFrame, SwarmyError> {
    let details = source
        .table(DETAILS_IPC)?
        .filter(col("player_observe").eq(lit(0u8)))
        .with_column(name_without_clan_expr().alias("name"));
    let ids = source
        .table(GAME_PLAYERS_IPC)?
        .select([col("ext_fs_id"), col("player_name"), col("player_id")])
        .unique(
            Some(cols(["ext_fs_id", "player_name"])),
            UniqueKeepStrategy::Any,
        );
    // The newest games of the player, one row each (two accounts in one game: the first by name).
    let own = details
        .clone()
        .filter(col("name").eq(lit(player)))
        .sort_by_exprs(
            [col("ext_fs_id"), col("player_name")],
            SortMultipleOptions::default(),
        )
        .unique_stable(Some(cols(["ext_fs_id"])), UniqueKeepStrategy::First)
        .sort_by_exprs(
            [col("ext_datetime"), col("ext_fs_id")],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .limit(PROGRESS_GAMES)
        .join(
            ids,
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Left),
        )
        .select([
            col("ext_fs_id"),
            col("player_id"),
            col("player_team_id").alias("team_id"),
            race_name_expr("player_race").alias("race"),
            col("player_result").alias("result"),
            col("title"),
            col("ext_datetime")
                .dt()
                .to_string("%Y-%m-%d %H:%M")
                .alias("date"),
        ]);
    let opponents = details
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("player_team_id"),
            col("player_control"),
        ])
        .join(
            own.clone().select([col("ext_fs_id"), col("team_id")]),
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("player_team_id").neq(col("team_id")))
        .group_by([col("ext_fs_id")])
        .agg([
            col("player_name").alias("opponents"),
            col("player_control")
                .eq(lit(COMPUTER_CONTROL))
                .sum()
                .cast(DataType::Int32)
                .alias("computers"),
        ]);
    Ok(own
        .join(
            opponents,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .sort_by_exprs(
            [col("date"), col("ext_fs_id")],
            SortMultipleOptions::default(),
        ))
}

/// A game of the player with its opponents, without the metrics.
pub fn shape(frame: DataFrame) -> Result<Vec<ProgressGame>, SwarmyError> {
    let ids = u64_values(&frame, "ext_fs_id")?;
    let dates = str_values(&frame, "date")?;
    let titles = str_values(&frame, "title")?;
    let races = str_values(&frame, "race")?;
    let results = str_values(&frame, "result")?;
    let computers: Vec<Option<i32>> = frame.column("computers")?.i32()?.iter().collect();
    let opponents = frame.column("opponents")?.list()?.clone();
    Ok((0..frame.height())
        .map(|row| ProgressGame {
            ext_fs_id: ids[row],
            date: dates[row].clone(),
            map_title: titles[row].clone(),
            race: races[row].clone(),
            result: results[row].clone(),
            opponents: opponents
                .get_as_series(row)
                .and_then(|names| {
                    names
                        .str()
                        .ok()
                        .map(|names| names.iter().flatten().map(str::to_string).collect())
                })
                .unwrap_or_default(),
            vs_computer: computers[row].unwrap_or_default() > 0,
            values: Default::default(),
        })
        .collect())
}

#[cfg(test)]
mod tests;
