use super::actions::*;
use super::*;
use crate::*;
use chrono::Utc;
use leptos::prelude::*;
use reactive_stores::Store;
use swarmy_tauri_common::*;

#[component]
pub fn ReplayList() -> impl IntoView {
    let player_name = RwSignal::new(String::new());
    let map_title = RwSignal::new(String::new());
    let query = move || ReplayListQuery {
        map_title: map_title.get(),
        player_name: player_name.get(),
        min_date: chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
        max_date: chrono::NaiveDate::from(Utc::now().date_naive()),
    };
    let (backend_response, set_backend_response) = signal(ApiResponse {
        meta: ResponseMeta::incomplete(),
        message: String::new(),
    });
    let replay_list_store = Store::new(ReplayListDataTable::default());
    trigger_fetch_query_replay_list(replay_list_store, query(), set_backend_response);
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
                            trigger_fetch_query_replay_list(
                                replay_list_store,
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
                            trigger_fetch_query_replay_list(
                                replay_list_store,
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
                {move || replay_list_store.total().get()} " Unique maps found in snapshot."
            </h2>
            <Show when=move || { replay_list_store.total().get() > 0 }>
                <ReplayListDataTable replay_list_store />
            </Show>
        </div>
    }
}

#[component]
pub fn ReplayListDataTable(replay_list_store: Store<ReplayListDataTable>) -> impl IntoView {
    view! {
        <div class="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
            <div class="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
                <table class="relative min-w-full divide-y divide-white/15">
                    <thead>
                        <tr>
                            <th
                                scope="col"
                                class="py-3.5 pr-3 pl-4 text-left text-sm font-semibold whitespace-nowrap sm:pl-0 text-white"
                            >
                                "Map Title"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Games"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Cache Handles"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Min Date"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Max Date"
                            </th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-white/10 bg-gray-900">
                        <For
                            each=move || replay_list_store.data()
                            key=|row| { row.read().sha256_sum.clone() }
                            children=|child| {
                                view! {
                                    <tr>
                                        <td>{child.read().replay_location.clone()}</td>
                                        <td class="px-2 py-2 text-sm whitespace-nowrap text-gray-400">
                                            {child.read().player_list.join(", ")}
                                        </td>
                                        <td>{child.read().duration}</td>
                                        <td class="px-2 py-2 text-sm whitespace-nowrap text-gray-400">
                                            {format!("{}", child.read().winner_list.join(", "))}
                                        </td>
                                        <td class="px-2 py-2 text-sm whitespace-nowrap text-gray-400">
                                            {format!("{}", child.read().replay_date)}
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
