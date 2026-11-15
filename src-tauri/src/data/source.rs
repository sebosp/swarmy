//! Where a query reads the snapshot from: the Arrow IPC files of a Replay Path in the app and the
//! server, frames made by hand in the tests ([`crate::data::fixtures`]). A query takes a
//! `&impl SnapshotSource` (or `&dyn SnapshotSource`) and never a path, so that the same `plan`
//! runs on three rows a human can check and on 36,000 games. See AGENTS.md, "A query is a file".

use super::{
    build_ipc_path, sanitize_replay_path, scan_day_partition, scan_snapshot_ipc, scan_usage,
};
use polars::prelude::*;
use swarmy_common::*;

/// The tables of a snapshot, by the file name constants of swarmy-common (`DETAILS_IPC`...).
pub trait SnapshotSource: Send + Sync {
    /// One of the tables of the snapshot, lazily: `details.arrow`, `game_players.arrow`...
    fn table(&self, file_name: &str) -> Result<LazyFrame, SwarmyError>;

    /// A usage table of the aggregations at a grain: `ability_usage.arrow`, `unit_usage.arrow`,
    /// `player_usage.arrow` per day, month or year.
    fn usage(&self, grain: TimeGrain, file_name: &str) -> Result<LazyFrame, SwarmyError>;
}

/// The snapshot of a Replay Path on disk, `<replay path>/ipcs`: the hot tables from memory, the
/// others scanned from their day partitions (see [`scan_snapshot_ipc`]); or one day of it
/// ([`Self::on_day`]), for the queries of one replay.
#[derive(Debug, Clone)]
pub struct IpcSnapshot {
    ipc_path: String,
    day: Option<chrono::NaiveDate>,
}

impl IpcSnapshot {
    /// The snapshot of a Replay Path, an error when it is not Optimized yet or being written.
    pub fn open(replay_path: &str) -> Result<Self, SwarmyError> {
        Ok(Self {
            ipc_path: build_ipc_path(&sanitize_replay_path(replay_path)?)?,
            day: None,
        })
    }

    /// A snapshot by the path of its `ipcs` directory, already checked.
    pub fn at(ipc_path: impl Into<String>) -> Self {
        Self {
            ipc_path: ipc_path.into(),
            day: None,
        }
    }

    /// The same snapshot, its tables read from the partition of one day only: the queries of a
    /// replay (its bases, its deaths) scan a day instead of the thousands of the snapshot, a
    /// filter on `ext_fs_id` cannot skip the partitions by itself.
    pub fn on_day(self, day: chrono::NaiveDate) -> Self {
        Self {
            day: Some(day),
            ..self
        }
    }

    pub fn ipc_path(&self) -> &str {
        &self.ipc_path
    }
}

impl SnapshotSource for IpcSnapshot {
    fn table(&self, file_name: &str) -> Result<LazyFrame, SwarmyError> {
        match self.day {
            Some(day) => scan_day_partition(&self.ipc_path, day, file_name),
            None => scan_snapshot_ipc(&self.ipc_path, file_name),
        }
    }

    fn usage(&self, grain: TimeGrain, file_name: &str) -> Result<LazyFrame, SwarmyError> {
        scan_usage(&self.ipc_path, grain, file_name)
    }
}

/// A snapshot made of frames in memory, for the tests: the same path the hot tables take in
/// production (`DataFrame::lazy`). Built by [`crate::data::fixtures::Snapshot::source`].
#[cfg(test)]
#[derive(Debug, Default, Clone)]
pub struct MemSnapshot {
    tables: std::collections::HashMap<String, DataFrame>,
}

#[cfg(test)]
impl MemSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a table, by its file name (`DETAILS_IPC`...).
    pub fn with(mut self, file_name: &str, table: DataFrame) -> Self {
        self.tables.insert(file_name.to_string(), table);
        self
    }

    /// Adds a usage table of a grain.
    pub fn with_usage(mut self, grain: TimeGrain, file_name: &str, table: DataFrame) -> Self {
        self.tables.insert(Self::usage_key(grain, file_name), table);
        self
    }

    fn usage_key(grain: TimeGrain, file_name: &str) -> String {
        format!("{grain:?}/{file_name}")
    }

    /// Registers the tables for the SQL of the READMEs: `details.arrow` is `details`, the usage
    /// table of a grain `month_ability_usage`.
    pub fn register_in(&self, ctx: &polars::sql::SQLContext) {
        for (key, table) in &self.tables {
            let name = key
                .trim_end_matches(".arrow")
                .replace('/', "_")
                .to_lowercase();
            ctx.register(&name, table.clone().lazy());
        }
    }

    fn get(&self, key: &str) -> Result<LazyFrame, SwarmyError> {
        self.tables
            .get(key)
            .map(|table| table.clone().lazy())
            .ok_or_else(|| {
                SwarmyError::Other(format!(
                    "The test snapshot has no {key} table, add it with MemSnapshot::with"
                ))
            })
    }
}

#[cfg(test)]
impl SnapshotSource for MemSnapshot {
    fn table(&self, file_name: &str) -> Result<LazyFrame, SwarmyError> {
        self.get(file_name)
    }

    fn usage(&self, grain: TimeGrain, file_name: &str) -> Result<LazyFrame, SwarmyError> {
        self.get(&Self::usage_key(grain, file_name))
    }
}
