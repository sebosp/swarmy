//! The notes, rating and tags of the user about a replay, kept in the personal database by the
//! sha256 of the replay so that they survive generating the snapshot again.

use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

#[derive(serde::Serialize)]
struct NotesArgs {
    notes: ReplayNotes,
}

#[derive(serde::Serialize)]
struct Sha256Args {
    sha256: String,
}

/// The stars of the rating, clicking the current one removes the rating.
#[component]
pub fn RatingStars(rating: RwSignal<Option<u8>>) -> impl IntoView {
    (1..=5u8)
        .map(|stars| {
            view! {
                <button
                    class=move || {
                        if rating.get().is_some_and(|rating| rating >= stars) {
                            "text-yellow-400 text-xl leading-none"
                        } else {
                            "text-gray-600 text-xl leading-none hover:text-yellow-200"
                        }
                    }
                    title=format!("{stars} stars, click again to remove the rating")
                    on:click=move |_| {
                        rating.update(|rating| {
                            *rating = if *rating == Some(stars) { None } else { Some(stars) };
                        })
                    }
                >
                    "★"
                </button>
            }
        })
        .collect_view()
}

/// The stars of a rating, read only.
pub fn rating_text(rating: Option<u8>) -> String {
    rating
        .map(|rating| "★".repeat(rating as usize))
        .unwrap_or_default()
}

#[component]
pub fn ReplayNotesPanel(sha256: String) -> impl IntoView {
    let rating = RwSignal::new(None::<u8>);
    let note = RwSignal::new(String::new());
    let tags = RwSignal::new(String::new());
    let known_tags = RwSignal::new(Vec::<GamesWins>::new());
    let status = RwSignal::new(String::new());
    {
        let sha256 = sha256.clone();
        spawn_local(async move {
            match invoke_json::<ReplayNotes>("get_replay_notes", &Sha256Args { sha256 }).await {
                Ok(notes) => {
                    rating.set(notes.rating);
                    note.set(notes.note);
                    tags.set(notes.tags.join(", "));
                }
                Err(err) => status.set(err),
            }
            if let Ok(loaded) = invoke_json::<Vec<GamesWins>>("list_replay_tags", &()).await {
                known_tags.set(loaded);
            }
        });
    }
    let save = move |_| {
        let notes = ReplayNotes {
            sha256: sha256.clone(),
            rating: rating.get(),
            note: note.get(),
            tags: tags
                .get()
                .split(',')
                .map(|tag| tag.trim().to_string())
                .filter(|tag| !tag.is_empty())
                .collect(),
        };
        spawn_local(async move {
            match invoke_json::<ReplayNotes>("set_replay_notes", &NotesArgs { notes }).await {
                Ok(saved) => {
                    tags.set(saved.tags.join(", "));
                    status.set("Saved".to_string());
                }
                Err(err) => status.set(err),
            }
        });
    };
    view! {
        <div class="rounded-md border border-white/15 bg-gray-800/40 p-2 space-y-2 text-sm">
            <div class="flex items-center gap-3">
                <h2 class="text-base/7 font-semibold text-white">"Notes"</h2>
                <RatingStars rating />
                <span class="text-xs text-gray-500">{move || status.get()}</span>
            </div>
            <textarea
                class="textarea textarea-sm w-full bg-gray-900 text-gray-200 border border-white/20"
                rows="3"
                placeholder="Notes about the replay, i.e. what to show when casting it"
                prop:value=move || note.get()
                on:input=move |ev| note.set(event_target_value(&ev))
            ></textarea>
            <div class="flex items-center gap-2">
                <input
                    class="input input-sm grow bg-gray-900 text-gray-200 border border-white/20"
                    list="replay-tags"
                    placeholder="Tags separated by commas, i.e. finals, comeback"
                    prop:value=move || tags.get()
                    on:input=move |ev| tags.set(event_target_value(&ev))
                />
                <datalist id="replay-tags">
                    {move || {
                        known_tags
                            .get()
                            .into_iter()
                            .map(|tag| view! { <option value=tag.label.clone()>{format!("{} ({})", tag.label, tag.games)}</option> })
                            .collect_view()
                    }}
                </datalist>
                <button class="btn btn-sm btn-primary" on:click=save title="Save the notes, rating and tags of the replay">
                    "Save"
                </button>
            </div>
        </div>
    }
}
