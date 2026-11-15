#![doc = include_str!("similar_accounts/README.md")]

use super::{accounts, player_rows};
use crate::data::*;
use polars::prelude::*;
use std::collections::HashMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The accounts with a name like the player's listed to group with it.
pub const SIMILAR_ACCOUNTS_SHOWN: usize = 20;

/// The accounts of other players whose name contains the player's (ignoring case), as
/// [`accounts`]: `account`, `grouped_as`, `name`, `name_games`, `games`.
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    player: &str,
) -> Result<LazyFrame, SwarmyError> {
    accounts::by_account(
        player_rows::plan(source, groups)?.filter(
            col("player").neq(lit(player)).and(
                col("name")
                    .str()
                    .to_lowercase()
                    .str()
                    .contains_literal(lit(player.to_lowercase())),
            ),
        ),
    )
}

/// The first [`SIMILAR_ACCOUNTS_SHOWN`] accounts, see [`accounts::shape`].
pub fn shape(frame: DataFrame) -> Result<Vec<PlayerAccount>, SwarmyError> {
    accounts::shape(frame, SIMILAR_ACCOUNTS_SHOWN)
}

#[cfg(test)]
mod tests;
