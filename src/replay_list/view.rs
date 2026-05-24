use super::actions::*;
use super::*;
use crate::*;
use chrono::Utc;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use phosphor_leptos::{Icon, IconWeight, FILE, FOLDER_OPEN};
use reactive_stores::Store;
use swarmy_common::*;

#[component]
pub fn ReplayList(active_page: RwSignal<String>) -> impl IntoView {
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
                <ReplayListDataTable replay_list_store active_page />
            </Show>
        </div>
    }
}

#[component]
pub fn ReplayListDataTable(
    replay_list_store: Store<ReplayListDataTable>,
    active_page: RwSignal<String>,
) -> impl IntoView {
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
                                "Map"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Path"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Player List"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Duration"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Player Win"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Date"
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
                                        <td>
                                            <span class="px-0 py-0 text-xs text-gray-400">
                                                {child.read().map_title.clone()}
                                            </span>
                                        </td>
                                        <td>
                                            <button
                                                class="btn btn-primary btn-xs b-0"
                                                on:click={
                                                    let value = child.read().replay_location.clone();
                                                    move |_ev: MouseEvent| {
                                                        trigger_request_copy_path_to_clipboard(&value)
                                                    }
                                                }
                                                title="Copy filename to clipboard"
                                            >
                                                <Icon
                                                    icon=FILE
                                                    weight=IconWeight::Light
                                                    prop:class="stroke-current"
                                                />
                                            </button>
                                            <button
                                                class="btn btn-primary btn-xs b-0"
                                                on:click={
                                                    let value = child.read().replay_location.clone();
                                                    let path = std::path::Path::new(&value).parent().unwrap();
                                                    let path_parent = format!("{}", path.display());
                                                    move |_ev: MouseEvent| {
                                                        trigger_request_open_folder(&path_parent)
                                                    }
                                                }
                                                title="Open directory"
                                            >
                                                <Icon
                                                    icon=FOLDER_OPEN
                                                    weight=IconWeight::Light
                                                    prop:class="stroke-current"
                                                />
                                            </button>
                                        </td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-gray-400">
                                            {child.read().player_list.join(", ")}
                                        </td>
                                        <td>{child.read().duration}</td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-gray-400">
                                            {format!("{}", child.read().winner_list.join(", "))}
                                        </td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-gray-400">
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
