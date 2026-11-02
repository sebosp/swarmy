//! Swarmy Tauri UI - Scan View

use super::actions::*;
use super::arrow_ipc_stats::ArrowIpcStats;
use super::duplicates::DuplicatesPanel;
use super::mpq_file_scan::ReplayScanTable;
use super::optimize_progress::OptimizeProgress;
use super::*;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::leptos_dom::logging::console_log;
use leptos::prelude::*;
use phosphor_leptos::{Icon, IconWeight, ARROWS_CLOCKWISE, ARROW_COUNTER_CLOCKWISE, DATABASE};
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
    let (confirm_rebuild, set_confirm_rebuild) = signal(false);

    let navigation = crate::navigation::use_navigation();
    crate::settings::with_fetch_current_app_config(
        set_app_settings,
        move |settings: AppSettings| {
            *set_snapshot_stats.write() = settings.snapshot_stats.clone();
            *set_activity_stage.write() = ActivityStage::from(settings);
            // The new replays banner asked to add the new replays.
            if navigation.optimize_requested.get_untracked() {
                navigation.optimize_requested.set(false);
                trigger_optimize_replay_path(
                    false,
                    app_settings,
                    set_backend_response,
                    set_activity_stage,
                    set_snapshot_stats,
                );
            }
        },
    );
    let dir_stats_data = Store::new(SC2ReplaysDirStatsTable::from(SC2ReplaysDirStats::default()));
    let open_config = move |ev: MouseEvent| {
        ev.prevent_default();
        navigation.active_page.set("Config".to_string());
    };
    // A snapshot to refresh, or to rebuild.
    let has_snapshot =
        move || snapshot_stats.get().num_games > 0 || snapshot_stats.get().ipc_dir_size > 0;

    view! {
        <div class="grid grid-cols-10">
            <div class="col-span-5 pl-1">
                // The replay path is changed in Config only, an input here would not be saved.
                <p class="block text-sm/6 font-medium text-base-content">
                    {move || {
                        let settings = app_settings.get();
                        if settings.is_remote() {
                            view! {
                                "Remote Replay Path on "
                                <a href="#" class="nav-link font-mono" title="Change the remote backend in Config" on:click=open_config>
                                    {settings.remote_host().to_string()}
                                </a>
                            }
                                .into_any()
                        } else {
                            "Replay Path".into_any()
                        }
                    }}
                </p>
                <p id="scan-directory-path" class="text-sm font-mono text-base-content break-all">
                    {move || {
                        let replay_path = app_settings.get().snapshot_replay_path();
                        if !replay_path.is_empty() {
                            replay_path
                        } else if app_settings.get().is_remote() {
                            "Not sent by the backend".to_string()
                        } else {
                            "Not set".to_string()
                        }
                    }}
                </p>
                <p class="text-xs text-muted">
                    {move || {
                        let settings = app_settings.get();
                        if !settings.is_remote() {
                            view! { <a href="#" class="nav-link" on:click=open_config>"Change it in Config"</a> }
                                .into_any()
                        } else if settings.snapshot_replay_path().is_empty() {
                            "The server runs an older swarmy that does not send its replay path, update it to 0.2.3 or later to see it here."
                                .into_any()
                        } else {
                            view! {
                                "Set in the "
                                <a href="#" class="nav-link" on:click=open_config>"Config"</a>
                                " of that computer, it cannot be changed from here."
                            }
                                .into_any()
                        }
                    }}
                </p>
            </div>
            // The actions, evenly spaced on one row.
            <div class="col-span-5 flex flex-wrap items-center gap-2 ml-6 mt-6">
                <button
                    class=move || {
                        if activity_stage.get() > ActivityStage::None {
                            "btn btn-primary btn-sm"
                        } else {
                            "btn btn-disabled btn-sm disabled:cursor-not-allowed disabled:bg-base-300 disabled:text-faint disabled:outline-base-content/20"
                        }
                    }
                    on:click=move |ev: MouseEvent| trigger_basic_scan_replay_path(
                        ev,
                        app_settings,
                        set_backend_response,
                        dir_stats_data,
                        set_activity_stage,
                    )
                    disabled=move || { !app_settings.get().can_scan() }
                    title="Counts the StarCraft II replays of the replay path and their players, without changing the snapshot"
                >
                    <Icon
                        icon=(move || { activity_stage.get().icon_data() })()
                        weight=IconWeight::Light
                        prop:class="stroke-current"
                    />
                    "Scan"
                </button>
                // The same command before and after the snapshot exists: the first time it creates
                // it, then it adds the new replays.
                <button
                    class=move || {
                        if app_settings.get().can_scan() {
                            "btn btn-accent btn-sm"
                        } else {
                            "btn btn-disabled btn-sm disabled:cursor-not-allowed disabled:bg-base-300 disabled:text-faint disabled:outline-base-content/20"
                        }
                    }
                    on:click=move |_| trigger_optimize_replay_path(
                        false,
                        app_settings,
                        set_backend_response,
                        set_activity_stage,
                        set_snapshot_stats,
                    )
                    disabled=move || {
                        activity_stage.get() == ActivityStage::DirectoryEntered
                            && activity_stage.get() != ActivityStage::ScanInit
                            && activity_stage.get() != ActivityStage::OptimizeInit
                    }
                    title=move || {
                        if has_snapshot() {
                            "Adds the replays that are not in the snapshot yet and downloads their map caches, the replays already in it are kept: quick when there are few new replays."
                        } else {
                            "Reads the replays into a snapshot of Arrow files per day (may take some time) and downloads their map caches from Blizzard's depot servers, the digest of each map version comes from them."
                        }
                    }
                >
                    {move || {
                        if has_snapshot() {
                            view! {
                                <Icon icon=ARROWS_CLOCKWISE weight=IconWeight::Light prop:class="stroke-current" />
                                "Refresh"
                            }
                                .into_any()
                        } else {
                            view! {
                                <Icon icon=DATABASE weight=IconWeight::Light prop:class="stroke-current" />
                                "Optimize and Download Caches"
                            }
                                .into_any()
                        }
                    }}
                </button>
                // Deleting the snapshot needs a second click; it is needed when the snapshot was
                // written by an older s2protocol, then it stands out.
                <Show when=has_snapshot>
                    <button
                        class=move || {
                            if confirm_rebuild.get() {
                                "btn btn-error btn-sm"
                            } else if !snapshot_stats.get().is_compatible {
                                "btn btn-warning btn-sm"
                            } else {
                                "btn btn-outline btn-sm"
                            }
                        }
                        on:click=move |_| {
                            if confirm_rebuild.get() {
                                set_confirm_rebuild.set(false);
                                trigger_optimize_replay_path(
                                    true,
                                    app_settings,
                                    set_backend_response,
                                    set_activity_stage,
                                    set_snapshot_stats,
                                );
                            } else {
                                set_confirm_rebuild.set(true);
                            }
                        }
                        on:mouseleave=move |_| set_confirm_rebuild.set(false)
                        disabled=move || {
                            !app_settings.get().can_scan()
                                || activity_stage.get() == ActivityStage::OptimizeInit
                        }
                        title="Deletes the snapshot and reads all the replays again: needed when the snapshot was written by an older s2protocol, much slower than Refresh"
                    >
                        <Icon icon=ARROW_COUNTER_CLOCKWISE weight=IconWeight::Light prop:class="stroke-current" />
                        {move || {
                            if confirm_rebuild.get() {
                                "Click again to delete and rebuild"
                            } else {
                                "Rebuild from scratch"
                            }
                        }}
                    </button>
                </Show>
            </div>
            <DisplayBackendStatus backend_response />
            <div class="col-span-10 m-0 p-0">
                <SnapshotStatusHeader app_settings=app_settings dir_stats_data activity_stage />
                <OptimizeProgress />
                <Show when=move || { activity_stage.get() == ActivityStage::ScanDone }>
                    <ReplayScanTable dir_stats_data />
                </Show>
                <Show when=move || { activity_stage.get() >= ActivityStage::OptimizeDone }>
                    <ArrowIpcStats snapshot_stats=snapshot_stats />
                </Show>
                // The remote backend finds and moves its duplicates with its own Config, in the
                // Scan of its computer.
                <Show when=move || !app_settings.get().is_remote()>
                    <DuplicatesPanel app_settings />
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
                        {move || {
                            let settings = app_settings.get();
                            if settings.is_remote() {
                                format!("{} backend status: ", settings.remote_host())
                            } else {
                                "Backend status: ".to_string()
                            }
                        }}
                        {move || activity_stage.get().text_content()}
                    </p>
                </div>
            </div>
        </div>
    }
}
