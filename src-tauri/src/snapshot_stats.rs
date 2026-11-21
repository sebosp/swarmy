//! Provides information about the analyzed game collection: the state and sizes of the snapshot
//! from its metadata, the games, maps and dates from the query file `summary.rs` (AGENTS.md, "A
//! query is a file").

pub mod summary;

use crate::data::{IpcSnapshot, QueryMeta};
use crate::host::SwarmyHost;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use swarmy_common::*;

pub async fn get_snapshot_metadata(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = host.majordomo();
    let res = ApiResponseBuilder::new().with_sources(QUERIES.map(QuerySource::from));
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();

    mdp_tx.send(AsyncTask::GetSnapshotStats(res_tx)).await?;
    let snapshot_medata = res_rx.await?;
    Ok(res
        .with_message(serde_json::to_string(&snapshot_medata)?)
        .with_status(snapshot_medata.meta.success)
        .build())
}

/// The queries of the view, for the debug links.
pub const QUERIES: [QueryMeta; 1] = [summary::META];

/// The last state check of a snapshot, see [`snapshot_unavailable`].
struct SnapshotStateCheck {
    root: PathBuf,
    checked_at: Instant,
    /// The snapshot as seen by the cheap checks of [`snapshot_signature`].
    signature: SnapshotSignature,
    message: Option<String>,
}

/// What changes when a snapshot is written: the marker of a generation in progress (or
/// interrupted) and the time of the metadata of the root, written last.
#[derive(Debug, Clone, PartialEq)]
struct SnapshotSignature {
    in_progress: bool,
    metadata_modified: Option<std::time::SystemTime>,
}

/// The cheap checks of a snapshot, a few file stats.
fn snapshot_signature(root: &Path) -> SnapshotSignature {
    SnapshotSignature {
        in_progress: root
            .join(s2protocol::snapshot::SNAPSHOT_IN_PROGRESS_FNAME)
            .exists(),
        metadata_modified: crate::data::snapshot_generation(&root.display().to_string()),
    }
}

static LAST_STATE_CHECK: Mutex<Option<SnapshotStateCheck>> = Mutex::new(None);

/// How long a state check is reused while a generation is in progress: the views send several
/// queries at once. A ready snapshot is checked again only when its signature changes: the full
/// check reads the metadata of every day partition, 0.4 s on 2,000 days (2026-11-21), before
/// every query.
const STATE_CHECK_TTL: Duration = Duration::from_secs(3);

/// Why the snapshot at the directory cannot be queried: being created (by this swarmy or another
/// one, i.e. the app and the remote backend of the same computer), interrupted or missing. None
/// when it is ready.
pub fn snapshot_unavailable(root: &Path) -> Option<String> {
    let mut last = LAST_STATE_CHECK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let signature = snapshot_signature(root);
    if let Some(check) = last.as_ref()
        && check.root == root
        && check.signature == signature
        && (!signature.in_progress || check.checked_at.elapsed() < STATE_CHECK_TTL)
    {
        return check.message.clone();
    }
    let message = s2protocol::snapshot::snapshot_state(root).message();
    if let Some(message) = &message {
        tracing::info!("The snapshot queries are refused: {message}");
    }
    *last = Some(SnapshotStateCheck {
        root: root.to_path_buf(),
        checked_at: Instant::now(),
        signature,
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
    let summary = summary::shape(summary::plan(&IpcSnapshot::at(&ipc_path))?.collect()?)?;

    Ok(SnapshotStats {
        ipc_dir_size,
        date_modified,
        max_date: summary.max_date,
        min_date: summary.min_date,
        num_games: summary.num_games,
        num_maps: summary.num_maps,
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
