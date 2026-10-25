//! Links to the other views, usable from any view. They share the `nav-link` style, a dotted
//! underline, so that it is clear that they can be clicked.

use crate::components::unit_icon::UnitIcon;
use crate::navigation::use_navigation;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use swarmy_common::clean_player_name_clan_escapes;

/// A green up triangle for a win, a red down triangle for a loss, like stock prices.
#[component]
pub fn ResultMarker(result: String) -> impl IntoView {
    match result.as_str() {
        "Win" => view! { <span class="text-green-500" title="Win">"▲ "</span> }.into_any(),
        "Loss" => view! { <span class="text-red-500" title="Loss">"▼ "</span> }.into_any(),
        _ => view! { <span class="text-gray-500" title=result>"■ "</span> }.into_any(),
    }
}

/// The player name, clicking it opens the Player view.
/// `player_name` is the name as stored in the replay, with the clan tag escaped.
#[component]
pub fn PlayerLink(player_name: String) -> impl IntoView {
    let navigation = use_navigation();
    let display_name = clean_player_name_clan_escapes(&player_name);
    view! {
        <a
            href="#"
            class="nav-link"
            title="Open the player"
            on:click=move |ev: MouseEvent| {
                ev.prevent_default();
                navigation.open_player(&player_name);
            }
        >
            {display_name}
        </a>
    }
}

/// The snapshot id of the replay, clicking it opens the Replay Detail view.
#[component]
pub fn ReplayLink(ext_fs_id: u64) -> impl IntoView {
    let navigation = use_navigation();
    view! {
        <a
            href="#"
            class="nav-link text-green-400"
            title="Open the replay details"
            on:click=move |ev: MouseEvent| {
                ev.prevent_default();
                navigation.open_replay(ext_fs_id);
            }
        >
            <code>{ext_fs_id}</code>
        </a>
    }
}

/// The map title, clicking it opens the Map Details view.
#[component]
pub fn MapLink(map_title: String) -> impl IntoView {
    let navigation = use_navigation();
    view! {
        <a
            href="#"
            class="nav-link"
            title="Open the map details"
            on:click={
                let map_title = map_title.clone();
                move |ev: MouseEvent| {
                    ev.prevent_default();
                    navigation.open_map(&map_title);
                }
            }
        >
            {map_title.clone()}
        </a>
    }
}

/// The unit name, clicking it opens the Unit Overview view with its stats in every patch.
#[component]
pub fn UnitLink(unit_name: String) -> impl IntoView {
    let navigation = use_navigation();
    let target = unit_name.clone();
    view! {
        <span class="whitespace-nowrap">
            <UnitIcon unit_name=unit_name.clone() />
            <a
                href="#"
                class="nav-link"
                title="Open the unit in every patch"
                on:click=move |ev: MouseEvent| {
                    ev.prevent_default();
                    navigation.open_unit(&target);
                }
            >
                {unit_name}
            </a>
        </span>
    }
}

/// The patch version of a replay, clicking it opens the Patch Overview view at that patch.
#[component]
pub fn PatchLink(version: u32) -> impl IntoView {
    let navigation = use_navigation();
    view! {
        <a
            href="#"
            class="nav-link"
            title="Open the unit changes of the patch"
            on:click=move |ev: MouseEvent| {
                ev.prevent_default();
                navigation.open_patch(version);
            }
        >
            {version}
        </a>
    }
}

/// The race, clicking it opens the Patch Overview view with the changes of the race.
#[component]
pub fn RaceLink(
    race: String,
    /// Shown instead of the race, i.e. its initial.
    #[prop(optional)]
    label: Option<String>,
) -> impl IntoView {
    let navigation = use_navigation();
    let target = race.clone();
    let label = label.unwrap_or_else(|| race.clone());
    view! {
        <a
            href="#"
            class="nav-link"
            title="Open the patch changes of the race"
            on:click=move |ev: MouseEvent| {
                ev.prevent_default();
                navigation.open_race(&target);
            }
        >
            {label}
        </a>
    }
}

/// Unit counts formatted as `2xMarine,1xMarauder`, each unit name is a [`UnitLink`].
#[component]
pub fn UnitCountLinks(counts: String) -> impl IntoView {
    counts
        .split(',')
        .filter(|entry| !entry.is_empty())
        .enumerate()
        .map(|(idx, entry)| {
            let (count, name) = entry.split_once('x').unwrap_or(("", entry));
            view! {
                {(idx > 0).then_some(", ")}
                {(!count.is_empty()).then(|| format!("{count} "))}
                <UnitLink unit_name=name.to_string() />
            }
        })
        .collect_view()
}
