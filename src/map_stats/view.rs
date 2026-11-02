//! Leptos view for map stats.

use super::actions::*;
use super::*;
use crate::components::abilities::group_digits;
use crate::components::links::MapLink;
use crate::components::loading::Loading;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use phosphor_leptos::{Icon, IconWeight, CUBE};
use reactive_stores::Store;
use swarmy_common::*;

#[component]
pub fn StatsByMap() -> impl IntoView {
    let player_name = RwSignal::new(String::new());
    let map_title = RwSignal::new(String::new());
    let query = move || MapStatsQuery {
        map_title: map_title.get(),
        player_name: player_name.get(),
    };
    let (backend_response, set_backend_response) = signal(ApiResponse {
        meta: ResponseMeta::incomplete(),
        message: String::new(),
    });
    let map_stats_store = Store::new(MapStatsDataTable::default());
    trigger_fetch_query_map_stats(map_stats_store, query(), set_backend_response);
    view! {
        // The filters on one line, each label beside its input.
        <div class="flex flex-wrap items-center gap-x-6 gap-y-2">
            <label class="flex items-center gap-2 text-sm text-base-content">
                "Map"
                <input
                    name="map_title"
                    class="input input-sm w-64"
                    bind:value=map_title
                    on:input=move |_| {
                        trigger_fetch_query_map_stats(map_stats_store, query(), set_backend_response);
                    }
                    type="text"
                />
            </label>
            <label class="flex items-center gap-2 text-sm text-base-content">
                "Player"
                <input
                    name="player_name"
                    class="input input-sm w-64"
                    bind:value=player_name
                    on:input=move |_| {
                        trigger_fetch_query_map_stats(map_stats_store, query(), set_backend_response);
                    }
                    type="text"
                />
            </label>
        </div>
        <DisplayBackendStatus backend_response />
        <div class="col-span-10">
            <Show when=move || !backend_response.get().meta.is_complete>
                <Loading message="Collecting the maps of the snapshot..." />
            </Show>
            // The counts are the figures of the view, in the primary color; the sentence is quiet,
            // smaller than the rows of maps that are the data.
            <Show when=move || backend_response.get().meta.is_complete>
                <p class="my-2 text-sm text-muted">
                    <span class="font-semibold text-primary tabular-nums">
                        {move || group_digits(map_stats_store.total().get() as u64)}
                    </span>
                    {move || if map_stats_store.total().get() == 1 { " map, " } else { " maps, " }}
                    <span class="font-semibold text-primary tabular-nums">
                        {move || {
                            group_digits(
                                map_stats_store
                                    .data()
                                    .read()
                                    .iter()
                                    .map(|map| map.num_games as u64)
                                    .sum(),
                            )
                        }}
                    </span>
                    {move || {
                        if map_title.get().is_empty() && player_name.get().is_empty() {
                            " games in the snapshot"
                        } else {
                            " games matching the search"
                        }
                    }}
                </p>
            </Show>
            <Show when=move || { map_stats_store.total().get() > 0 }>
                <MapStatsDataTable map_stats_store />
            </Show>
        </div>
    }
}

#[component]
pub fn MapStatsDataTable(map_stats_store: Store<MapStatsDataTable>) -> impl IntoView {
    view! {
        <div class="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
            <div class="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
                <table class="relative min-w-full divide-y divide-base-content/15">
                    <thead>
                        <tr>
                            <th
                                scope="col"
                                class="py-3.5 pr-3 pl-4 text-left text-xs font-semibold whitespace-nowrap xs:pl-0 text-base-content"
                            >
                                "Map Title"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 num text-xs font-semibold text-base-content"
                            >
                                "Games"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-xs font-semibold whitespace-nowrap text-base-content"
                            >
                                "Min Date"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-xs font-semibold whitespace-nowrap text-base-content"
                            >
                                "Max Date"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-xs font-semibold whitespace-nowrap text-base-content"
                                data-no-sort
                            >
                                "Actions"
                            </th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-base-content/10 bg-base-100">
                        <For
                            each=move || map_stats_store.data()
                            key=|row| (row.read().title.clone(), row.read().map_info_sha256.clone())
                            children=|child| {
                                let cache_ids: String = child.read().cache_handles.clone();
                                view! {
                                    <tr>
                                        <td class="py-2 pr-3 text-xs text-base-content">
                                            <MapLink map_title=child.read().title.clone() />
                                        </td>
                                        <td class="px-2 py-2 text-xs num text-muted">
                                            {child.read().num_games}
                                        </td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-muted">
                                            {format!("{}", child.read().min_date)}
                                        </td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-muted">
                                            {format!("{}", child.read().max_date)}
                                        </td>
                                        <td class="px-2 py-2">
                                            <button
                                                class="btn btn-primary btn-xs b-0"
                                                on:click=move |ev: MouseEvent| {
                                                    ev.prevent_default();
                                                    trigger_swarmy_bevy_exec_on_caches(&cache_ids)
                                                }
                                                title="Open in Swarmy-Bevy"
                                            >
                                                <Icon icon=CUBE weight=IconWeight::Light prop:class="stroke-current" />
                                            </button>
                                        </td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
        </div>
    }
}
