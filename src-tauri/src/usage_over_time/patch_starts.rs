#![doc = include_str!("patch_starts/README.md")]

use crate::data::*;
use crate::replay_rerun::decisions::closest_balance_version;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The first period of each protocol version in the games: `ext_protocol_version` and `period`,
/// by version.
pub fn plan(source: &dyn SnapshotSource, grain: TimeGrain) -> Result<LazyFrame, SwarmyError> {
    Ok(source
        .usage(grain, PLAYER_USAGE_IPC)?
        .group_by([col("ext_protocol_version")])
        .agg([col("period").min()])
        .sort(["ext_protocol_version"], Default::default()))
}

/// The first period of each patch of the balance data (the versions of a patch are its closest
/// balance data), the oldest first: the dashed lines of the charts.
pub fn shape(frame: DataFrame) -> Result<Vec<PatchStart>, SwarmyError> {
    let mut starts: BTreeMap<u32, String> = BTreeMap::new();
    for (version, period) in u32_values(&frame, "ext_protocol_version")?
        .into_iter()
        .zip(str_values(&frame, "period")?)
    {
        let start = starts
            .entry(closest_balance_version(version)?)
            .or_insert_with(|| period.clone());
        if period < *start {
            *start = period;
        }
    }
    let mut starts: Vec<PatchStart> = starts
        .into_iter()
        .map(|(patch, period)| PatchStart { period, patch })
        .collect();
    starts.sort_by(|a, b| a.period.cmp(&b.period).then(a.patch.cmp(&b.patch)));
    Ok(starts)
}

#[cfg(test)]
mod tests;
