#![doc = include_str!("ability_uses/README.md")]

use crate::data::*;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The use of an ability by the players of its race in each period of the grain: `period`,
/// `games` (u32, the player games it was used in), `wins` (u32) and `uses` (u64), by period.
pub fn plan(
    source: &dyn SnapshotSource,
    grain: TimeGrain,
    ability: &str,
    race: &str,
) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .usage(grain, ABILITY_USAGE_IPC)?
        .filter(
            col("ability")
                .eq(lit(ability))
                .and(race_name_expr("player_race").eq(lit(race))),
        )
        .group_by([col("period")])
        .agg([col("games").sum(), col("wins").sum(), col("uses").sum()])
        .sort(["period"], Default::default()))
}

/// By period: the games used in, the wins and the uses.
pub fn shape(frame: DataFrame) -> Result<BTreeMap<String, (u64, u64, u64)>, SwarmyError> {
    let games = u32_values(&frame, "games")?;
    let wins = u32_values(&frame, "wins")?;
    let uses = u64_values(&frame, "uses")?;
    Ok(str_values(&frame, "period")?
        .into_iter()
        .enumerate()
        .map(|(idx, period)| (period, (games[idx] as u64, wins[idx] as u64, uses[idx])))
        .collect())
}

#[cfg(test)]
mod tests;
