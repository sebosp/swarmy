use super::actions::*;
use super::*;
use crate::components::links::{MapLink, PlayerLink, RaceLink, ReplayLink, ResultMarker};
use crate::components::loading::Loading;
use crate::navigation::use_navigation;
use crate::*;
use chrono::Utc;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, CLIPBOARD, CUBE, FOLDER_OPEN, STRATEGY};
use reactive_stores::Store;
use swarmy_common::*;

/// The 1v1 matchups, the races sorted as in Map Details.
const MATCHUPS: [&str; 6] = ["PvP", "PvT", "PvZ", "TvT", "TvZ", "ZvZ"];

const SELECT_CLASS: &str =
    "select select-sm w-full bg-gray-800 text-gray-200 border border-white/20";
const LABEL_CLASS: &str = "block text-sm/6 font-medium text-white";

/// A dropdown of the replay list filters, `options` are the values and their labels, the first
/// one is the default.
#[component]
fn FilterSelect(
    label: &'static str,
    #[prop(into)] options: Signal<Vec<(String, String)>>,
    #[prop(into)] selected: Signal<String>,
    on_select: Callback<String>,
) -> impl IntoView {
    view! {
        <div>
            <label class=LABEL_CLASS>{label}</label>
            <select class=SELECT_CLASS on:change=move |ev| on_select.run(event_target_value(&ev))>
                {move || {
                    options
                        .get()
                        .into_iter()
                        .map(|(value, text)| {
                            let is_selected = value == selected.get();
                            view! { <option value=value selected=is_selected>{text}</option> }
                        })
                        .collect_view()
                }}
            </select>
        </div>
    }
}

#[component]
pub fn ReplayList(/*active_page: RwSignal<String>*/) -> impl IntoView {
    let navigation = use_navigation();
    let replay_query = navigation.replay_query;
    let time_frame = navigation.replay_time_frame;
    let filters = RwSignal::new(ReplayListFilters::default());
    spawn_local(async move {
        match invoke_json::<ReplayListFilters>("list_replay_filters", &()).await {
            Ok(loaded) => filters.set(loaded),
            Err(err) => leptos::logging::error!("Error listing the replay filters: {err}"),
        }
    });
    let (backend_response, set_backend_response) = signal(ApiResponse {
        meta: ResponseMeta::incomplete(),
        message: String::new(),
    });
    let replay_list_store = Store::new(ReplayListDataTable::default());
    // Any change of the filters or the sort queries the backend again.
    Effect::new(move |_| {
        trigger_fetch_query_replay_list(
            replay_list_store,
            replay_query.get(),
            set_backend_response,
        );
    });
    let text_filter = move |name: &'static str,
                            label: &'static str,
                            get: fn(&ReplayListQuery) -> String,
                            set: fn(&mut ReplayListQuery, String)| {
        view! {
            <div class="col-span-2">
                <label for=name class=LABEL_CLASS>{label}</label>
                <input
                    name=name
                    class=text_input_tailwind_classes().join(" ")
                    prop:value=move || get(&replay_query.read())
                    on:input=move |ev| replay_query.update(|query| set(query, event_target_value(&ev)))
                    type="text"
                />
            </div>
        }
    };
    let matchup_options = Signal::derive(|| {
        std::iter::once((String::new(), "Any".to_string()))
            .chain(
                MATCHUPS
                    .iter()
                    .map(|matchup| (matchup.to_string(), matchup.to_string())),
            )
            .collect::<Vec<_>>()
    });
    let team_size_options = Signal::derive(|| {
        std::iter::once((String::new(), "Any".to_string()))
            .chain((1..=4).map(|size| (size.to_string(), format!("{size}v{size}"))))
            .collect::<Vec<_>>()
    });
    let time_frames = move || {
        [
            TimeFrame::AllTime,
            TimeFrame::LastWeek,
            TimeFrame::LastMonth,
            TimeFrame::LastThreeMonths,
            TimeFrame::LastYear,
        ]
        .into_iter()
        .chain(
            filters
                .read()
                .years
                .iter()
                .map(|year| TimeFrame::Year(*year)),
        )
        .collect::<Vec<_>>()
    };
    let time_frame_options = Signal::derive(move || {
        time_frames()
            .into_iter()
            .enumerate()
            .map(|(idx, frame)| (idx.to_string(), frame.label()))
            .collect::<Vec<_>>()
    });
    let patch_options = Signal::derive(move || {
        std::iter::once((String::new(), "Any".to_string()))
            .chain(filters.read().protocol_versions.iter().map(|version| {
                (
                    version.label.clone(),
                    format!("{} ({} games)", version.label, version.games),
                )
            }))
            .collect::<Vec<_>>()
    });
    view! {
        <div class="grid grid-cols-10 gap-2 items-end">
            {text_filter("map_title", "Map", |query| query.map_title.clone(), |query, value| query.map_title = value)}
            {text_filter("player_name", "Player", |query| query.player_name.clone(), |query, value| query.player_name = value)}
            <div class="col-span-1">
                <FilterSelect
                    label="Matchup"
                    options=matchup_options
                    selected=Signal::derive(move || normalize_matchup(&replay_query.read().matchup))
                    on_select=Callback::new(move |matchup: String| {
                        replay_query.update(|query| query.matchup = matchup)
                    })
                />
            </div>
            <div class="col-span-1">
                <FilterSelect
                    label="Game"
                    options=team_size_options
                    selected=Signal::derive(move || {
                        replay_query.read().team_size.map(|size| size.to_string()).unwrap_or_default()
                    })
                    on_select=Callback::new(move |size: String| {
                        replay_query.update(|query| query.team_size = size.parse().ok())
                    })
                />
            </div>
            <div class="col-span-2">
                <FilterSelect
                    label="Time frame"
                    options=time_frame_options
                    selected=Signal::derive(move || {
                        time_frames()
                            .iter()
                            .position(|frame| *frame == time_frame.get())
                            .unwrap_or_default()
                            .to_string()
                    })
                    on_select=Callback::new(move |idx: String| {
                        let frame = idx
                            .parse::<usize>()
                            .ok()
                            .and_then(|idx| time_frames().get(idx).copied())
                            .unwrap_or_default();
                        let today = Utc::now().date_naive();
                        time_frame.set(frame);
                        replay_query.update(|query| {
                            query.min_date = frame.min_date(today);
                            query.max_date = frame.max_date();
                        });
                    })
                />
            </div>
            <div class="col-span-2">
                <FilterSelect
                    label="Patch"
                    options=patch_options
                    selected=Signal::derive(move || {
                        replay_query
                            .read()
                            .protocol_version
                            .map(|version| version.to_string())
                            .unwrap_or_default()
                    })
                    on_select=Callback::new(move |version: String| {
                        replay_query.update(|query| query.protocol_version = version.parse().ok())
                    })
                />
            </div>
        </div>
        <DisplayBackendStatus backend_response />
        <div class="col-span-10">
            <Show when=move || !backend_response.get().meta.is_complete>
                <Loading message="Collecting the replays of the snapshot..." />
            </Show>
            <Show when=move || backend_response.get().meta.is_complete>
                <h2 class="text-base/7 font-semibold text-white">
                    {move || {
                        let total = replay_list_store.total().get();
                        let shown = replay_list_store.data().read().len();
                        if shown < total {
                            format!(
                                "Showing the first {shown} of {total} replays found in the snapshot.",
                            )
                        } else {
                            format!("{total} replays found in the snapshot.")
                        }
                    }}
                </h2>
            </Show>
            <Show when=move || { replay_list_store.total().get() > 0 }>
                <ReplayListDataTable replay_list_store replay_query />
            </Show>
        </div>
    }
}

/// A header of the replay list, clicking it sorts the replays by its column in the backend,
/// clicking it again reverses the order.
#[component]
fn SortHeader(
    label: &'static str,
    sort_by: ReplayListSort,
    replay_query: RwSignal<ReplayListQuery>,
) -> impl IntoView {
    let aria_sort = move || {
        let query = replay_query.read();
        (query.sort_by == sort_by).then_some(if query.descending {
            "descending"
        } else {
            "ascending"
        })
    };
    view! {
        <th
            scope="col"
            class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white cursor-pointer select-none hover:text-indigo-300"
            aria-sort=aria_sort
            title="Sort by this column, click again to reverse"
            on:click=move |_| {
                replay_query.update(|query| {
                    if query.sort_by == sort_by {
                        query.descending = !query.descending;
                    } else {
                        query.sort_by = sort_by;
                        // The newest, longest games and the highest ids first.
                        query.descending = matches!(
                            sort_by,
                            ReplayListSort::Date | ReplayListSort::Duration | ReplayListSort::Id
                        );
                    }
                })
            }
        >
            {label}
        </th>
    }
}

#[component]
pub fn ReplayListDataTable(
    replay_list_store: Store<ReplayListDataTable>,
    /// The sort of the replays.
    replay_query: RwSignal<ReplayListQuery>,
) -> impl IntoView {
    view! {
        <div class="-mx-4 -my-2 overflow-x-auto sm:-mx-6 lg:-mx-8">
            <div class="inline-block min-w-full py-2 align-middle sm:px-6 lg:px-8">
                <table class="relative min-w-full divide-y divide-white/15" data-server-sort>
                    <thead>
                        <tr>
                            <SortHeader label="ID" sort_by=ReplayListSort::Id replay_query />
                            <SortHeader label="Map" sort_by=ReplayListSort::Map replay_query />
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Actions"
                            </th>
                            <SortHeader label="Players" sort_by=ReplayListSort::Players replay_query />
                            <SortHeader label="Duration" sort_by=ReplayListSort::Duration replay_query />
                            <SortHeader label="Races" sort_by=ReplayListSort::Races replay_query />
                            <SortHeader label="Date" sort_by=ReplayListSort::Date replay_query />
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-white/10 bg-gray-900">
                        <For
                            each=move || replay_list_store.data()
                            key=|row| { row.read().ext_fs_id }
                            children=|child| {
                                let players: Vec<(String, String)> = {
                                    let row = child.read();
                                    row.player_list
                                        .iter()
                                        .cloned()
                                        .zip(row.player_results.iter().cloned())
                                        .collect()
                                };
                                view! {
                                    <tr>
                                        <td>
                                            <span class="ps-0 pe-1 py-0 text-xs">
                                                <ReplayLink ext_fs_id=child.read().ext_fs_id />
                                            </span>
                                        </td>
                                        <td>
                                            <span class="px-0 py-0 text-xs text-gray-400">
                                                <MapLink map_title=child.read().map_title.clone() />
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
                                                    icon=CLIPBOARD
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
                                            <button
                                                class="btn btn-primary btn-xs b-0"
                                                on:click={
                                                    let location = child.read().replay_location.clone();
                                                    move |_ev: MouseEvent| {
                                                        trigger_swarmy_rerun_on_replay(&location)
                                                    }
                                                }
                                                title="Load Rerun Preview"
                                            >
                                                <Icon
                                                    icon=STRATEGY
                                                    weight=IconWeight::Light
                                                    prop:class="stroke-current"
                                                />
                                            </button>
                                            <button
                                                class="btn btn-primary btn-xs b-0"
                                                on:click={
                                                    let location = child.read().replay_location.clone();
                                                    move |_ev: MouseEvent| {
                                                        trigger_swarmy_bevy_on_replay(&location)
                                                    }
                                                }
                                                title="Load in swarmy-bevy: the units born and died over time on the map"
                                            >
                                                <Icon
                                                    icon=CUBE
                                                    weight=IconWeight::Light
                                                    prop:class="stroke-current"
                                                />
                                            </button>
                                        </td>
                                        <td class="px-2 py-2 text-xs whitespace-nowrap text-gray-400">
                                        <ul>
                                        { players
                                                .into_iter()
                                                .map(|(player_name, result)|
                                                    view! {
                                                        <li>
                                                            <ResultMarker result />
                                                            <PlayerLink player_name />
                                                        </li>
                                                    }
                                                )
                                                .collect_view()
                                        }
                                        </ul>
                                        </td>
                                        <td class="px-2 py-2 text-xs font-mono text-gray-400">
                                            {format_game_time(child.read().duration.max(0) as u32)}
                                        </td>
                                        <td>
                                            {child
                                                .read()
                                                .player_races
                                                .iter()
                                                .enumerate()
                                                .map(|(idx, race)| {
                                                    let label = Some(race_letter(race).to_string());
                                                    view! {
                                                        {(idx > 0).then_some(", ")}
                                                        <RaceLink race=race.clone() label=label.unwrap_or_default() />
                                                    }
                                                })
                                                .collect_view()}
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

pub fn blizzard_profile_link_href(
    player_toon_region: String,
    player_toon_realm: String,
    player_toon_id: String,
    player_name: String,
) -> impl IntoView {
    view! {
        <a
        href=move || format!(
            "https://starcraft2.blizzard.com/en-us/profile/{}/{}/{}",
            player_toon_region, player_toon_realm, player_toon_id
        )
        >
            {player_name}
        </a>
    }
}
