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

/// Runs a query that aggregates the event files of the whole snapshot on the streaming engine,
/// the files are read in batches instead of all at once. The small queries are faster in memory,
/// they use `collect`.
pub fn collect_streaming(query: LazyFrame) -> Result<DataFrame, SwarmyError> {
    Ok(query
        .collect_with_engine(Engine::Streaming)?
        .unwrap_single())
}

/// Runs the queries as one plan on the streaming engine, the scans and the subqueries they share
/// are computed once instead of collecting them and starting other queries from the result.
pub fn collect_all(queries: Vec<LazyFrame>) -> Result<Vec<DataFrame>, SwarmyError> {
    let opt_state = queries
        .first()
        .map(|query| query.get_current_optimizations())
        .unwrap_or_default();
    Ok(LazyFrame::collect_all_with_engine(
        queries
            .into_iter()
            .map(|query| query.logical_plan)
            .collect(),
        Engine::Streaming,
        opt_state,
    )?)
}

/// Filters the rows of the given replays.
pub fn is_in_replays(ext_fs_ids: &[u64]) -> Expr {
    col("ext_fs_id").is_in(
        lit(Series::new("ext_fs_ids".into(), ext_fs_ids)).implode(false),
        false,
    )
}

/// Whether the column is one of the values, as a literal list it is pushed down to the scans.
pub fn is_one_of<T: AsRef<str>>(column: &str, values: &[T]) -> Expr {
    let values: Vec<&str> = values.iter().map(|value| value.as_ref()).collect();
    col(column).is_in(
        lit(Series::new("values".into(), values)).implode(false),
        false,
    )
}

/// T, Z or P from the localized race names of the column, `?` for the others.
pub fn race_letter_expr(column: &str) -> Expr {
    when(is_one_of(column, &TERRAN))
        .then(lit("T"))
        .when(is_one_of(column, &ZERG))
        .then(lit("Z"))
        .when(is_one_of(column, &PROTOSS))
        .then(lit("P"))
        .otherwise(lit("?"))
}

/// Terran, Zerg or Protoss from the localized race names of the column, `Other` for the races of
/// the arcade maps or an unknown language.
pub fn race_name_expr(column: &str) -> Expr {
    when(is_one_of(column, &TERRAN))
        .then(lit("Terran"))
        .when(is_one_of(column, &ZERG))
        .then(lit("Zerg"))
        .when(is_one_of(column, &PROTOSS))
        .then(lit("Protoss"))
        .otherwise(lit("Other"))
}

/// When the snapshot was last written, the results computed from it are kept until it changes.
pub fn snapshot_generation(ipc_path: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(
        std::path::Path::new(ipc_path).join(s2protocol::snapshot::SNAPSHOT_METADATA_FNAME),
    )
    .and_then(|metadata| metadata.modified())
    .ok()
}

/// The patch of each protocol version of the games: the closest version of the balance data, as
/// the `ext_protocol_version` and `patch` columns. The games of a protocol version without
/// balance data are compared with the ones of the closest patch.
pub fn patch_versions(ipc_path: &str) -> Result<LazyFrame, SwarmyError> {
    let versions = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .select([col("ext_protocol_version")])
        .unique(None, UniqueKeepStrategy::Any)
        .collect()?;
    let versions = u32_values(&versions, "ext_protocol_version")?;
    let patches: Vec<u32> = versions
        .iter()
        .map(|version| crate::replay_rerun::decisions::closest_balance_version(*version))
        .collect::<Result<_, _>>()?;
    Ok(DataFrame::new(
        versions.len(),
        vec![
            Column::new("ext_protocol_version".into(), versions),
            Column::new("patch".into(), patches),
        ],
    )?
    .lazy())
}

/// One row per player of every game, not the observers: `ext_fs_id`, `player_name`, its tracker
/// `player_id`, `race` (see [`race_name_expr`]), `player_result`, `title` and `patch`.
///
/// The tracker events identify the players by `player_id` (`control_player_id`,
/// `killer_player_id`), the details by name. The units born have both, the starting townhalls
/// tell the id of each name.
pub fn snapshot_players(ipc_path: &str) -> Result<LazyFrame, SwarmyError> {
    let ids = scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
        .filter(
            col("ext_replay_loop")
                .eq(lit(0))
                .and(col("player_name").is_not_null()),
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("control_player_id").alias("player_id"),
        ])
        .unique(
            Some(cols(["ext_fs_id", "player_name"])),
            UniqueKeepStrategy::Any,
        );
    Ok(scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .filter(col("player_observe").eq(lit(0u8)))
        .select([
            col("ext_fs_id"),
            col("player_name"),
            race_name_expr("player_race").alias("race"),
            col("player_result"),
            col("title"),
            col("ext_protocol_version"),
        ])
        .join(
            patch_versions(ipc_path)?,
            [col("ext_protocol_version")],
            [col("ext_protocol_version")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            ids,
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Left),
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("player_id"),
            col("race"),
            col("player_result"),
            col("title"),
            col("patch"),
        ]))
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
