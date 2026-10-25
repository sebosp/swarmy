//! A dropdown of the players or maps of the snapshot, by name with their number of games.

use crate::invoke_json;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::GamesWins;

/// The players are identified by their name without the clan tag.
fn without_clan(name: &str) -> &str {
    name.rsplit("<sp/>").next().unwrap_or(name)
}

/// Lists the names returned by `command`, the selected one is `value`.
#[component]
pub fn NameSelect(
    label: &'static str,
    /// The backend command listing the names, i.e. `list_players`.
    command: &'static str,
    #[prop(into)] value: Signal<String>,
    on_select: Callback<String>,
    /// The option for no selection, i.e. no player to compare with.
    #[prop(optional)]
    empty_label: Option<&'static str>,
) -> impl IntoView {
    let names = RwSignal::new(Vec::<GamesWins>::new());
    spawn_local(async move {
        match invoke_json::<Vec<GamesWins>>(command, &()).await {
            Ok(loaded) => names.set(loaded),
            Err(err) => leptos::logging::error!("Error listing {label}: {err}"),
        }
    });
    view! {
        <div class="w-64">
            <label class="block text-sm/6 font-medium text-white">{label}</label>
            <select
                class="select select-sm w-full bg-gray-800 text-gray-200 border border-white/20"
                on:change=move |ev| on_select.run(event_target_value(&ev))
            >
                <option value="" selected=move || value.get().is_empty()>
                    {empty_label.unwrap_or("Choose...")}
                </option>
                {move || {
                    names
                        .get()
                        .into_iter()
                        .map(|name| {
                            let option = name.label.clone();
                            view! {
                                <option
                                    value=name.label.clone()
                                    selected=move || without_clan(&value.get()) == option
                                >
                                    {format!("{} ({} games)", name.label, name.games)}
                                </option>
                            }
                        })
                        .collect_view()
                }}
            </select>
        </div>
    }
}
