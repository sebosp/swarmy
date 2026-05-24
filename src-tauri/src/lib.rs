//! Swarmy Tauri UI Library

pub mod mpq_file_scan;
pub use mpq_file_scan::*;
pub mod settings;
pub use settings::*;
pub mod snapshot_stats;
pub use snapshot_stats::*;
pub mod common;
pub use common::*;
pub mod map_stats;
pub use map_stats::*;
pub mod map_details;
pub mod replay_caches;
pub use replay_caches::*;
pub mod data;
pub mod majordomo;
pub mod replay_list;
pub mod rerun;
use crate::replay_list::{
    connect_swarmy_rerun, copy_path_to_clipboard, open_folder, query_replay_list,
};
use std::process;
use std::thread;

use tauri::AppHandle;
use tauri::async_runtime::spawn;
use tokio::sync::mpsc;

use crate::majordomo::AsyncTask;

#[derive(Debug)]
pub struct SetupState {
    majordomo_tx: mpsc::Sender<AsyncTask>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let (majordomo_tx, majordomo_rx) = mpsc::channel(4_096); // TODO: Magic number removal
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(SetupState { majordomo_tx })
        .setup(|app| {
            log::info!(
                "Starting Tauri application setup {}:{:?}",
                process::id(),
                thread::current().id()
            );
            // Spawn setup as a non-blocking task so the windows can be
            // created and ran while it executes
            spawn(setup(majordomo_rx, app.handle().clone()));
            log::info!(
                "Setup function called on {}:{:?}",
                process::id(),
                thread::current().id()
            );
            // The hook expects an Ok result
            Ok(())
        })
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_current_app_config,
            basic_scan_replay_path,
            optimize_replay_path,
            get_snapshot_metadata,
            query_map_stats,
            download_replay_caches,
            exec_swarmy_bevy_map_caches,
            query_replay_list,
            copy_path_to_clipboard,
            open_folder,
            connect_swarmy_rerun,
        ])
        .plugin(tauri_plugin_store::Builder::default().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// An async function that does some heavy setup task
async fn setup(rx: mpsc::Receiver<AsyncTask>, app: AppHandle) -> Result<(), ()> {
    log::info!(
        "Starting majordomo setup task{}:{:?}",
        process::id(),
        thread::current().id()
    );
    let _ = majordomo::MajordomoCoordinator::init_coordinator_thread(rx, app);
    Ok(())
}
