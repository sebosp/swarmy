//! The replay before or after the current one by date, of the locked map and players, so that a
//! caster can go through the games of a series.

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub fn try_adjacent_replay(
    replay_path: String,
    query: AdjacentReplayQuery,
) -> Result<Option<u64>, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let games = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .group_by([col("ext_fs_id")])
        .agg([
            col("title").first(),
            col("ext_datetime").max(),
            col("player_name").alias("player_names"),
        ]);
    let mut locked = games.clone();
    if let Some(map_title) = &query.map_title {
        locked = locked.filter(col("title").eq(lit(map_title.clone())));
    }
    for player_name in &query.player_names {
        locked = locked.filter(
            col("player_names")
                .list()
                .contains(lit(player_name.clone()), false),
        );
    }
    let order = SortMultipleOptions::default();
    let locked = locked
        .select([col("ext_fs_id"), col("ext_datetime")])
        .sort(["ext_datetime", "ext_fs_id"], order)
        .collect()?;
    let ids = u64_values(&locked, "ext_fs_id")?;
    let current = match (query.step, query.ext_fs_id) {
        (ReplayStep::Latest, _) | (_, None) => return Ok(ids.last().copied()),
        (_, Some(ext_fs_id)) => ext_fs_id,
    };
    // The current replay may not be of the locked map and players, its position is found by date.
    let dates = locked
        .column("ext_datetime")?
        .cast(&DataType::Int64)?
        .i64()?
        .iter()
        .map(|date| date.unwrap_or_default())
        .collect::<Vec<_>>();
    let current_date = games
        .filter(col("ext_fs_id").eq(lit(current)))
        .select([col("ext_datetime").cast(DataType::Int64)])
        .collect()?
        .column("ext_datetime")?
        .i64()?
        .get(0)
        .ok_or_else(|| SwarmyError::Other(format!("Replay {current} not in the snapshot")))?;
    let position = |date: i64, id: u64| (date, id);
    let current_position = position(current_date, current);
    let positions = dates.into_iter().zip(ids);
    Ok(match query.step {
        ReplayStep::Next => positions
            .filter(|(date, id)| position(*date, *id) > current_position)
            .map(|(_, id)| id)
            .next(),
        ReplayStep::Previous => positions
            .rev()
            .find(|(date, id)| position(*date, *id) < current_position)
            .map(|(_, id)| id),
        ReplayStep::Latest => None,
    })
}
