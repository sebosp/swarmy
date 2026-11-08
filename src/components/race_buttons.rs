//! The race buttons of the views that pick a race first, then a unit or an ability of it: as in
//! Race Detail, the race chosen is the primary button.

use leptos::prelude::*;

/// The races of StarCraft II, the units without a race are Neutral.
pub const PLAYABLE_RACES: [&str; 3] = ["Terran", "Protoss", "Zerg"];

/// A button per race, `value` is the race chosen; with `all_label` a first button for no race
/// (the empty string).
#[component]
pub fn RaceButtons(
    value: RwSignal<String>,
    #[prop(default = &PLAYABLE_RACES)] races: &'static [&'static str],
    #[prop(optional)] all_label: Option<&'static str>,
    /// Called with the race clicked, after `value` is set.
    #[prop(optional)]
    on_pick: Option<Callback<String>>,
) -> impl IntoView {
    let button = move |race: &'static str, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || if value.get() == race { "btn btn-sm btn-primary" } else { "btn btn-sm btn-ghost" }
                aria-pressed=move || (value.get() == race).to_string()
                on:click=move |_| {
                    value.set(race.to_string());
                    if let Some(on_pick) = on_pick {
                        on_pick.run(race.to_string());
                    }
                }
            >
                {label}
            </button>
        }
    };
    view! {
        <div class="flex flex-wrap items-center gap-1" role="group" aria-label="Race">
            {all_label.map(|label| button("", label))}
            {races.iter().map(|race| button(race, race)).collect_view()}
        </div>
    }
}
