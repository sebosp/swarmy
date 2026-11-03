//! The players of the snapshot, by account: an account (`region-realm-id` of the toon handle of
//! the replays) changes its name and clan tag, it is one player. Accounts grouped under a player
//! in the personal database (Clem's accounts of each region and of the tournaments) are that
//! player.

use super::scan_snapshot_ipc;
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
/// account played the most games with, or the name for the rows without an account.
pub fn details_with_players(
    ipc_path: &str,
    groups: &HashMap<String, String>,
) -> Result<LazyFrame, SwarmyError> {
    let details = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .filter(col("player_observe").eq(lit(0u8)))
        .with_columns([
            account_expr().alias("account"),
            name_without_clan_expr().alias("name"),
        ]);
    // The name of each account: the one of the most games.
    let names = details
        .clone()
        .filter(col("account").is_not_null())
        .group_by([col("account"), col("name")])
        .agg([col("ext_fs_id").n_unique().alias("games")])
        .group_by([col("account")])
        .agg([col("name")
            .sort_by(
                [col("games"), col("name")],
                SortMultipleOptions::default().with_order_descending_multi([true, false]),
            )
            .first()
            .alias("account_name")]);
    let (accounts, players): (Vec<String>, Vec<String>) = groups
        .iter()
        .map(|(account, player)| (account.clone(), player.clone()))
        .unzip();
    let grouped = DataFrame::new(
        accounts.len(),
        vec![
            Column::new("account".into(), accounts),
            Column::new("grouped_as".into(), players),
        ],
    )?
    .lazy();
    Ok(details
        .join(
            names,
            [col("account")],
            [col("account")],
            JoinArgs::new(JoinType::Left),
        )
        .join(
            grouped,
            [col("account")],
            [col("account")],
            JoinArgs::new(JoinType::Left),
        )
        .with_column(
            coalesce(&[col("grouped_as"), col("account_name"), col("name")]).alias("player"),
        ))
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

/// The accounts of the rows, with their names (the most played first), games and group.
pub fn accounts_of(
    rows: LazyFrame,
    groups: &HashMap<String, String>,
    limit: u32,
) -> Result<Vec<PlayerAccount>, SwarmyError> {
    let accounts = rows
        .filter(col("account").is_not_null())
        .group_by([col("account"), col("name")])
        .agg([col("ext_fs_id").n_unique().alias("name_games")])
        .group_by([col("account")])
        .agg([
            col("name")
                .sort_by(
                    [col("name_games")],
                    SortMultipleOptions::default().with_order_descending(true),
                )
                .alias("names"),
            col("name_games")
                .sum()
                .cast(DataType::UInt64)
                .alias("games"),
        ])
        .sort(
            ["games", "account"],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        )
        .limit(limit)
        .collect()?;
    let keys = accounts.column("account")?.str()?.clone();
    let games = accounts.column("games")?.u64()?.clone();
    let names = accounts.column("names")?.list()?.clone();
    Ok((0..accounts.height())
        .map(|idx| {
            let account = keys.get(idx).unwrap_or_default().to_string();
            PlayerAccount {
                grouped_as: groups.get(&account).cloned().unwrap_or_default(),
                account,
                names: names
                    .get_as_series(idx)
                    .and_then(|series| {
                        series
                            .str()
                            .ok()
                            .map(|names| names.iter().flatten().map(str::to_string).collect())
                    })
                    .unwrap_or_default(),
                games: games.get(idx).unwrap_or_default() as usize,
            }
        })
        .collect())
}
