//! The replays that are copies of another replay, they can be moved to the duplicate replays folder
//! of the settings, outside of the replay path. Nothing is deleted.

use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, COPY};
use swarmy_common::*;

/// The duplicates listed, the others are counted.
const MAX_LISTED: usize = 200;

#[component]
pub fn DuplicatesPanel(app_settings: ReadSignal<AppSettings>) -> impl IntoView {
    let duplicates = RwSignal::new(None::<Vec<DuplicateReplay>>);
    let status = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let find = move || {
        busy.set(true);
        status.set("Looking for the copies of the replays...".to_string());
        spawn_local(async move {
            match invoke_json::<Vec<DuplicateReplay>>("find_duplicate_replays", &()).await {
                Ok(found) => {
                    status.set(format!("{} duplicate replays", found.len()));
                    duplicates.set(Some(found));
                }
                Err(err) => status.set(err),
            }
            busy.set(false);
        });
    };
    let move_to_duplicates = move |_| {
        busy.set(true);
        spawn_local(async move {
            match invoke_json::<usize>("move_duplicate_replays", &()).await {
                Ok(moved) => {
                    status.set(format!(
                        "Moved {moved} duplicate replays to {}",
                        app_settings.get_untracked().duplicates_path
                    ));
                    duplicates.set(Some(vec![]));
                }
                Err(err) => status.set(err),
            }
            busy.set(false);
        });
    };
    view! {
        <div class="mt-4 rounded-md border border-white/15 bg-gray-800/40 p-2 space-y-2">
            <div class="flex items-center gap-3">
                <h3 class="text-base font-semibold text-white">"Duplicates"</h3>
                <button
                    class="btn btn-sm btn-primary"
                    disabled=move || busy.get()
                    title="The replays with the same contents as a replay of the snapshot or another replay"
                    on:click=move |_| find()
                >
                    <Icon icon=COPY weight=IconWeight::Light />
                    "Find duplicates"
                </button>
                <Show when=move || duplicates.get().is_some_and(|found| !found.is_empty())>
                    <button
                        class="btn btn-sm btn-accent"
                        disabled=move || busy.get() || app_settings.get().duplicates_path.is_empty()
                        title=move || {
                            let folder = app_settings.get().duplicates_path;
                            if folder.is_empty() {
                                "Set the Duplicate Replays Folder in Config first".to_string()
                            } else {
                                format!("Moves the duplicates to {folder}, with the directories they were found in, nothing is deleted")
                            }
                        }
                        on:click=move_to_duplicates
                    >
                        "Move to the duplicates folder"
                    </button>
                </Show>
                <span class="text-sm text-gray-400">{move || status.get()}</span>
            </div>
            <Show when=move || app_settings.get().duplicates_path.is_empty()>
                <p class="text-xs text-gray-500">
                    "Set the Duplicate Replays Folder in Config to move the duplicates out of the replay path."
                </p>
            </Show>
            {move || duplicates.get().filter(|found| !found.is_empty()).map(|found| {
                let more = found.len().saturating_sub(MAX_LISTED);
                view! {
                    <table class="min-w-full text-xs text-gray-400">
                        <thead>
                            <tr class="text-left text-gray-300">
                                <th class="pr-2">"Duplicate"</th>
                                <th>"Same contents as"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {found
                                .into_iter()
                                .take(MAX_LISTED)
                                .map(|duplicate| view! {
                                    <tr>
                                        <td class="pr-2 break-all">{duplicate.file}</td>
                                        <td class="break-all">{duplicate.original}</td>
                                    </tr>
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                    {(more > 0).then(|| view! { <p class="text-xs">{format!("And {more} more")}</p> })}
                }
            })}
        </div>
    }
}
