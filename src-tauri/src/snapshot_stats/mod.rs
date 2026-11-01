//! Provides information about the analyzed game collection.
use crate::data::scan_snapshot_ipc;
use crate::host::SwarmyHost;
use polars::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use swarmy_common::*;

pub async fn get_snapshot_metadata(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = host.majordomo();
    let res = ApiResponseBuilder::new();
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();

    mdp_tx.send(AsyncTask::GetSnapshotStats(res_tx)).await?;
    let snapshot_medata = res_rx.await?;
    Ok(res
        .with_message(serde_json::to_string(&snapshot_medata)?)
        .with_status(snapshot_medata.meta.success)
        .build())
}

/// The last state check of a snapshot, see [`snapshot_unavailable`].
struct SnapshotStateCheck {
    root: PathBuf,
    checked_at: Instant,
    message: Option<String>,
}

static LAST_STATE_CHECK: Mutex<Option<SnapshotStateCheck>> = Mutex::new(None);

/// How long a state check is reused: the views send several queries at once, and the check
/// reads the metadata of every day partition of the snapshot.
const STATE_CHECK_TTL: Duration = Duration::from_secs(3);

/// Why the snapshot at the directory cannot be queried: being created (by this swarmy or another
/// one, i.e. the app and the remote backend of the same computer), interrupted or missing. None
/// when it is ready.
pub fn snapshot_unavailable(root: &Path) -> Option<String> {
    let mut last = LAST_STATE_CHECK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(check) = last.as_ref() {
        if check.root == root && check.checked_at.elapsed() < STATE_CHECK_TTL {
            return check.message.clone();
        }
    }
    let message = s2protocol::snapshot::snapshot_state(root).message();
    if let Some(message) = &message {
        tracing::info!("The snapshot queries are refused: {message}");
    }
    *last = Some(SnapshotStateCheck {
        root: root.to_path_buf(),
        checked_at: Instant::now(),
        message: message.clone(),
    });
    message
}

/// Gets the list of maps from the details.ipc file
pub fn try_get_snapshot_metadata(
    replay_path: String,
    file_cache_path: String,
) -> Result<SnapshotStats, SwarmyError> {
    // remove trailing slash if exists
    let replay_path = replay_path.trim_end_matches('/').to_string();
    let ipc_path = format!("{}/{}", replay_path, IPC_DIR);
    tracing::info!("Getting snapshot metadata from: {}", ipc_path);
    let root = std::path::Path::new(&ipc_path);
    // Being created or interrupted: the files cannot be read, only the state is told.
    let state = s2protocol::snapshot::snapshot_state(root);
    let metadata = match state {
        s2protocol::snapshot::SnapshotState::Ready(metadata) => metadata,
        other => {
            return Ok(SnapshotStats {
                snapshot_state: SnapshotStateKind::from(&other),
                snapshot_message: other.message(),
                replay_path,
                ..Default::default()
            });
        }
    };
    // The metadata has the sizes of the files of all the day partitions.
    let ipc_dir_size = metadata.files.iter().map(|file| file.bytes).sum();
    let mut caches_size = 0;
    let mut num_caches = 0;
    if std::path::Path::new(&file_cache_path).exists() {
        for entry in std::fs::read_dir(&file_cache_path)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = std::fs::metadata(path)?;
            let size = metadata.len();
            caches_size += size;
            num_caches += 1;
        }
    }
    let date_modified = metadata.created_at.into();
    let details_query = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?;

    // Date range from the details.ipc file
    let res = details_query
        .select([
            col("ext_datetime")
                .min()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("min_date"),
            col("ext_datetime")
                .max()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("max_date"),
            // The ids of the replays added later follow the existing ones, there can be gaps.
            col("ext_fs_id")
                .n_unique()
                .cast(DataType::UInt64)
                .alias("num_games"),
            col("title").n_unique().alias("num_maps"),
        ])
        .collect()?;

    let min_date = col_ymd_to_naive_date(&res, "min_date")?;
    let max_date = col_ymd_to_naive_date(&res, "max_date")?;
    let num_games = res.column("num_games")?.u64()?.get(0).unwrap_or(0);
    let num_maps = res.column("num_maps")?.u32()?.get(0).unwrap_or(0);

    Ok(SnapshotStats {
        ipc_dir_size,
        date_modified,
        max_date,
        min_date,
        num_games,
        num_maps,
        num_caches,
        caches_size,
        is_compatible: metadata.is_compatible(),
        snapshot_state: SnapshotStateKind::Ready,
        snapshot_message: None,
        replay_path,
        partitions: metadata.partitions,
        added_replays: metadata.added,
        s2protocol_version: Some(metadata.s2protocol_version),
    })
}
