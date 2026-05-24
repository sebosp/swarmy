//! The actions related to the Scan view.
//!
use leptos::task::spawn_local;
use reactive_stores::{Patch, Store};
use leptos::ev::MouseEvent;
use leptos::leptos_dom::logging::console_log;
use leptos::prelude::*;
use swarmy_tauri_common::*;
use s2protocol::SC2ReplaysDirStats;
use crate::*;
use super::*;


/// Step 1, a user selects a directory to scan.
/// This may contain multiple subdirectories.
/// TODO: We could store a swarmy-stats.json in the root directory to prevent re-scanning.
/// This would also allow this step to be marked as completion when the UI is loaded.
pub fn trigger_basic_scan_replay_path(
    ev: MouseEvent,
    app_settings: ReadSignal<AppSettings>,
    set_backend_response: WriteSignal<ApiResponse>,
    data: Store<SC2ReplaysDirStatsTable>,
    set_activity_stage: WriteSignal<ActivityStage>,
) {
    ev.prevent_default();
    *set_activity_stage.write() = ActivityStage::ScanInit;
    *set_backend_response.write() = ApiResponse::new_incomplete();
    if app_settings.get().replay_path.is_empty() {
        console_log("Replay path is empty.");
        return;
    }

    let app_settings_cp = app_settings.get();

    spawn_local(async move {
        let args = serde_wasm_bindgen::to_value(&app_settings_cp).unwrap();
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        match serde_wasm_bindgen::from_value::<SC2ReplaysDirStats>(
            invoke("basic_scan_replay_path", args).await,
        ) {
            Ok(stats) => {
                let mut stats_table: SC2ReplaysDirStatsTable = stats.into();
                console_log(&format!("New data = {:?}", stats_table));
                data.top_10_players().write().retain(|_| false);
                data.top_10_players()
                    .write()
                    .append(&mut stats_table.top_10_players);
                data.top_10_maps().write().retain(|_| false);
                data.top_10_maps()
                    .write()
                    .append(&mut stats_table.top_10_maps);
                data.total_files().patch(stats_table.total_files);
                data.total_supported_replays()
                    .patch(stats_table.total_supported_replays);
                data.ability_supported_replays()
                    .patch(stats_table.ability_supported_replays);
            }
            Err(e) => {
                console_log(&format!("Error invoking basic_scan_replay_path: {:?}", e));
            }
        }
        *set_activity_stage.write() = ActivityStage::ScanDone;
    });
}

/// Step 2: Once a directory has gone through the basic scan, the process of generating the
/// ArrowIpc "snapshot" can be done.
/// TODO: When an `ipc` directory already exists, we could mark this step as done, but provide the
/// user the possibility to re-run the optimization.
pub fn trigger_optimize_replay_path(
    app_settings: ReadSignal<AppSettings>,
    set_backend_response: WriteSignal<ApiResponse>,
    set_activity_stage: WriteSignal<ActivityStage>,
) {
    *set_activity_stage.write() = ActivityStage::OptimizeInit;
    // Reset backend response status.
    *set_backend_response.write() = ApiResponse::new_incomplete();

    if app_settings.get().replay_path.is_empty() {
        console_log("Replay path is empty.");
        return;
    }

    let app_settings_cp = app_settings.get();
    spawn_local(async move {
        let args = serde_wasm_bindgen::to_value(&app_settings_cp).unwrap();
        console_log(&format!(
            "Invoking optimize_replay_path with args: {:?}",
            args
        ));
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        let response = invoke("optimize_replay_path", args).await;
        console_log(&format!("optimize_replay_path response: {:?}", response));
        match serde_wasm_bindgen::from_value::<ApiResponse>(response) {
            Ok(res) => {
                if res.meta.success {
                    console_log("Optimize replay path succeeded.");
                } else {
                    console_log(&format!("Optimize replay path failed: {:?}", res.message));
                }
                set_backend_response.set(res);
            }
            Err(e) => {
                console_log(&format!("Error invoking optimize_replay_path: {:?}", e));
                set_backend_response.set(ApiResponse {
                    meta: ResponseMeta {
                        success: false,
                        duration_ms: 0,
                        is_complete: true,
                    },
                    message: format!("Error invoking optimize_replay_path: {:?}", e),
                });
            }
        }
        *set_activity_stage.write() = ActivityStage::OptimizeDone;
    });
}

pub fn trigger_download_replay_caches(
    app_settings: ReadSignal<AppSettings>,
    set_backend_response: WriteSignal<ApiResponse>,
    set_activity_stage: WriteSignal<ActivityStage>,
) {
    *set_activity_stage.write() = ActivityStage::DownloadingCachesInit;
    // Reset backend response status.
    *set_backend_response.write() = ApiResponse::new_incomplete();

    if app_settings.get().replay_path.is_empty() {
        console_log("Replay path is empty.");
        return;
    }

    let app_settings_cp = app_settings.get();
    spawn_local(async move {
        let args = serde_wasm_bindgen::to_value(&app_settings_cp).unwrap();
        console_log(&format!(
            "Invoking download_replay_caches with args: {:?}",
            args
        ));
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        let response = invoke("download_replay_caches", args).await;
        console_log(&format!("download_replay_caches response: {:?}", response));
        match serde_wasm_bindgen::from_value::<ApiResponse>(response) {
            Ok(res) => {
                if res.meta.success {
                    console_log("Download replay caches succeeded.");
                } else {
                    console_log(&format!("Download replay caches failed: {:?}", res.message));
                }
                set_backend_response.set(res);
            }
            Err(e) => {
                console_log(&format!("Error invoking download_replay_caches: {:?}", e));
                set_backend_response.set(ApiResponse {
                    meta: ResponseMeta {
                        success: false,
                        duration_ms: 0,
                        is_complete: true,
                    },
                    message: format!("Error invoking download_replay_caches: {:?}", e),
                });
            }
        }
        *set_activity_stage.write() = ActivityStage::DownloadingCachesDone;
    });
}

