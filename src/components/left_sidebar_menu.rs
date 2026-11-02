use crate::navigation::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use phosphor_leptos::{
    Icon, IconWeight, ALIEN, BARCODE, BINOCULARS, CHART_LINE, CLOCK_COUNTER_CLOCKWISE, FADERS,
    FILM_STRIP, GIT_DIFF, HOUSE, INTERSECT_SQUARE, MAP_PIN, MAP_TRIFOLD, QUESTION, SHIELD_STAR,
    SIGNPOST, TREND_UP, USER,
};

/// The pages of the menu, in order, separated in groups. The last group, the pages read once
/// (the tour, the help, the changelog), is at the bottom of the menu, dimmed.
const MENU_GROUPS: [&[&str]; 5] = [
    &[HOME_PAGE, "Scan", "Config"],
    &[
        MAP_LIST_PAGE,
        MAP_DETAILS_PAGE,
        "Replay List",
        REPLAY_DETAIL_PAGE,
        PLAYER_PAGE,
        PROGRESS_PAGE,
    ],
    &[
        PATCH_OVERVIEW_PAGE,
        UNIT_OVERVIEW_PAGE,
        PATCH_EVOLUTION_PAGE,
        RACE_DETAIL_PAGE,
    ],
    // Experimental: may change a lot, or go away if they do not prove useful.
    &[MAP_SCOUT_PAGE],
    &[TOUR_PAGE, HELP_PAGE, CHANGELOG_PAGE],
];

/// The index of the experimental group in [`MENU_GROUPS`].
const EXPERIMENTAL_GROUP: usize = 3;

#[component]
pub fn LeftSideBarMenu(active_page: RwSignal<String>) -> impl IntoView {
    view! {
        <div
            id="swarmy-tauri-left-sidebar"
            class="flex flex-col items-center w-12 h-full overflow-y-auto text-muted bg-base-100 rounded border-r border-base-100 py-1"
        >
            {MENU_GROUPS
                .iter()
                .enumerate()
                .map(|(idx, pages)| {
                    let experimental = idx == EXPERIMENTAL_GROUP;
                    let separator = if idx == 0 {
                        ""
                    } else if idx == MENU_GROUPS.len() - 1 {
                        "mt-auto pt-1 border-t border-base-content/15 opacity-70"
                    } else if experimental {
                        "mt-1 pt-0.5 border-t border-dashed border-warning/60"
                    } else {
                        "mt-1 pt-1 border-t border-secondary"
                    };
                    view! {
                        <div
                            class=format!("flex flex-col items-center gap-0.5 {separator}")
                            title=experimental.then_some("Experimental: these views may change a lot, or go away if they do not prove useful")
                        >
                            {experimental.then(|| view! {
                                <span class="text-[9px] leading-none text-warning/80">"exp."</span>
                            })}
                            {pages
                                .iter()
                                .map(|name| view! { <SidebarMenuItem name active_page /> })
                                .collect_view()}
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn SidebarMenuItem(name: &'static str, active_page: RwSignal<String>) -> impl IntoView {
    let active_icon_class =
        "flex items-center justify-center w-9 h-9 rounded text-base-content bg-base-300";
    let inactive_icon_class =
        "flex items-center justify-center w-9 h-9 rounded hover:bg-base-300 hover:text-muted";
    let icon_data = match name {
        HOME_PAGE => HOUSE,
        TOUR_PAGE => SIGNPOST,
        "Scan" => BARCODE,
        "Config" => FADERS,
        MAP_LIST_PAGE => MAP_TRIFOLD,
        MAP_DETAILS_PAGE => MAP_PIN,
        "Replay List" => INTERSECT_SQUARE,
        REPLAY_DETAIL_PAGE => FILM_STRIP,
        PLAYER_PAGE => USER,
        MAP_SCOUT_PAGE => BINOCULARS,
        PATCH_OVERVIEW_PAGE => GIT_DIFF,
        UNIT_OVERVIEW_PAGE => ALIEN,
        PATCH_EVOLUTION_PAGE => CHART_LINE,
        RACE_DETAIL_PAGE => SHIELD_STAR,
        PROGRESS_PAGE => TREND_UP,
        HELP_PAGE => QUESTION,
        CHANGELOG_PAGE => CLOCK_COUNTER_CLOCKWISE,
        _ => HOUSE,
    };

    view! {
        <a
            id=format!("sidebar-menu-item-{}", name.replace(" ", "-").to_lowercase())
            href="#"
            class=move || {
                if active_page.get() == name { active_icon_class } else { inactive_icon_class }
            }
            title=if name == MAP_SCOUT_PAGE { "Map Scout (experimental)" } else { name }
            on:click=move |ev: MouseEvent| {
                ev.prevent_default();
                active_page.set(name.to_string());
            }
        >
            <Icon icon=icon_data weight=IconWeight::Bold size="18px" />
        </a>
    }
}
