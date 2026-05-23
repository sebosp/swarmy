//! Leptos view for map stats.

use super::actions::*;
use super::*;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use reactive_stores::Store;
use swarmy_tauri_common::*;

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
        <div class="grid grid-cols-10 gap-1">
            <div class="col-span-4">
                <label for="map_title" class="block text-sm/6 font-medium text-white">
                    Map
                </label>
                <div>
                    <input
                        name="map_title"
                        class=text_input_tailwind_classes().join(" ")
                        bind:value=map_title
                        on:input=move |_| {
                            trigger_fetch_query_map_stats(
                                map_stats_store,
                                query(),
                                set_backend_response,
                            );
                        }
                        type="text"
                    />
                </div>
            </div>
            <div class="col-span-1"></div>
            <div class="col-span-4">
                <label for="player_name" class="block text-sm/6 font-medium text-white">
                    Player
                </label>
                <div>
                    <input
                        name="player_name"
                        class=text_input_tailwind_classes().join(" ")
                        bind:value=player_name
                        on:input=move |_| {
                            trigger_fetch_query_map_stats(
                                map_stats_store,
                                query(),
                                set_backend_response,
                            );
                        }
                        type="text"
                    />
                </div>
            </div>
            <div class="col-span-1"></div>
        </div>
        <DisplayBackendStatus backend_response />
        <div class="col-span-10">
            <h2 class="text-base/7 font-semibold text-white">
                {move || map_stats_store.total().get()} " Unique maps found in snapshot."
            </h2>
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
                <table class="relative min-w-full divide-y divide-white/15">
                    <thead>
                        <tr>
                            <th
                                scope="col"
                                class="py-3.5 pr-3 pl-4 text-left text-xs font-semibold whitespace-nowrap xs:pl-0 text-white"
                            >
                                "Map Title"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-xs font-semibold whitespace-nowrap text-white"
                            >
                                "Games"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-xs font-semibold whitespace-nowrap text-white"
                            >
                                "Cache Handles"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-xs font-semibold whitespace-nowrap text-white"
                            >
                                "Min Date"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-xs font-semibold whitespace-nowrap text-white"
                            >
                                "Max Date"
                            </th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-white/10 bg-gray-900">
                        <For
                            each=move || map_stats_store.data()
                            key=|row| {
                                format!("{}:{}", row.read().title.clone(), row.read().cache_handles)
                            }
                            children=|child| {
                                let cache_ids: String = child.read().cache_handles.clone();
                                let map_title: String = child.read().title.clone();
                                let handles_count = child
                                    .read()
                                    .cache_handles
                                    .split(",")
                                    .collect::<Vec<&str>>()
                                    .len();
                                view! {
                                    <tr>
                                        <td>
                                            <button
                                                class="btn btn-primary btn-sm b-0 p-0 m-0"
                                                on:click=move |ev: MouseEvent| {
                                                    ev.prevent_default();
                                                    trigger_swarmy_bevy_exec_on_caches(&map_title, &cache_ids)
                                                }
                                                title="Open in Swarmy-Bevy"
                                            >
                                                {child.read().title.clone()}
                                            </button>
                                        </td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-gray-400">
                                            {child.read().num_games}
                                        </td>
                                        <td>{handles_count}</td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-gray-400">
                                            {format!("{}", child.read().min_date)}
                                        </td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-gray-400">
                                            {format!("{}", child.read().max_date)}
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
