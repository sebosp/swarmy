//! Swarmy Tauri UI - Scan View

use super::actions::*;
use super::arrow_ipc_stats::ArrowIpcStats;
use super::mpq_file_scan::ReplayScanTable;
use super::*;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::leptos_dom::logging::console_log;
use leptos::prelude::*;
use phosphor_leptos::{DATABASE, Icon, IconWeight, SHIPPING_CONTAINER};
use reactive_graph::traits::Write;
use reactive_stores::Store;
use s2protocol::dir_stats::SC2ReplaysDirStats;
use swarmy_common::*;

#[component]
pub fn ScanDirectory() -> impl IntoView {
    let (app_settings, set_app_settings) = signal(AppSettings::default());
    let (activity_stage, set_activity_stage) = signal(ActivityStage::default());

    let (backend_response, set_backend_response) = signal(ApiResponse {
        meta: ResponseMeta::incomplete(),
        message: String::new(),
    });
    let (snapshot_stats, set_snapshot_stats) = signal(SnapshotStats::default());

    crate::settings::with_fetch_current_app_config(
        set_app_settings,
        move |settings: AppSettings| {
            *set_snapshot_stats.write() = settings.snapshot_stats.clone();
            *set_activity_stage.write() = ActivityStage::from(settings);
        },
    );
    let tx_update_replay_dir = move |ev| {
        let v = event_target_value(&ev);
        set_app_settings.update(|settings| {
            settings.replay_path = v;
        });
        *set_activity_stage.write() = ActivityStage::DirectoryEntered;
    };
    let dir_stats_data = Store::new(SC2ReplaysDirStatsTable::from(SC2ReplaysDirStats::default()));

    view! {
        <div class="grid grid-cols-10">
            <div class="col-span-5 pl-1">
                <label for="scan_path" class="block text-sm/6 font-medium text-white">
                    Path
                </label>
                <input
                    name="scan_path"
                    class=text_input_tailwind_classes().join(" ")
                    id="scan-directory-input"
                    value=move || app_settings.get().replay_path
                    on:input=tx_update_replay_dir
                    type="text"
                />
            </div>
            <div class="col-span-5 justify-start ml-6 mt-6">
                <button
                    class=move || {
                        if activity_stage.get() > ActivityStage::None {
                            "btn btn-primary btn-sm b-0"
                        } else {
                            "btn btn-disabled btn-sm b-0 disabled:cursor-not-allowed disabled:bg-gray-50 disabled:text-gray-500 disabled:outline-gray-200"
                        }
                    }
                    on:click=move |ev: MouseEvent| trigger_basic_scan_replay_path(
                        ev,
                        app_settings,
                        set_backend_response,
                        dir_stats_data,
                        set_activity_stage,
                    )
                    disabled=move || { app_settings.get().replay_path.is_empty() }
                    title="Initial scan for StarCraft II replays"
                >
                    <Icon
                        icon=(move || { activity_stage.get().icon_data() })()
                        weight=IconWeight::Light
                        prop:class="stroke-current"
                    />
                    "Scan"
                </button>
                <button
                    class=move || {
                        if !app_settings.get().replay_path.is_empty() {
                            "btn btn-accent btn-sm"
                        } else {
                            "btn btn-disabled btn-sm disabled:cursor-not-allowed disabled:bg-gray-50 disabled:text-gray-500 disabled:outline-gray-200"
                        }
                    }
                    on:click=move |_| trigger_optimize_replay_path(
                        app_settings,
                        set_backend_response,
                        set_activity_stage,
                    )
                    disabled=move || {
                        activity_stage.get() == ActivityStage::DirectoryEntered
                            && activity_stage.get() != ActivityStage::ScanInit
                            && activity_stage.get() != ActivityStage::OptimizeInit
                    }
                    title="Optimize the replay generating Arrow files (may take some time)"
                >
                    <Icon icon=DATABASE weight=IconWeight::Light prop:class="stroke-current" />
                    "Optimize"
                </button>
                <button
                    class=move || {
                        if !app_settings.get().replay_path.is_empty() {
                            "btn btn-info btn-sm"
                        } else {
                            "btn btn-disabled btn-sm disabled:cursor-not-allowed disabled:bg-gray-50 disabled:text-gray-500 disabled:outline-gray-200"
                        }
                    }
                    on:click=move |_| trigger_download_replay_caches(
                        app_settings,
                        set_backend_response,
                        set_activity_stage,
                    )
                    disabled=move || {
                        activity_stage.get() == ActivityStage::DirectoryEntered
                            && activity_stage.get() != ActivityStage::OptimizeInit
                    }
                    title="Downloads the caches from Starcraft II servers that contain map information such as Height Map"
                >
                    <Icon
                        icon=SHIPPING_CONTAINER
                        weight=IconWeight::Light
                        prop:class="stroke-current"
                    />
                    "Download Caches"
                </button>
            </div>
            <DisplayBackendStatus backend_response />
            <div class="col-span-10 m-0 p-0">
                <SnapshotStatusHeader app_settings=app_settings dir_stats_data activity_stage />
                <Show when=move || { activity_stage.get() == ActivityStage::ScanDone }>
                    <ReplayScanTable dir_stats_data />
                </Show>
                <Show when=move || { activity_stage.get() >= ActivityStage::OptimizeDone }>
                    <ArrowIpcStats snapshot_stats=snapshot_stats />
                </Show>
            </div>
        </div>
    }
}
#[component]
pub fn SnapshotStatusHeader(
    app_settings: ReadSignal<AppSettings>,
    dir_stats_data: Store<SC2ReplaysDirStatsTable>,
    activity_stage: ReadSignal<ActivityStage>,
) -> impl IntoView {
    let has_replay_files = move || dir_stats_data.total_files().get() > 0;
    let has_optimized_snapshot = move || app_settings.get().snapshot_stats.ipc_dir_size > 0;
    let has_caches_downloaded = move || app_settings.get().snapshot_stats.num_caches > 0;
    console_log(&format!(
        "has_replay_files {} has_optimized_snapshot {} has_caches_downloaded {}",
        has_replay_files(),
        has_optimized_snapshot(),
        has_caches_downloaded()
    ));
    view! {
        <div class=move || { activity_stage.get().top_container_class() }>
            <div class="flex">
                <div class=move || { activity_stage.get().alert_container_class() }>
                    <Icon
                        icon=(move || { activity_stage.get().icon_data() })()
                        weight=IconWeight::Bold
                        prop:class="stroke-current"
                    />
                </div>
                <div class="ml-3">
                    <p class=move || { activity_stage.get().text_class().join(" ") }>
                        <code>{move || app_settings.get().replay_path}</code>
                        ": "
                        {move || activity_stage.get().text_content()}
                    </p>
                </div>
            </div>
        </div>
    }
}
