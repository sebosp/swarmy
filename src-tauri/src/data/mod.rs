use polars::prelude::*;
use s2protocol::arrow_store::partitioned;
use swarmy_common::*;

/// Sanitcizes the replay path, this should be stored locally in the AppConfig.
pub fn sanitize_replay_path(replay_path: &str) -> Result<String, SwarmyError> {
    if replay_path.is_empty() {
        return Err(SwarmyError::Other(
            "Replay path is not set, please set it in Scan tab.".to_string(),
        ));
    }
    Ok(replay_path.trim_end_matches('/').to_string())
}

/// Builds the IPC path for the given replay path, the path must exist, done via the Optimize
/// functionality in the Scan tab.
pub fn build_ipc_path(replay_path: &str) -> Result<String, SwarmyError> {
    let replay_path = format!("{}/{}", replay_path, IPC_DIR);
    let ipc_path = std::path::Path::new(&replay_path);
    if !ipc_path.exists() {
        return Err(SwarmyError::Other(format!(
            "Directory {} not Optimized yet, go to Scan first.",
            replay_path
        )));
    }
    Ok(replay_path)
}

/// Scans one of the IPC files of the snapshot, from all its day partitions, or from its root
/// for the snapshots written before the partitions.
pub fn scan_snapshot_ipc(ipc_path: &str, file_name: &str) -> Result<LazyFrame, SwarmyError> {
    let ipc_path = ipc_path.trim_end_matches('/');
    let path = if partitioned::has_flat_layout(std::path::Path::new(ipc_path)) {
        format!("{ipc_path}/{file_name}")
    } else {
        format!("{ipc_path}/{}", partitioned::partition_glob(file_name))
    };
    Ok(LazyFrame::scan_ipc(
        PlRefPath::new(path),
        Default::default(),
        UnifiedScanArgs {
            // The year, month and day of the partitions are not added as columns, the
            // ext_datetime column has the date.
            hive_options: HiveOptions::new_disabled(),
            ..Default::default()
        },
    )?)
}

/// Filters the rows of the given replays.
pub fn is_in_replays(ext_fs_ids: &[u64]) -> Expr {
    col("ext_fs_id").is_in(
        lit(Series::new("ext_fs_ids".into(), ext_fs_ids)).implode(false),
        false,
    )
}

/// Reads a string column, nulls are empty strings.
pub fn str_values(df: &DataFrame, name: &str) -> Result<Vec<String>, SwarmyError> {
    Ok(df
        .column(name)?
        .str()?
        .iter()
        .map(|value| value.unwrap_or_default().to_string())
        .collect())
}

pub fn u64_values(df: &DataFrame, name: &str) -> Result<Vec<u64>, SwarmyError> {
    Ok(df
        .column(name)?
        .u64()?
        .iter()
        .map(|value| value.unwrap_or_default())
        .collect())
}

pub fn u32_values(df: &DataFrame, name: &str) -> Result<Vec<u32>, SwarmyError> {
    Ok(df
        .column(name)?
        .u32()?
        .iter()
        .map(|value| value.unwrap_or_default())
        .collect())
}

pub fn u8_values(df: &DataFrame, name: &str) -> Result<Vec<Option<u8>>, SwarmyError> {
    Ok(df.column(name)?.u8()?.iter().collect())
}

pub fn f32_values(df: &DataFrame, name: &str) -> Result<Vec<f32>, SwarmyError> {
    Ok(df
        .column(name)?
        .f32()?
        .iter()
        .map(|value| value.unwrap_or_default())
        .collect())
}

pub fn i64_values(df: &DataFrame, name: &str) -> Result<Vec<i64>, SwarmyError> {
    Ok(df
        .column(name)?
        .i64()?
        .iter()
        .map(|value| value.unwrap_or_default())
        .collect())
}

/// The game seconds of the `ext_replay_loop` column, at Faster speed.
///
/// The `ext_replay_seconds` column of the snapshot is computed by s2protocol-rs with 68.58 ms
/// per loop, about 14.6 loops per second instead of 22.4, so it is not used.
pub fn game_seconds() -> Expr {
    (col("ext_replay_loop").cast(DataType::Float64)
        / lit(crate::replay_rerun::viewer::GAME_LOOPS_PER_SECOND))
    .cast(DataType::UInt32)
    .alias("game_seconds")
}
