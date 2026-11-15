#![doc = include_str!("accounts/README.md")]

use super::player_rows;
use crate::data::*;
use polars::prelude::*;
use std::collections::HashMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The accounts shown in the Accounts panel.
pub const ACCOUNTS_SHOWN: usize = 100;

/// The accounts of the player and the names they played as: a row per account and name with
/// `account`, `grouped_as`, `name`, `name_games` (u32) and `games` (u32, of the account).
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    player: &str,
) -> Result<LazyFrame, SwarmyError> {
    by_account(player_rows::plan(source, groups)?.filter(col("player").eq(lit(player))))
}

/// The accounts of some rows of `player_rows`, with their names: the most games first, within
/// an account the most played name first.
pub(super) fn by_account(rows: LazyFrame) -> Result<LazyFrame, SwarmyError> {
    Ok(rows
        .filter(col("account").is_not_null())
        .group_by([col("account"), col("grouped_as"), col("name")])
        .agg([col("ext_fs_id").n_unique().alias("name_games")])
        .with_column(
            col("name_games")
                .sum()
                .over([col("account")])?
                .alias("games"),
        )
        .sort_by_exprs(
            [col("games"), col("account"), col("name_games"), col("name")],
            SortMultipleOptions::default().with_order_descending_multi([true, false, true, false]),
        ))
}

/// The rows grouped into [`PlayerAccount`]s, the first `limit` accounts.
pub fn shape(frame: DataFrame, limit: usize) -> Result<Vec<PlayerAccount>, SwarmyError> {
    let accounts = str_values(&frame, "account")?;
    let grouped_as = str_values(&frame, "grouped_as")?;
    let names = str_values(&frame, "name")?;
    let games = u32_values(&frame, "games")?;
    let mut shaped: Vec<PlayerAccount> = vec![];
    for row in 0..frame.height() {
        if shaped
            .last()
            .is_none_or(|account| account.account != accounts[row])
        {
            if shaped.len() == limit {
                break;
            }
            shaped.push(PlayerAccount {
                account: accounts[row].clone(),
                names: vec![],
                games: games[row] as usize,
                grouped_as: grouped_as[row].clone(),
            });
        }
        shaped
            .last_mut()
            .expect("pushed above")
            .names
            .push(names[row].clone());
    }
    Ok(shaped)
}

#[cfg(test)]
mod tests;
