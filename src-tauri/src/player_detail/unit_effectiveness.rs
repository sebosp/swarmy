#![doc = include_str!("unit_effectiveness/README.md")]

use super::player_rows;
use crate::data::*;
use polars::prelude::*;
use s2protocol::game_events::VersionedBalanceUnits;
use std::collections::{BTreeMap, HashMap};
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The units of the player over all their games: `unit`, `born`, `kills` and `value_killed`,
/// summed from `game_units`.
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    player: &str,
) -> Result<LazyFrame, SwarmyError> {
    let mine = player_rows::plan(source, groups)?
        .filter(col("player").eq(lit(player)))
        .select([col("ext_fs_id"), col("player_name")]);
    Ok(source
        .table(GAME_UNITS_IPC)?
        .join(
            mine,
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("unit")])
        .agg([
            col("born").sum(),
            col("kills").sum(),
            col("value_killed").sum(),
        ])
        .sort(["unit"], Default::default()))
}

/// The units as the view shows them, with their cost from the balance data.
pub fn shape(
    frame: DataFrame,
    balance: &VersionedBalanceUnits,
) -> Result<Vec<UnitEffectiveness>, SwarmyError> {
    let names = str_values(&frame, "unit")?;
    let born = u32_values(&frame, "born")?;
    let kills = u32_values(&frame, "kills")?;
    let values = u64_values(&frame, "value_killed")?;
    let mut produced: BTreeMap<String, usize> = BTreeMap::new();
    let mut killed: BTreeMap<String, (usize, u32)> = BTreeMap::new();
    for row in 0..frame.height() {
        if born[row] > 0 {
            produced.insert(names[row].clone(), born[row] as usize);
        }
        if kills[row] > 0 {
            killed.insert(
                names[row].clone(),
                (kills[row] as usize, values[row] as u32),
            );
        }
    }
    Ok(crate::replay_detail::data::unit_effectiveness(
        produced, &killed, balance,
    ))
}

#[cfg(test)]
mod tests;
