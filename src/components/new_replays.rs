//! The banner shown when the replay directory watcher finds replays that are not in the
//! snapshot, see the `watch_new_replays` setting.

use crate::invoke_json;
use crate::navigation::use_navigation;
use crate::scan::optimize_progress::TauriEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, DATABASE};
use s2protocol::snapshot::SnapshotProgress;
use swarmy_common::{
    AppSettings, Lesson, NewReplaysStatus, NEW_REPLAYS_EVENT, OPTIMIZE_PROGRESS_EVENT,
};
use wasm_bindgen::prelude::*;

/// Listens to the watcher for as long as the app runs.
fn listen_to_new_replays(status: RwSignal<NewReplaysStatus>) {
    spawn_local(async move {
        match invoke_json::<NewReplaysStatus>("get_new_replays_status", &()).await {
            Ok(current) => status.set(current),
            Err(err) => leptos::logging::error!("Error reading the new replays: {err}"),
        }
    });
    let handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        match serde_wasm_bindgen::from_value::<TauriEvent<NewReplaysStatus>>(event) {
            Ok(event) => status.set(event.payload),
            Err(err) => leptos::logging::error!("Invalid new replays status: {err:?}"),
        }
    });
    spawn_local(async move {
        let _ = crate::listen(NEW_REPLAYS_EVENT, &handler).await;
        // The banner is mounted once, the handler lives as long as the app.
        handler.forget();
    });
}

/// Listens to the progress of Optimize, here or in the remote backend: `updated` is set to the
/// replays it read when it finishes.
fn listen_to_optimize(updated: RwSignal<Option<usize>>) {
    let read = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        match serde_wasm_bindgen::from_value::<TauriEvent<SnapshotProgress>>(event) {
            Ok(event) => match event.payload {
                SnapshotProgress::ValidReplays { count } => read.set(count),
                SnapshotProgress::Done { .. } => updated.set(Some(read.get())),
                _ => {}
            },
            Err(err) => leptos::logging::error!("Invalid optimize progress: {err:?}"),
        }
    });
    spawn_local(async move {
        let _ = crate::listen(OPTIMIZE_PROGRESS_EVENT, &handler).await;
        handler.forget();
    });
}

/// The message of a new replay is shown for this long.
const NEW_REPLAY_TOAST_SECONDS: u64 = 15;

/// The replay is analysed for the lesson once StarCraft II finished writing it.
const LESSON_DELAY_SECONDS: u64 = 3;

#[component]
pub fn NewReplaysBanner() -> impl IntoView {
    let navigation = use_navigation();
    let received = RwSignal::new(NewReplaysStatus::default());
    // Only the administrator of the browser version is told about them, once.
    let session = crate::components::web_session::use_web_session();
    let listening = RwSignal::new(false);
    Effect::new(move |_| {
        if session.admin.get() && !listening.get_untracked() {
            listening.set(true);
            listen_to_new_replays(received);
        }
    });
    // The settings of this computer: with a remote backend the status is the one of the backend,
    // shown when this computer asked to receive it, with the messages it asked for.
    let settings = RwSignal::new(None::<AppSettings>);
    Effect::new(move |_| {
        received.track();
        spawn_local(async move {
            if let Ok(current) = serde_wasm_bindgen::from_value::<AppSettings>(
                crate::invoke_without_args("get_ui_settings").await,
            ) {
                settings.set(Some(current));
            }
        });
    });
    let status = Memo::new(move |_| {
        let mut status = received.get();
        if let Some(settings) = settings.get().filter(AppSettings::is_remote) {
            if !settings.watch_new_replays {
                status = NewReplaysStatus::default();
            }
            status.notify = settings.watch_new_replays && settings.notify_new_replays;
        }
        status
    });
    // With the notify_new_replays setting, each replay the watcher finds is shown for a while.
    let toast = RwSignal::new(None::<String>);
    // The most costly lesson of the Main Player Name of Config in the new replay.
    let lesson = RwSignal::new(None::<Lesson>);
    Effect::new(move |previous: Option<Option<String>>| {
        let current = status.read().latest_replay.clone();
        let notify = status.read().notify;
        if notify && current.is_some() && previous.as_ref() != Some(&current) && previous.is_some()
        {
            let name = current
                .as_deref()
                .and_then(|path| path.rsplit(['/', '\\']).next())
                .unwrap_or_default()
                .to_string();
            toast.set(Some(name.clone()));
            lesson.set(None);
            let location = current.clone().unwrap_or_default();
            set_timeout(
                move || {
                    spawn_local(async move {
                        #[derive(serde::Serialize)]
                        struct Args {
                            replay_location: String,
                        }
                        let args = Args {
                            replay_location: location,
                        };
                        if let Ok(found) =
                            invoke_json::<Option<Lesson>>("query_replay_lesson", &args).await
                        {
                            lesson.set(found);
                        }
                    });
                },
                std::time::Duration::from_secs(LESSON_DELAY_SECONDS),
            );
            set_timeout(
                move || {
                    if toast.get_untracked().as_deref() == Some(name.as_str()) {
                        toast.set(None);
                    }
                },
                std::time::Duration::from_secs(NEW_REPLAY_TOAST_SECONDS),
            );
        }
        current
    });
    // What the banner says: the new replays to add, what Refresh does.
    let message = move || {
        let status = status.get();
        let mut message = match status.new_replays {
            0 => String::new(),
            1 => "1 new replay available. Refresh the snapshot to add it.".to_string(),
            count => format!("{count} new replays available. Refresh the snapshot to add them."),
        };
        if status.outdated_days > 0 {
            message.push_str(&format!(
                " {} days were read by an older version of swarmy, Refresh reads them again.",
                status.outdated_days
            ));
        }
        if status.missing_aggregates > 0 {
            message.push_str(&format!(
                " The statistics of {} periods are not computed yet, Refresh computes them.",
                status.missing_aggregates
            ));
        }
        message.trim().to_string()
    };
    // An Optimize finished, here or in the remote backend: the views show the games they loaded,
    // a reload shows the new ones. Not shown on Scan, which shows the result.
    let updated = RwSignal::new(None::<usize>);
    listen_to_optimize(updated);
    view! {
        {move || {
            toast
                .get()
                .map(|name| {
                    view! {
                        <div class="toast toast-end toast-bottom z-50">
                            <div class="alert alert-info text-sm">
                                <span>
                                    {format!("New replay found: {name}")}
                                    {move || {
                                        lesson
                                            .get()
                                            .map(|lesson| {
                                                view! {
                                                    <br />
                                                    <span class="font-semibold">
                                                        {format!("To fix: {}", lesson.title)}
                                                    </span>
                                                    {format!(", {}", lesson.detail)}
                                                }
                                            })
                                    }}
                                </span>
                                <button
                                    class="btn btn-accent btn-xs"
                                    title="Opens Scan and runs Optimize, it only adds the replays that are not in the snapshot"
                                    on:click=move |_| {
                                        toast.set(None);
                                        navigation.optimize_snapshot();
                                    }
                                >
                                    "Add to snapshot"
                                </button>
                                <button
                                    class="btn btn-ghost btn-xs"
                                    title="Close"
                                    on:click=move |_| toast.set(None)
                                >
                                    "✕"
                                </button>
                            </div>
                        </div>
                    }
                })
        }}
        <Show when=move || status.get().needs_optimize()>
            <div class="flex items-center justify-between gap-2 px-3 py-1 text-sm text-primary bg-primary/20 border-b border-primary/40">
                <span>{message}</span>
                <button
                    class="btn btn-accent btn-xs"
                    title="Opens Scan and adds the new replays to the snapshot, the others are kept"
                    on:click=move |_| navigation.optimize_snapshot()
                >
                    <Icon icon=DATABASE weight=IconWeight::Light prop:class="stroke-current" />
                    "Refresh snapshot"
                </button>
            </div>
        </Show>
        <Show when=move || {
            updated.get().is_some() && navigation.active_page.get() != crate::navigation::SCAN_PAGE
        }>
            <div class="flex items-center justify-between gap-2 px-3 py-1 text-sm text-success bg-success/15 border-b border-success/40">
                <span>
                    {move || match updated.get().unwrap_or_default() {
                        0 => "The snapshot was updated. Reload the view to see it.".to_string(),
                        1 => "The snapshot was updated: 1 new game. Reload the view to see it.".to_string(),
                        games => format!("The snapshot was updated: {games} new games. Reload the view to see them."),
                    }}
                </span>
                <span class="flex gap-1">
                    <button
                        class="btn btn-success btn-xs"
                        title="Shows the view again with the games of the snapshot"
                        on:click=move |_| {
                            updated.set(None);
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().reload();
                            }
                        }
                    >
                        "Reload view"
                    </button>
                    <button class="btn btn-ghost btn-xs" title="Close" on:click=move |_| updated.set(None)>
                        "✕"
                    </button>
                </span>
            </div>
        </Show>
    }
}
