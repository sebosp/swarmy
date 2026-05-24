//! Provides information about the analyzed game collection.
use polars::prelude::*;
use swarmy_tauri_common::*;

#[tauri::command(rename_all = "snake_case")]
pub async fn get_snapshot_metadata(replay_path: String) -> ApiResponse {
    // create a thread to get the metadata in the background:
    let t = std::thread::spawn(move || {
        let init_time = std::time::Instant::now();
        match try_get_snapshot_metadata(replay_path) {
            Ok(val) => ApiResponse::new(
                ResponseMetaBuilder::new(true)
                    .duration_ms(init_time.elapsed().as_millis() as u64)
                    .build(),
                serde_json::to_string(&val).unwrap_or_default(),
            ),
            Err(e) => {
                log::error!("Error getting snapshot metadata: {}", e);
                ApiResponse::new(
                    ResponseMetaBuilder::new(false)
                        .duration_ms(init_time.elapsed().as_millis() as u64)
                        .build(),
                    format!("Error getting snapshot metadata: {:?}", e),
                )
            }
        }
    });
    t.join().unwrap()
}

/// Gets the list of maps from the details.ipc file
pub fn try_get_snapshot_metadata(replay_path: String) -> Result<SnapshotStats, SwarmyTauriError> {
    // remove trailing slash if exists
    let replay_path = replay_path.trim_end_matches('/').to_string();
    let ipc_path = format!("{}/{}/", replay_path, IPC_DIR);
    let file_cache_path = format!("{}/{}/", replay_path, CACHES_DIR);
    log::info!("Getting snapshot metadata from: {}", ipc_path);
    // Add the size of all the files in state.source_dir
    let mut ipc_dir_size = 0;
    for entry in std::fs::read_dir(&ipc_path)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();
        ipc_dir_size += size;
    }
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
    // get the date_modified of the details.ipc file
    let details_ipc_filename = format!("{}/{}", ipc_path, DETAILS_IPC);
    let date_modified = std::fs::metadata(details_ipc_filename)?.modified()?;
    let details_query = LazyFrame::scan_ipc(
        PlRefPath::new(&format!("{}/{}", ipc_path, DETAILS_IPC)),
        Default::default(),
        Default::default(),
    )?;

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
            col("ext_fs_id").max().alias("num_games"),
            col("title").n_unique().alias("num_maps"),
        ])
        .collect()?;

    let min_date = col_ymd_to_naive_date(&res, "min_date")?;
    let max_date = col_ymd_to_naive_date(&res, "max_date")?;
    let num_games = res.column("num_games")?.u64()?.get(0).unwrap_or(0) + 1;
    let num_maps = res.column("num_maps")?.u32()?.get(0).unwrap_or(0) + 1;
    // let data_str = crate::common::convert_df_to_json_data(&res)?;

    Ok(SnapshotStats {
        ipc_dir_size,
        date_modified,
        max_date,
        min_date,
        num_games,
        num_maps,
        num_caches,
        caches_size,
    })
}
