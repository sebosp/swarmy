#![doc = include_str!("player_rows/README.md")]

use crate::data::*;
use polars::prelude::*;
use std::collections::HashMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The rows of `details` of the players (not the observers) with `account`, `name` (without the
/// clan tag), `account_name` (the name the account played the most games with), `grouped_as`
/// (the player the account is grouped under in the personal database, `groups`) and `player`:
/// the first of `grouped_as`, `account_name` and `name` that exists.
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
) -> Result<LazyFrame, SwarmyError> {
    let details = source
        .table(DETAILS_IPC)?
        .filter(col("player_observe").eq(lit(0u8)))
        .with_columns([
            account_expr().alias("account"),
            name_without_clan_expr().alias("name"),
        ]);
    // The name of each account: the one of the most games, then the first by name.
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

#[cfg(test)]
mod tests;
