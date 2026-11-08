//! A field to pick one of the players or maps of the snapshot, by name, with their number of games.

use crate::invoke_json;
use leptos::ev::KeyboardEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::GamesWins;

/// The players are identified by their name without the clan tag.
fn without_clan(name: &str) -> &str {
    name.rsplit("<sp/>").next().unwrap_or(name)
}

/// A combobox of the names returned by `command` (most games first), the selected one is `value`.
/// Typing filters the names that contain the text, at most the rows of a table page (Config): a
/// snapshot has thousands of players, a list of all of them blocks the window. The arrows move in
/// the list, Enter or a click picks, Escape closes it. Not a `<datalist>`: the webview of the app on
/// Linux does not show its suggestions.
#[component]
pub fn NameSelect(
    label: &'static str,
    /// The backend command listing the names, i.e. `list_players`.
    command: &'static str,
    #[prop(into)] value: Signal<String>,
    on_select: Callback<String>,
    /// What no selection means, i.e. no player to compare with.
    #[prop(optional)]
    empty_label: Option<&'static str>,
) -> impl IntoView {
    let names = RwSignal::new(Vec::<GamesWins>::new());
    let query = RwSignal::new(String::new());
    let open = RwSignal::new(false);
    let highlighted = RwSignal::new(0usize);
    spawn_local(async move {
        match invoke_json::<Vec<GamesWins>>(command, &()).await {
            Ok(loaded) => names.set(loaded),
            Err(err) => leptos::logging::error!("Error listing {label}: {err}"),
        }
    });
    // The field shows the selection until something is typed.
    Effect::new(move |_| query.set(without_clan(&value.get()).to_string()));
    let id = format!("names-{}", label.to_lowercase().replace(' ', "-"));
    let list_id = format!("{id}-list");
    let suggestions = Memo::new(move |_| {
        let typed = query.get().to_lowercase();
        let selected = without_clan(&value.get()).to_lowercase();
        // The selection shown in the field lists every name, typing narrows them.
        let filter = if typed == selected {
            String::new()
        } else {
            typed
        };
        let limit = crate::components::paged::page_rows();
        names.with(|names| {
            names
                .iter()
                .filter(|name| filter.is_empty() || name.label.to_lowercase().contains(&filter))
                .take(limit)
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let pick = move |name: String| {
        open.set(false);
        query.set(name.clone());
        on_select.run(name);
    };
    let on_key = move |ev: KeyboardEvent| {
        let count = suggestions.with_untracked(Vec::len);
        match ev.key().as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                open.set(true);
                highlighted.update(|idx| *idx = (*idx + 1).min(count.saturating_sub(1)));
            }
            "ArrowUp" => {
                ev.prevent_default();
                highlighted.update(|idx| *idx = idx.saturating_sub(1));
            }
            "Enter" => {
                ev.prevent_default();
                let typed = query.get_untracked();
                if typed.trim().is_empty() && empty_label.is_some() {
                    open.set(false);
                    on_select.run(String::new());
                } else if let Some(name) = suggestions
                    .with_untracked(|names| names.get(highlighted.get_untracked()).cloned())
                {
                    pick(name.label);
                }
            }
            "Escape" => {
                open.set(false);
                query.set(without_clan(&value.get_untracked()).to_string());
            }
            _ => {}
        }
    };
    view! {
        // The label beside its field, on one line.
        <div class="flex items-center gap-2 text-sm font-medium text-base-content">
            <label for=id.clone()>{label}</label>
            <div class="relative">
                <input
                    id=id.clone()
                    type="text"
                    role="combobox"
                    autocomplete="off"
                    aria-autocomplete="list"
                    aria-controls=list_id.clone()
                    aria-expanded=move || open.get().to_string()
                    class="input input-sm w-72 bg-base-200 text-base-content border border-base-content/20 font-normal"
                    placeholder=empty_label.unwrap_or("By name")
                    title="Type a name, then pick it in the list"
                    prop:value=move || query.get()
                    on:input=move |ev| {
                        query.set(event_target_value(&ev));
                        highlighted.set(0);
                        open.set(true);
                    }
                    // The name shown is selected: typing replaces it instead of adding to it.
                    on:focus=move |ev| {
                        open.set(true);
                        if let Some(input) = ev
                            .target()
                            .and_then(|target| {
                                wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlInputElement>(target).ok()
                            })
                        {
                            input.select();
                        }
                    }
                    // A click in the list happens before the field loses the focus.
                    on:blur=move |_| {
                        set_timeout(
                            move || {
                                open.set(false);
                                query.set(without_clan(&value.get_untracked()).to_string());
                            },
                            std::time::Duration::from_millis(150),
                        )
                    }
                    on:keydown=on_key
                />
                <Show when=move || open.get() && suggestions.with(Vec::is_empty) && !names.with(Vec::is_empty)>
                    <p class="absolute z-20 mt-1 w-full rounded border border-base-content/20 bg-base-100 shadow-lg px-2 py-1 text-xs font-normal text-muted">
                        {move || format!("No {} contains \"{}\"", label.to_lowercase(), query.get())}
                    </p>
                </Show>
                <Show when=move || open.get() && !suggestions.with(Vec::is_empty)>
                    <ul
                        id=list_id.clone()
                        role="listbox"
                        class="absolute z-20 mt-1 w-full max-h-72 overflow-y-auto rounded border border-base-content/20 bg-base-100 shadow-lg py-1"
                    >
                        {move || {
                            suggestions
                                .get()
                                .into_iter()
                                .enumerate()
                                .map(|(idx, name)| {
                                    let label = name.label.clone();
                                    view! {
                                        <li
                                            role="option"
                                            aria-selected=move || (highlighted.get() == idx).to_string()
                                            class="flex justify-between gap-2 px-2 py-1 cursor-pointer font-normal"
                                            class:bg-base-300=move || highlighted.get() == idx
                                            on:mouseenter=move |_| highlighted.set(idx)
                                            // Before the blur of the field.
                                            on:mousedown=move |ev| {
                                                ev.prevent_default();
                                                pick(label.clone());
                                            }
                                        >
                                            <span class="truncate text-base-content">{name.label.clone()}</span>
                                            <span class="text-xs text-muted tabular-nums whitespace-nowrap">
                                                {format!("{} games", name.games)}
                                            </span>
                                        </li>
                                    }
                                })
                                .collect_view()
                        }}
                    </ul>
                </Show>
            </div>
        </div>
    }
}
