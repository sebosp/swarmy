use crate::navigation::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use phosphor_leptos::{
    Icon, IconWeight, ALIEN, BARCODE, BINOCULARS, CLOCK_COUNTER_CLOCKWISE, FADERS, FILM_STRIP,
    GIT_DIFF, HOUSE, INTERSECT_SQUARE, MAP_PIN, MAP_TRIFOLD, QUESTION, USER,
};

/// The pages of the menu, in order, separated in groups.
const MENU_GROUPS: [&[&str]; 4] = [
    &[HOME_PAGE, "Scan"],
    &[
        MAP_LIST_PAGE,
        MAP_DETAILS_PAGE,
        "Replay List",
        REPLAY_DETAIL_PAGE,
        PLAYER_PAGE,
        MAP_SCOUT_PAGE,
    ],
    &[PATCH_OVERVIEW_PAGE, UNIT_OVERVIEW_PAGE],
    &[HELP_PAGE, CHANGELOG_PAGE, "Config"],
];

#[component]
pub fn LeftSideBarMenu(active_page: RwSignal<String>) -> impl IntoView {
    view! {
        <div
            id="swarmy-tauri-left-sidebar"
            class="flex flex-col items-center w-12 h-full overflow-y-auto text-gray-400 bg-gray-900 rounded border-r border-base-100 py-1"
        >
            {MENU_GROUPS
                .iter()
                .enumerate()
                .map(|(idx, pages)| {
                    let separator = if idx == 0 { "" } else { "mt-1 pt-1 border-t border-purple-700" };
                    view! {
                        <div class=format!("flex flex-col items-center gap-0.5 {separator}")>
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
        "flex items-center justify-center w-9 h-9 rounded text-gray-200 bg-gray-700";
    let inactive_icon_class =
        "flex items-center justify-center w-9 h-9 rounded hover:bg-gray-700 hover:text-gray-300";
    let icon_data = match name {
        HOME_PAGE => HOUSE,
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
            title=name
            on:click=move |ev: MouseEvent| {
                ev.prevent_default();
                active_page.set(name.to_string());
            }
        >
            <Icon icon=icon_data weight=IconWeight::Bold size="18px" />
        </a>
    }
}
