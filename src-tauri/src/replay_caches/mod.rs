use crate::SetupState;
use crate::{data::*, majordomo::AsyncTask};
use polars::prelude::*;
use std::path::{Path, PathBuf};
use swarmy_tauri_common::*;
use tauri::State;
use tracing::instrument;

#[tauri::command(rename_all = "snake_case")]
pub async fn download_replay_caches(
    state: State<'_, SetupState>,
) -> Result<ApiResponse, SwarmyTauriError> {
    let mdp_tx = state.majordomo_tx.clone();
    let init_time = std::time::Instant::now();
    let (res_tx, res_rx) = tokio::sync::oneshot::channel();
    let mut res = match mdp_tx.send(AsyncTask::DownloadCaches(res_tx)).await {
        Ok(_) => ApiResponse::new(
            ResponseMetaBuilder::new(true)
                .duration_ms(init_time.elapsed().as_millis() as u64)
                .build(),
            "Triggered download caches task on background".to_string(),
        ),
        Err(e) => {
            log::error!("Error download caches: {}", e);
            ApiResponse::new(
                ResponseMetaBuilder::new(true)
                    .duration_ms(init_time.elapsed().as_millis() as u64)
                    .build(),
                format!("Error triggering download cache: {:?}", e),
            )
        }
    };
    if let Err(e) = res_rx.await {
        log::error!("Error waiting for download caches result: {}", e);
        res = ApiResponse::new(
            ResponseMetaBuilder::new(true)
                .duration_ms(init_time.elapsed().as_millis() as u64)
                .build(),
            format!("Error waiting for download caches result: {:?}", e),
        )
    };
    Ok(res)
}

pub async fn try_download_replay_caches(
    app_handle: tauri::AppHandle,
) -> Result<String, SwarmyTauriError> {
    let app_settings = crate::settings::load_app_settings(app_handle).await?;
    let replay_path = app_settings.replay_path;
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;

    let cache_path = PathBuf::from(&replay_path);
    let destination = cache_path.join("caches");
    if !destination.exists() {
        std::fs::create_dir_all(&destination)?;
    }
    log::info!(
        "Downloading replay caches from files in {} and storing into {}",
        cache_path.display(),
        destination.display()
    );
    let res = tokio::task::spawn_blocking(move || {
        let details_query = LazyFrame::scan_ipc(
            PlRefPath::new(&format!("{}/{}", ipc_path, DETAILS_IPC)),
            Default::default(),
            Default::default(),
        )?;
        log::info!("Loaded details.ipc, extracting unique cache handles...");
        details_query
            .unique(
                Some(Selector::Matches("cache_handles".into())),
                UniqueKeepStrategy::Any,
            )
            .limit(1000)
            .collect()
    })
    .await??;
    // TODO: Maybe we don't need the cache region or s2ma format...
    let mut unique_cache_handles: Vec<String> = vec![];
    log::info!("Extracting unique cache handles from details.ipc...");
    for idx in 0..res.height() {
        let slice = res.slice(idx as i64, 1);
        let cache_handles_str = slice.column("cache_handles")?.str()?.get(0).unwrap_or("");
        let cache_handles: Vec<&str> = cache_handles_str.split(',').collect();
        for handle in cache_handles {
            if unique_cache_handles.contains(&handle.to_string()) {
                continue;
            }
            unique_cache_handles.push(handle.to_string());
        }
    }
    log::info!(
        "Found {} unique cache handles, starting download...",
        unique_cache_handles.len()
    );
    for (idx, handle) in unique_cache_handles.iter().enumerate() {
        log::info!(
            "Downloading cache {}/{}: {}",
            idx + 1,
            unique_cache_handles.len(),
            handle
        );
        if let Err(e) = download_cache(handle, &destination).await {
            log::error!("Error downloading cache {}: {}", handle, e);
        }
        if let Err(e) = download_cache(handle, &destination).await {
            log::error!("Error downloading cache {}: {}", handle, e);
        }
    }
    Ok(String::from("Download caches finished successfully."))
}

#[instrument]
pub async fn download_cache(handle: &str, destination: &Path) -> Result<(), SwarmyTauriError> {
    log::info!("Downloading cache with handle: {}", handle);
    let cache_download_target = destination.join(format!("{}.s2ma", handle));
    if cache_download_target.exists() {
        log::info!(
            "Cache {} already exists, skipping download.",
            cache_download_target.display()
        );
        return Ok(());
    }

    let response = reqwest::get(format!(
        "https://eu-s2-depot.classic.blizzard.com/{}.s2ma",
        handle
    ))
    .await?;
    if !response.status().is_success() {
        return Err(SwarmyTauriError::Other(format!(
            "Failed to download cache {}, status code: {}",
            handle,
            response.status()
        )));
    }
    let response_bytes = response.bytes().await?;
    std::fs::write(&cache_download_target, response_bytes)?;
    Ok(())
}
