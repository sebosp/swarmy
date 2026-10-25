//! The progress of Optimize, sent by the backend as Tauri events while the snapshot is written.

use crate::components::section::Panel;
use leptos::prelude::*;
use leptos::task::spawn_local;
use s2protocol::snapshot::{SnapshotFile, SnapshotProgress};
use serde::Deserialize;
use si_scale::helpers::bibytes2;
use swarmy_common::OPTIMIZE_PROGRESS_EVENT;
use wasm_bindgen::prelude::*;

/// The envelope of a Tauri event.
#[derive(Deserialize)]
pub struct TauriEvent<T> {
    pub payload: T,
}

/// The progress so far, built from the updates received.
#[derive(Debug, Clone, Default)]
struct ProgressState {
    files_found: Option<usize>,
    valid_replays: Option<usize>,
    already_in_snapshot: Option<usize>,
    /// The steps started, the last one is in progress until Done.
    steps: Vec<(usize, usize, String)>,
    replays_processed: Option<(usize, usize)>,
    files: Vec<SnapshotFile>,
    elapsed_ms: Option<u64>,
}

impl ProgressState {
    fn apply(&mut self, progress: SnapshotProgress) {
        match progress {
            SnapshotProgress::Step { index, total, name } => {
                // The first step starts a new Optimize run.
                if index == 1 {
                    *self = Self::default();
                }
                self.steps.push((index, total, name));
            }
            SnapshotProgress::FilesFound { count } => self.files_found = Some(count),
            SnapshotProgress::ValidReplays { count } => self.valid_replays = Some(count),
            SnapshotProgress::AlreadyInSnapshot { count } => self.already_in_snapshot = Some(count),
            SnapshotProgress::ReplaysProcessed { done, total } => {
                self.replays_processed = Some((done, total))
            }
            SnapshotProgress::FileWritten(file) => self.files.push(file),
            SnapshotProgress::Done { elapsed_ms } => self.elapsed_ms = Some(elapsed_ms),
        }
    }

    fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// Listens to the Optimize progress events for as long as the component is mounted.
fn listen_to_progress(set_state: WriteSignal<ProgressState>) {
    let handler = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        match serde_wasm_bindgen::from_value::<TauriEvent<SnapshotProgress>>(event) {
            Ok(event) => {
                set_state.try_update(|state| state.apply(event.payload));
            }
            Err(err) => leptos::logging::error!("Invalid optimize progress: {err:?}"),
        }
    });
    let unlisten: StoredValue<Option<js_sys::Function>, LocalStorage> =
        StoredValue::new_local(None);
    spawn_local(async move {
        let unlisten_fn = crate::listen(OPTIMIZE_PROGRESS_EVENT, &handler).await;
        // The handler must outlive the listener, it is a small leak per mount.
        handler.forget();
        let unlisten_fn = unlisten_fn.dyn_into::<js_sys::Function>().ok();
        if unlisten.try_set_value(unlisten_fn.clone()).is_some() {
            // The component was unmounted before the listener was registered.
            if let Some(unlisten_fn) = unlisten_fn {
                let _ = unlisten_fn.call0(&JsValue::NULL);
            }
        }
    });
    on_cleanup(move || {
        if let Some(Some(unlisten_fn)) = unlisten.try_get_value() {
            let _ = unlisten_fn.call0(&JsValue::NULL);
        }
    });
}

fn count(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "...".to_string())
}

/// The steps, files and sizes of the snapshot being written by Optimize.
#[component]
pub fn OptimizeProgress() -> impl IntoView {
    let (state, set_state) = signal(ProgressState::default());
    listen_to_progress(set_state);
    view! {
        <Show when=move || !state.with(ProgressState::is_empty)>
            <div class="mt-2">
                <Panel title="Optimize progress">
                    {move || {
                        let state = state.get();
                        let is_done = state.elapsed_ms.is_some();
                        let last_step = state.steps.len();
                        let steps = state
                            .steps
                            .iter()
                            .enumerate()
                            .map(|(idx, (index, total, name))| {
                                let finished = is_done || idx + 1 < last_step;
                                view! {
                                    <li class:text-gray-400=finished class:text-white=!finished>
                                        {if finished { "✓ " } else { "⏳ " }}
                                        {format!("{index}/{total} {name}")}
                                    </li>
                                }
                            })
                            .collect_view();
                        let processed = state
                            .replays_processed
                            .map(|(done, total)| {
                                let percent = done * 100 / total.max(1);
                                view! {
                                    <div class="mt-1 text-xs text-gray-300">
                                        {format!("{done} of {total} replays processed ({percent}%)")}
                                    </div>
                                    <div class="h-2 w-full rounded bg-gray-700">
                                        <div
                                            class="h-2 rounded bg-indigo-500"
                                            style=format!("width: {percent}%")
                                        ></div>
                                    </div>
                                }
                            });
                        let files = state
                            .files
                            .iter()
                            .map(|file| {
                                view! {
                                    <tr>
                                        <td class="pr-4 text-white">{file.name.clone()}</td>
                                        <td class="pr-4 text-right">
                                            {bibytes2(file.bytes as f64)}
                                        </td>
                                        <td class="text-right">{file.records.to_string()}</td>
                                    </tr>
                                }
                            })
                            .collect_view();
                        view! {
                            <div class="grid grid-cols-2 gap-4 text-sm text-gray-300">
                                <div>
                                    <p>
                                        "Files found: "
                                        <span class="text-white">{count(state.files_found)}</span>
                                    </p>
                                    <p>
                                        "Already in the snapshot or copies of another replay: "
                                        <span class="text-white">
                                            {count(state.already_in_snapshot)}
                                        </span>
                                    </p>
                                    <p>
                                        "New replays with valid init data: "
                                        <span class="text-white">
                                            {count(state.valid_replays)}
                                        </span>
                                    </p>
                                    <ul class="mt-1 space-y-0.5">{steps}</ul>
                                    {processed}
                                    {state
                                        .elapsed_ms
                                        .map(|elapsed_ms| {
                                            view! {
                                                <p class="mt-1 text-green-400">
                                                    {format!(
                                                        "Snapshot written in {:.1}s",
                                                        elapsed_ms as f64 / 1000.,
                                                    )}
                                                </p>
                                            }
                                        })}
                                </div>
                                <div>
                                    <table class="text-xs text-gray-400">
                                        <thead>
                                            <tr class="text-left text-gray-300">
                                                <th class="pr-4">"File"</th>
                                                <th class="pr-4 text-right">"Size"</th>
                                                <th class="text-right">"Records"</th>
                                            </tr>
                                        </thead>
                                        <tbody>{files}</tbody>
                                    </table>
                                </div>
                            </div>
                        }
                    }}
                </Panel>
            </div>
        </Show>
    }
}
