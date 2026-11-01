//! A guided tour of swarmy: the configuration, building the snapshot and each view, one step at
//! a time, with a button to open the view of the step.

use crate::navigation::*;
use leptos::prelude::*;
use phosphor_leptos::{Icon, IconWeight, ARROW_LEFT, ARROW_RIGHT, ARROW_SQUARE_OUT};

/// A step of the tour.
struct TourStep {
    title: &'static str,
    /// The view the step is about, it is opened by the button.
    page: &'static str,
    /// What to do or look at in the view.
    points: &'static [&'static str],
}

const STEPS: [TourStep; 14] = [
    TourStep {
        title: "Configure the directories",
        page: "Config",
        points: &[
            "Replay Path: the directory with your replays, searched recursively. On Windows StarCraft II saves them in Documents\\StarCraft II\\Accounts, the default.",
            "Map Caches Path: where the maps and their Map Info are downloaded from Blizzard's depot servers, swarmy-caches in your home directory by default.",
            "Watch the replay path: a banner tells when there are new replays to add, and with \"Show a message\" each new replay is announced, i.e. after a game.",
            "Save, the other views read the snapshot of the Replay Path.",
        ],
    },
    TourStep {
        title: "Build the snapshot",
        page: "Scan",
        points: &[
            "Scan counts the replays of the Replay Path and finds the duplicates, the same game saved twice is counted once once they are moved away.",
            "Optimize reads the replays into the snapshot, the ipcs directory, and downloads the map caches. Run it again after adding replays, only the new ones are read.",
            "Rebuild reads every replay again, i.e. after a new version of swarmy changes the snapshot.",
        ],
    },
    TourStep {
        title: "The maps",
        page: MAP_LIST_PAGE,
        points: &[
            "The maps of the snapshot with their games and dates.",
            "Open in Swarmy-Bevy draws the map in 3D: its levels, ramps, resources and start locations.",
        ],
    },
    TourStep {
        title: "A map in detail",
        page: MAP_DETAILS_PAGE,
        points: &[
            "The win rates of each matchup, the top players and the recent games of the map.",
            "The minimap, the resources and the start locations from the map cache.",
            "Siege Tank spots: where the tanks sieged and how many enemy units they killed there, Show in swarmy-bevy draws all of them as bars on the terrain.",
        ],
    },
    TourStep {
        title: "Scout a map",
        page: MAP_SCOUT_PAGE,
        points: &[
            "Where the units of the first minutes go on a map, i.e. the scouting paths and the proxies, for all the players or one.",
        ],
    },
    TourStep {
        title: "The games",
        page: REPLAY_LIST_PAGE,
        points: &[
            "Every game of the snapshot, filtered by map, player, matchup and dates, sorted by clicking the headers.",
            "Clicking a game ID opens its Replay Detail.",
        ],
    },
    TourStep {
        title: "A game in detail",
        page: REPLAY_DETAIL_PAGE,
        points: &[
            "The players side by side: build orders and expansions compared to the average of the patch, resources lost and killed, macro, focus and economy.",
            "Abilities: the uses of the abilities with energy or a cooldown, i.e. Abduct, Fungal Growth or Inject Larva, against the average of the patch, with the rank of the game and of the player.",
            "The engagements and the chat, the notes and ratings you keep for the game, and Load in swarmy-bevy to see the units born and died on the map.",
        ],
    },
    TourStep {
        title: "A player",
        page: PLAYER_PAGE,
        points: &[
            "The games, wins and matchups of a player over time, its favourite units and openings, compared with the snapshot, and the head to head against another player.",
        ],
    },
    TourStep {
        title: "The patches",
        page: PATCH_OVERVIEW_PAGE,
        points: &[
            "The units that changed in each patch of the balance data, by race.",
            "The abilities commanded in the games of the patch: per game, in how many games and the top players.",
        ],
    },
    TourStep {
        title: "A unit",
        page: UNIT_OVERVIEW_PAGE,
        points: &[
            "A unit in every patch, and how it was used in the snapshot: made, died, kills per unit by month and the games in which it was exceptional.",
        ],
    },
    TourStep {
        title: "How the patches evolve",
        page: PATCH_EVOLUTION_PAGE,
        points: &[
            "Lines of the costs, times and requirements of units and what they make over the patches, the queries can be saved.",
            "Ability use per game: the abilities of a race over the patches of the games.",
        ],
    },
    TourStep {
        title: "A race",
        page: RACE_DETAIL_PAGE,
        points: &[
            "The matchups and patches of a race, the maps it wins the most on, its units and abilities, and its players with the most games.",
        ],
    },
    TourStep {
        title: "Your progress",
        page: PROGRESS_PAGE,
        points: &[
            "With your name in Config, your workers at 06:00, income at 08:00, seconds supply blocked and army at 10:00 over your last games, against humans or computers.",
            "Set a goal for each number, the charts draw it next to your games and the average of the last 10.",
        ],
    },
    TourStep {
        title: "Help and what is new",
        page: HELP_PAGE,
        points: &[
            "Help explains every view and how its statistics are calculated, the Changelog has what changed in each version.",
            "Names with a dotted underline open their own view from anywhere: maps, games, players, units, races and patches.",
        ],
    },
];

#[component]
pub fn TourView() -> impl IntoView {
    let navigation = use_navigation();
    let step = RwSignal::new(0usize);
    let last = STEPS.len() - 1;
    view! {
        <div class="max-w-3xl p-4 space-y-4">
            <h1 class="text-2xl font-semibold text-white">"Tour of swarmy"</h1>
            <p class="text-sm text-gray-400">
                "The steps to set swarmy up and what each view is for. Open the view of a step, then come back to the Tour from the sidebar to continue."
            </p>
            <ol class="flex flex-wrap gap-1">
                {STEPS
                    .iter()
                    .enumerate()
                    .map(|(idx, tour_step)| view! {
                        <li>
                            <button
                                class=move || if step.get() == idx { "btn btn-xs btn-primary" } else { "btn btn-xs btn-ghost" }
                                title=tour_step.title
                                on:click=move |_| step.set(idx)
                            >
                                {idx + 1}
                            </button>
                        </li>
                    })
                    .collect_view()}
            </ol>
            {move || {
                let current = &STEPS[step.get()];
                let page = current.page;
                view! {
                    <div class="rounded-md border border-white/15 bg-gray-800/40 p-4 space-y-3">
                        <h2 class="text-lg font-semibold text-white">
                            {format!("{}. {}", step.get() + 1, current.title)}
                        </h2>
                        <ul class="list-disc list-inside text-sm text-gray-300 space-y-1">
                            {current.points.iter().map(|point| view! { <li>{*point}</li> }).collect_view()}
                        </ul>
                        <button
                            class="btn btn-accent btn-sm"
                            on:click=move |_| navigation.active_page.set(page.to_string())
                        >
                            <Icon icon=ARROW_SQUARE_OUT weight=IconWeight::Light />
                            {format!("Open {page}")}
                        </button>
                    </div>
                }
            }}
            <div class="flex gap-2">
                <button
                    class="btn btn-sm btn-ghost"
                    prop:disabled=move || step.get() == 0
                    on:click=move |_| step.update(|step| *step = step.saturating_sub(1))
                >
                    <Icon icon=ARROW_LEFT weight=IconWeight::Light />
                    "Previous"
                </button>
                <button
                    class="btn btn-sm btn-primary"
                    prop:disabled=move || step.get() == last
                    on:click=move |_| step.update(|step| *step = (*step + 1).min(last))
                >
                    "Next"
                    <Icon icon=ARROW_RIGHT weight=IconWeight::Light />
                </button>
            </div>
        </div>
    }
}
