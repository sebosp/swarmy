#![doc = include_str!("unit_uses/README.md")]

use crate::data::*;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The use of a unit in each period of the grain: `period`, `games` (u32, the player games it
/// was made in), `wins` (u32), `born`, `died`, `kills` and `value_killed` (u64), by period.
pub fn plan(
    source: &dyn SnapshotSource,
    grain: TimeGrain,
    unit: &str,
) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .usage(grain, UNIT_USAGE_IPC)?
        .filter(col("unit").eq(lit(unit)))
        .group_by([col("period")])
        .agg([
            col("games").sum(),
            col("wins").sum(),
            col("born").sum(),
            col("died").sum(),
            col("kills").sum(),
            col("value_killed").sum(),
        ])
        .sort(["period"], Default::default()))
}

/// The use of the unit in a period.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UnitUse {
    pub games: u64,
    pub wins: u64,
    pub born: u64,
    pub died: u64,
    pub kills: u64,
    pub value_killed: u64,
}

pub fn shape(frame: DataFrame) -> Result<BTreeMap<String, UnitUse>, SwarmyError> {
    let games = u32_values(&frame, "games")?;
    let wins = u32_values(&frame, "wins")?;
    let born = u64_values(&frame, "born")?;
    let died = u64_values(&frame, "died")?;
    let kills = u64_values(&frame, "kills")?;
    let values = u64_values(&frame, "value_killed")?;
    Ok(str_values(&frame, "period")?
        .into_iter()
        .enumerate()
        .map(|(idx, period)| {
            (
                period,
                UnitUse {
                    games: games[idx] as u64,
                    wins: wins[idx] as u64,
                    born: born[idx],
                    died: died[idx],
                    kills: kills[idx],
                    value_killed: values[idx],
                },
            )
        })
        .collect())
}

#[cfg(test)]
mod tests;
