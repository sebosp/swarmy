//! The players of the snapshot, by account: an account (`region-realm-id` of the toon handle of
//! the replays) changes its name and clan tag, it is one player. Accounts grouped under a player
//! in the personal database (Clem's accounts of each region and of the tournaments) are that
//! player.

use polars::prelude::*;
use std::collections::HashMap;
use swarmy_common::*;

/// The `player_name` column without the clan tag.
pub fn name_without_clan_expr() -> Expr {
    col("player_name").str().split(lit("<sp/>")).list().last()
}

/// The account of a player row, `region-realm-id`, null for the computers (id 0).
pub fn account_expr() -> Expr {
    when(col("player_toon_id").eq(lit(0u64)))
        .then(lit(NULL).cast(DataType::String))
        .otherwise(concat_str(
            [
                col("player_toon_region").cast(DataType::String),
                lit("-"),
                col("player_toon_realm").cast(DataType::String),
                lit("-"),
                col("player_toon_id").cast(DataType::String),
            ],
            "",
            true,
        ))
}

/// The players of the details: the rows of the players (not the observers) with `account`, `name`
/// (without the clan tag) and `player`: the player the account is grouped under, or the name the
/// account played the most games with, or the name for the rows without an account. The query
/// file is `player_detail/player_rows.rs`, this is for the callers that have a path.
pub fn details_with_players(
    ipc_path: &str,
    groups: &HashMap<String, String>,
) -> Result<LazyFrame, SwarmyError> {
    crate::player_detail::player_rows::plan(&super::IpcSnapshot::at(ipc_path), groups)
}

/// The player of a name: the name itself when it is a player, else the player of the accounts
/// that played the most games with it (i.e. "LiquidClem" is Clem when its account is grouped under
/// Clem), else the name.
pub fn resolve_player(players: &LazyFrame, name: &str) -> Result<String, SwarmyError> {
    let is_player = players
        .clone()
        .filter(col("player").eq(lit(name)))
        .select([len()])
        .collect()?
        .column("len")?
        .u32()?
        .get(0)
        .unwrap_or(0)
        > 0;
    if is_player {
        return Ok(name.to_string());
    }
    let of_name = players
        .clone()
        .filter(col("name").eq(lit(name)))
        .group_by([col("player")])
        .agg([len().alias("rows")])
        .sort(
            ["rows"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .limit(1)
        .collect()?;
    Ok(of_name
        .column("player")?
        .str()?
        .get(0)
        .unwrap_or(name)
        .to_string())
}
