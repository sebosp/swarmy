//! The banner shown when the replay directory watcher finds replays that are not in the
//! snapshot, see the `watch_new_replays` setting.

use crate::invoke_json;
use crate::navigation::use_navigation;
use crate::scan::optimize_progress::TauriEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, DATABASE};
use swarmy_common::{Lesson, NewReplaysStatus, NEW_REPLAYS_EVENT};
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

/// The message of a new replay is shown for this long.
const NEW_REPLAY_TOAST_SECONDS: u64 = 15;

/// The replay is analysed for the lesson once StarCraft II finished writing it.
const LESSON_DELAY_SECONDS: u64 = 3;

#[component]
pub fn NewReplaysBanner() -> impl IntoView {
    let navigation = use_navigation();
    let status = RwSignal::new(NewReplaysStatus::default());
    listen_to_new_replays(status);
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
                        if let Ok(found) = invoke_json::<Option<Lesson>>(
                            "query_replay_lesson",
                            &args,
                        )
                        .await
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
    let message = move || {
        let status = status.get();
        let mut parts = vec![];
        if status.new_replays > 0 {
            parts.push(format!(
                "{} new replays are not in the snapshot",
                status.new_replays
            ));
        }
        if status.outdated_days > 0 {
            parts.push(format!(
                "{} days were parsed by another version of s2protocol",
                status.outdated_days
            ));
        }
        format!(
            "{}. Rebuild only parses these replays, the others are kept.",
            parts.join(", ")
        )
    };
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
            <div class="flex items-center justify-between gap-2 px-3 py-1 text-sm text-indigo-100 bg-indigo-900/60 border-b border-indigo-500/40">
                <span>{message}</span>
                <button
                    class="btn btn-accent btn-xs"
                    title="Opens Scan and runs Optimize, it only adds the replays that are not in the snapshot"
                    on:click=move |_| navigation.optimize_snapshot()
                >
                    <Icon icon=DATABASE weight=IconWeight::Light prop:class="stroke-current" />
                    "Rebuild"
                </button>
            </div>
        </Show>
    }
}
