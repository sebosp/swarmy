//! The banner shown when the replay directory watcher finds replays that are not in the
//! snapshot, see the `watch_new_replays` setting.

use crate::invoke_json;
use crate::navigation::use_navigation;
use crate::scan::optimize_progress::TauriEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, DATABASE};
use swarmy_common::{NewReplaysStatus, NEW_REPLAYS_EVENT};
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

#[component]
pub fn NewReplaysBanner() -> impl IntoView {
    let navigation = use_navigation();
    let status = RwSignal::new(NewReplaysStatus::default());
    listen_to_new_replays(status);
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
