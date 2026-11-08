//! Swarmy Tauri UI Library

pub mod battlenet;
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
pub mod ability_detail;
pub mod ability_usage;
#[cfg(test)]
mod bench_queries;
pub mod commands;
pub mod data;
pub mod duplicates;
pub mod host;
pub mod logging;
pub mod majordomo;
pub mod map_details;
pub mod map_scout;
pub mod patch_overview;
pub mod personal_db;
pub mod player_detail;
pub mod progress;
pub mod race_detail;
pub mod remote;
pub mod replay_detail;
pub mod replay_list;
pub mod replay_rerun;
pub mod replay_watcher;
pub mod server;
pub mod swarmy_bevy;
pub mod unit_icons;
pub mod usage_over_time;
pub mod web;
use crate::replay_list::{connect_swarmy_rerun, copy_path_to_clipboard, open_folder};
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
    let args = <logging::Args as clap::Parser>::parse();
    logging::init(&args);
    // Once in the binary: the window and the browser version use the same frontend.
    let context = tauri::generate_context!();
    if let Some(addr) = args.serve {
        // The remote backend of other swarmy apps and the browser version, without a window.
        let web = web::WebAssets {
            root: args.web_root,
            ..web::WebAssets::from_context(&context)
        };
        let options = server::ServeOptions {
            data_dir: args.data_dir,
            token: args.token,
            admin_key_file: args.admin_key_file,
            web,
        };
        if let Err(err) = tauri::async_runtime::block_on(server::serve(addr, options)) {
            tracing::error!("The server stopped: {err}");
            process::exit(1);
        }
        return;
    }
    let (majordomo_tx, majordomo_rx) = mpsc::channel(4_096); // TODO: Magic number removal
    let builder = tauri::Builder::default().plugin(tauri_plugin_clipboard_manager::init());
    // The updater is for the desktop bundles, Android and iOS update through their stores.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
        .plugin(tauri_plugin_shell::init())
        .manage(SetupState { majordomo_tx })
        .manage(RerunViewerState::default())
        .manage(std::sync::Arc::new(
            replay_watcher::ReplayWatcherState::default(),
        ))
        .setup(|app| {
            tracing::info!(
                "Starting Tauri application setup {}:{:?}",
                process::id(),
                thread::current().id()
            );
            // Spawn setup as a non-blocking task so the windows can be
            // created and ran while it executes
            spawn(setup(majordomo_rx, app.handle().clone()));
            replay_watcher::configure(std::sync::Arc::new(app.handle().clone()), false);
            remote::relay_events(app.handle().clone());
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
        .invoke_handler(remote::invoke_handler(
            commands::backend_handler(),
            tauri::generate_handler![
                get_current_app_config,
                set_current_app_config,
                settings::get_ui_settings,
                settings::zoom_step,
                settings::pick_folder,
                remote::check_remote_backend,
                swarmy_bevy::exec_swarmy_bevy_map_caches,
                swarmy_bevy::exec_swarmy_bevy_replay,
                swarmy_bevy::exec_swarmy_bevy_siege_spots,
                swarmy_bevy::exec_swarmy_bevy_map_layers,
                copy_path_to_clipboard,
                open_folder,
                connect_swarmy_rerun,
                replay_detail::query_replay_lesson,
                replay_detail::export_replay_report,
                patch_overview::list_patch_units,
                unit_icons::list_unit_icons,
                patch_overview::query_unit_overview,
                patch_overview::query_patch_changes,
                patch_overview::list_evolution_items,
                patch_overview::query_patch_evolution,
                patch_overview::list_saved_evolutions,
                patch_overview::save_evolution,
                get_app_versions,
                open_external_url,
            ],
        ))
        .plugin(tauri_plugin_store::Builder::default().build())
        // The zoom of the last session, before the first view is shown: the webview loads its
        // page at 100%.
        .on_page_load(|webview, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                match settings::load_stored_app_settings(webview.app_handle()) {
                    Ok(settings) => {
                        settings::apply_zoom(webview.app_handle(), settings.zoom_percent)
                    }
                    Err(err) => tracing::warn!("Unable to read the zoom of the settings: {err}"),
                }
            }
        })
        .build(context)
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
    let _ = majordomo::MajordomoCoordinator::init_coordinator_thread(rx, std::sync::Arc::new(app));
    Ok(())
}
