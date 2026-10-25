//! Provides information about the analyzed game collection.
use crate::data::scan_snapshot_ipc;
use crate::SetupState;
use polars::prelude::*;
use swarmy_common::*;
use tauri::State;

#[tauri::command(rename_all = "snake_case")]
pub async fn get_snapshot_metadata(
    state: State<'_, SetupState>,
) -> Result<ApiResponse, SwarmyError> {
    let mdp_tx = state.majordomo_tx.clone();
    let res = ApiResponseBuilder::new();
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();

    mdp_tx.send(AsyncTask::GetSnapshotStats(res_tx)).await?;
    let snapshot_medata = res_rx.await?;
    Ok(res
        .with_message(serde_json::to_string(&snapshot_medata)?)
        .with_status(snapshot_medata.meta.success)
        .build())
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
    let metadata = s2protocol::snapshot::SnapshotMetadata::read_from_dir(root).ok();
    // The metadata has the sizes of the files of all the day partitions.
    let ipc_dir_size = match &metadata {
        Some(metadata) => metadata.files.iter().map(|file| file.bytes).sum(),
        None => {
            let mut ipc_dir_size = 0;
            for entry in std::fs::read_dir(root)? {
                ipc_dir_size += entry?.metadata()?.len();
            }
            ipc_dir_size
        }
    };
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
    let date_modified = match &metadata {
        Some(metadata) => metadata.created_at.into(),
        None => std::fs::metadata(root.join(DETAILS_IPC))?.modified()?,
    };
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
        is_compatible: metadata
            .as_ref()
            .is_some_and(|metadata| metadata.is_compatible()),
        partitions: metadata.as_ref().map_or(0, |metadata| metadata.partitions),
        added_replays: metadata.as_ref().map_or(0, |metadata| metadata.added),
        s2protocol_version: metadata.map(|metadata| metadata.s2protocol_version),
    })
}
