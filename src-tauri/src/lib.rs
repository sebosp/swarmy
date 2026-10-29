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
pub mod ability_usage;
#[cfg(test)]
mod bench_queries;
pub mod data;
pub mod duplicates;
pub mod logging;
pub mod majordomo;
pub mod map_details;
pub mod map_scout;
pub mod patch_overview;
pub mod personal_db;
pub mod player_detail;
pub mod progress;
pub mod race_detail;
pub mod replay_detail;
pub mod replay_list;
pub mod replay_rerun;
pub mod replay_watcher;
pub mod swarmy_bevy;
pub mod unit_icons;
use crate::replay_list::{
    connect_swarmy_rerun, copy_path_to_clipboard, open_folder, query_replay_list,
};
use crate::replay_rerun::viewer::RerunViewerState;
use std::process;
use std::thread;

use tauri::async_runtime::spawn;
use tauri::{AppHandle, Manager, RunEvent};
use tokio::sync::mpsc;

use crate::majordomo::AsyncTask;

#[derive(Debug)]
pub struct SetupState {
    majordomo_tx: mpsc::Sender<AsyncTask>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init(&<logging::Args as clap::Parser>::parse());
    let (majordomo_tx, majordomo_rx) = mpsc::channel(4_096); // TODO: Magic number removal
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(SetupState { majordomo_tx })
        .manage(RerunViewerState::default())
        .manage(replay_watcher::ReplayWatcherState::default())
        .setup(|app| {
            tracing::info!(
                "Starting Tauri application setup {}:{:?}",
                process::id(),
                thread::current().id()
            );
            // Spawn setup as a non-blocking task so the windows can be
            // created and ran while it executes
            spawn(setup(majordomo_rx, app.handle().clone()));
            replay_watcher::configure(app.handle());
            tracing::info!(
                "Setup function called on {}:{:?}",
                process::id(),
                thread::current().id()
            );
            // The hook expects an Ok result
            Ok(())
        })
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_persisted_scope::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_current_app_config,
            set_current_app_config,
            basic_scan_replay_path,
            optimize_replay_path,
            get_snapshot_metadata,
            settings::pick_folder,
            query_map_stats,
            swarmy_bevy::exec_swarmy_bevy_map_caches,
            swarmy_bevy::exec_swarmy_bevy_replay,
            swarmy_bevy::exec_swarmy_bevy_siege_spots,
            swarmy_bevy::query_siege_spots,
            query_replay_list,
            replay_list::list_replay_filters,
            replay_watcher::get_new_replays_status,
            copy_path_to_clipboard,
            open_folder,
            connect_swarmy_rerun,
            replay_detail::query_replay_detail,
            replay_detail::query_adjacent_replay,
            personal_db::get_replay_notes,
            duplicates::find_duplicate_replays,
            duplicates::move_duplicate_replays,
            personal_db::set_replay_notes,
            personal_db::list_replay_tags,
            player_detail::query_player_detail,
            player_detail::list_players,
            player_detail::query_head_to_head,
            map_scout::query_map_scout,
            map_details::query_map_details,
            map_details::list_maps,
            patch_overview::list_patch_units,
            unit_icons::list_unit_icons,
            patch_overview::query_unit_overview,
            patch_overview::query_unit_snapshot_stats,
            patch_overview::query_patch_changes,
            patch_overview::list_evolution_items,
            patch_overview::query_patch_evolution,
            patch_overview::list_saved_evolutions,
            patch_overview::save_evolution,
            ability_usage::query_ability_usage,
            race_detail::query_race_detail,
            progress::query_progress,
            progress::set_progress_goals,
            get_app_versions,
            open_external_url,
        ])
        .plugin(tauri_plugin_store::Builder::default().build())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Sidecars spawned from rust are not killed by the shell plugin on exit.
            if let RunEvent::Exit = event {
                app.state::<RerunViewerState>().kill();
            }
        });
}

// An async function that does some heavy setup task
async fn setup(rx: mpsc::Receiver<AsyncTask>, app: AppHandle) -> Result<(), ()> {
    tracing::info!(
        "Starting majordomo setup task{}:{:?}",
        process::id(),
        thread::current().id()
    );
    let _ = majordomo::MajordomoCoordinator::init_coordinator_thread(rx, app);
    Ok(())
}
