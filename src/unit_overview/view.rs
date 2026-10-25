//! A unit with its costs, life, weapons, etc in every patch, the changes from the previous patch
//! are marked: red up when the unit costs more, green down when it costs less.

use crate::components::charts::UnitUsageChart;
use crate::components::links::{MapLink, PatchLink, PlayerLink, RaceLink, ReplayLink, UnitLink};
use crate::components::loading::Loading;
use crate::components::patch::ChangeMarker;
use crate::components::unit_icon::UnitIcon;
use crate::navigation::use_navigation;
use crate::replay_list::actions::{trigger_swarmy_bevy_on_replay, trigger_swarmy_rerun_on_replay};
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, CUBE, STRATEGY};
use swarmy_common::*;

const RACES: [&str; 4] = ["Terran", "Zerg", "Protoss", "Other"];

#[component]
pub fn UnitOverviewView() -> impl IntoView {
    let navigation = use_navigation();
    let unit_list = RwSignal::new(PatchUnitList::default());
    let unit = RwSignal::new(None::<PatchUnit>);
    let error = RwSignal::new(None::<String>);
    spawn_local(async move {
        match invoke_json::<PatchUnitList>("list_patch_units", &()).await {
            Ok(list) => unit_list.set(list),
            Err(err) => error.set(Some(err)),
        }
    });
    Effect::new(move |_| {
        let unit_name = navigation.unit_name.get();
        if unit_name.is_empty() {
            return;
        }
        spawn_local(async move {
            match invoke_json::<PatchUnit>("query_unit_overview", &PatchUnitQuery { unit_name })
                .await
            {
                Ok(response) => {
                    error.set(None);
                    unit.set(Some(response));
                }
                Err(err) => error.set(Some(err)),
            }
        });
    });
    // The units grouped by race.
    let options = move || {
        let units = unit_list.get().units;
        let selected = navigation.unit_name.get_untracked();
        RACES
            .iter()
            .map(|race| {
                let race_units: Vec<String> = units
                    .iter()
                    .filter(|unit| unit.race == *race)
                    .map(|unit| unit.name.clone())
                    .collect();
                view! {
                    <optgroup label=*race>
                        {race_units
                            .into_iter()
                            .map(|name| {
                                let is_selected = name == selected;
                                let label = name.clone();
                                view! { <option value=name selected=is_selected>{label}</option> }
                            })
                            .collect_view()}
                    </optgroup>
                }
            })
            .collect_view()
    };
    view! {
        <div class="p-2 space-y-2">
            <h1 class="text-xl font-semibold text-white">"Unit overview"</h1>
            <p class="text-xs text-gray-500">
                <span class="text-red-500">"▲"</span>
                " the unit costs more or got worse than in the previous patch, "
                <span class="text-green-500">"▼"</span>
                " it costs less, "
                <span class="text-green-500">"▲"</span>
                " / "
                <span class="text-red-500">"▼"</span>
                " more or less of a property that is better the higher, i.e. life, "
                <span class="text-yellow-400">"●"</span>
                " changed. Hover a value to see the previous one."
            </p>
            <div class="grid grid-cols-10 gap-2 items-end">
                <div class="col-span-3">
                    <label class="block text-sm/6 font-medium text-white">"Unit"</label>
                    <select
                        class="select select-sm w-full bg-gray-800 text-gray-200 border border-white/20"
                        on:change=move |ev| navigation.unit_name.set(event_target_value(&ev))
                    >
                        {options}
                    </select>
                </div>
            </div>
            <ErrorMessage error />
            {move || unit.get().map(|unit| view! { <UnitTable unit /> })}
            {move || {
                let unit_name = navigation.unit_name.get();
                (!unit_name.is_empty()).then(|| view! { <UnitSnapshotPanel unit_name /> })
            }}
        </div>
    }
}

/// Consecutive patches without any change of the unit are shown as one column, as the indices of
/// the first and last patch of each column.
fn version_columns(unit: &PatchUnit) -> Vec<(usize, usize)> {
    let mut columns: Vec<(usize, usize)> = vec![];
    for idx in 0..unit.versions.len() {
        let changed = unit
            .rows
            .iter()
            .any(|row| row.values[idx].change != PatchChange::Same);
        match columns.last_mut() {
            Some(column) if !changed => column.1 = idx,
            _ => columns.push((idx, idx)),
        }
    }
    columns
}

#[component]
fn UnitTable(unit: PatchUnit) -> impl IntoView {
    let columns = version_columns(&unit);
    let headers = columns
        .iter()
        .map(|(first, last)| {
            let (first_version, last_version) = (unit.versions[*first], unit.versions[*last]);
            if first == last {
                view! { <th class="pr-3">{first_version}</th> }.into_any()
            } else {
                let unchanged = unit.versions[first + 1..=*last]
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                view! {
                    <th class="pr-3" title=format!("No changes in {unchanged}")>
                        {format!("{first_version} – {last_version}")}
                    </th>
                }
                .into_any()
            }
        })
        .collect_view();
    // The label is only shown on the first row of a group, i.e. the first "Strong vs" row.
    let mut previous_label = String::new();
    let rows = unit
        .rows
        .iter()
        .map(|row| {
            let kind = row.kind;
            let show_label = row.label != previous_label;
            previous_label = row.label.clone();
            let cells = columns
                .iter()
                .map(|(first, _)| {
                    let value = &row.values[*first];
                    let previous = first.checked_sub(1).map(|previous| &row.values[previous]);
                    // Unchanged values are shown once, the next columns only mark them as
                    // unchanged.
                    if previous.is_some() && value.change == PatchChange::Same {
                        return view! {
                            <td class="pr-3 py-1 text-gray-600" title=format!("Unchanged: {}", value.value)>
                                "〃"
                            </td>
                        }
                        .into_any();
                    }
                    let title = previous
                        .filter(|_| value.change != PatchChange::Same)
                        .map(|previous| format!("Was {}", previous.value))
                        .unwrap_or_default();
                    // Requirements and producers are units too.
                    let is_unit_list = matches!(row.label.as_str(), "Requires" | "Produced by");
                    let content = if is_unit_list && value.value != "-" {
                        value
                            .value
                            .split(", ")
                            .filter(|name| !name.is_empty())
                            .map(|name| view! {
                                <span class="mr-1">
                                    <UnitLink unit_name=name.to_string() />
                                </span>
                            })
                            .collect_view()
                            .into_any()
                    } else {
                        value.value.clone().into_any()
                    };
                    view! {
                        <td class="pr-3 py-1 whitespace-nowrap" title=title>
                            <ChangeMarker kind change=value.change />
                            {content}
                        </td>
                    }
                    .into_any()
                })
                .collect_view();
            let label = row.label.clone();
            view! {
                <tr class=if show_label { "border-t border-white/20" } else { "" }>
                    <td class="pr-3 py-1 text-white whitespace-nowrap">
                        {show_label.then_some(label)}
                    </td>
                    <td class="pr-3 py-1 whitespace-nowrap">
                        {row.unit.clone().map(|counter| view! { <UnitLink unit_name=counter /> })}
                    </td>
                    {cells}
                </tr>
            }
        })
        .collect_view();
    view! {
        <h2 class="text-base/7 font-semibold text-white">
            <UnitIcon unit_name=unit.name.clone() size="w-12 h-12" />
            {format!("{} (", unit.name)}
            <RaceLink race=unit.race.clone() />
            ")"
        </h2>
        <p class="text-xs text-gray-500">
            "〃 the value did not change from the previous column, hover to see it. Patches without \
            any change of the unit are merged into one column."
        </p>
        <div class="overflow-x-auto">
            <table class="min-w-full text-xs text-gray-400">
                <thead>
                    <tr class="text-left text-gray-300">
                        <th class="pr-3">"Patch"</th>
                        <th></th>
                        {headers}
                    </tr>
                </thead>
                <tbody class="divide-y divide-white/5">{rows}</tbody>
            </table>
        </div>
    }
}

/// A label and a value, i.e. "Units made" "20,788".
fn stat(label: &'static str, value: String, title: &'static str) -> impl IntoView {
    view! {
        <div class="rounded-md border border-white/10 px-3 py-2" title=title>
            <div class="text-xs text-gray-400">{label}</div>
            <div class="text-lg font-semibold text-white">{value}</div>
        </div>
    }
}

/// How the unit was used in the games of the snapshot.
#[component]
fn UnitSnapshotPanel(unit_name: String) -> impl IntoView {
    let stats = RwSignal::new(None::<UnitSnapshotStats>);
    let error = RwSignal::new(None::<String>);
    spawn_local(async move {
        match invoke_json::<UnitSnapshotStats>(
            "query_unit_snapshot_stats",
            &PatchUnitQuery { unit_name },
        )
        .await
        {
            Ok(response) => stats.set(Some(response)),
            Err(err) => error.set(Some(err)),
        }
    });
    view! {
        <h2 class="text-base/7 font-semibold text-white">"In the snapshot"</h2>
        <ErrorMessage error />
        {move || match stats.get() {
            None if error.get().is_none() => {
                view! { <Loading message="Reading the snapshot" /> }.into_any()
            }
            None => ().into_any(),
            Some(stats) => view! { <UnitSnapshotStatsView stats /> }.into_any(),
        }}
    }
}

#[component]
fn UnitSnapshotStatsView(stats: UnitSnapshotStats) -> impl IntoView {
    let games_percent = 100. * stats.games_with_unit as f64 / stats.games.max(1) as f64;
    view! {
        <div class="grid grid-cols-2 md:grid-cols-6 gap-2">
            {stat(
                "Games",
                format!("{} of {} ({games_percent:.0}%)", stats.games_with_unit, stats.games),
                "The games in which at least one player made the unit",
            )}
            {stat("Units made", stats.born.to_string(), "Burrowing, sieging, etc is the same unit")}
            {stat("Died", stats.died.to_string(), "The units that died")}
            {stat(
                "Killed",
                stats.kills.to_string(),
                "The enemy units that cost resources killed by the unit",
            )}
            {stat(
                "Kills per unit",
                format!("{:.2}", stats.kills_per_unit()),
                "The enemy units killed for each unit made",
            )}
            {stat(
                "Value killed per cost",
                format!("{:.2}", stats.value_per_cost()),
                "The minerals and vespene of the enemy units killed for each mineral and vespene spent on the unit, at the latest patch cost",
            )}
        </div>
        <h3 class="text-sm font-semibold text-white">"Use over time"</h3>
        <p class="text-xs text-gray-500">
            "The games of each month in which at least one player made the unit, hover a month to \
            see the units made per game."
        </p>
        <UnitUsageChart data=stats.by_month.clone() />
        <h3 class="text-sm font-semibold text-white">"Exceptional games"</h3>
        <p class="text-xs text-gray-500">
            {format!(
                "The players that made at least {EXCEPTIONAL_MIN_BORN} and killed at least {:.2} \
                 enemy units per unit, two standard deviations above the average of {:.2}.",
                stats.kills_per_unit_threshold,
                stats.kills_per_unit_mean,
            )}
        </p>
        <ExceptionalGames games=stats.exceptional />
    }
}

#[component]
fn ExceptionalGames(games: Vec<ExceptionalUnitGame>) -> impl IntoView {
    if games.is_empty() {
        return view! { <p class="text-xs text-gray-500">"No exceptional games."</p> }.into_any();
    }
    let rows = games
        .into_iter()
        .map(|game| {
            let rerun_location = game.replay_location.clone();
            let bevy_location = game.replay_location.clone();
            view! {
                <tr>
                    <td class="pr-3 py-1"><ReplayLink ext_fs_id=game.ext_fs_id /></td>
                    <td class="pr-3 py-1 whitespace-nowrap">{game.date.clone()}</td>
                    <td class="pr-3 py-1"><MapLink map_title=game.map_title.clone() /></td>
                    <td class="pr-3 py-1"><PlayerLink player_name=game.player_name.clone() /></td>
                    <td class="pr-3 py-1"><PatchLink version=game.protocol_version /></td>
                    <td class="pr-3 py-1">{game.born}</td>
                    <td class="pr-3 py-1">{game.kills}</td>
                    <td class="pr-3 py-1 text-green-400">{format!("{:.2}", game.kills_per_unit())}</td>
                    <td class="pr-3 py-1">{game.value_killed}</td>
                    <td class="py-1 whitespace-nowrap">
                        <button
                            class="btn btn-primary btn-xs b-0"
                            title="Load Rerun Preview"
                            on:click=move |_ev: MouseEvent| trigger_swarmy_rerun_on_replay(&rerun_location)
                        >
                            <Icon icon=STRATEGY weight=IconWeight::Light prop:class="stroke-current" />
                        </button>
                        <button
                            class="btn btn-primary btn-xs b-0"
                            title="Load in swarmy-bevy: the units born and died over time on the map"
                            on:click=move |_ev: MouseEvent| trigger_swarmy_bevy_on_replay(&bevy_location)
                        >
                            <Icon icon=CUBE weight=IconWeight::Light prop:class="stroke-current" />
                        </button>
                    </td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full text-xs text-gray-400">
                <thead>
                    <tr class="text-left text-gray-300">
                        <th class="pr-3">"Replay"</th>
                        <th class="pr-3">"Date"</th>
                        <th class="pr-3">"Map"</th>
                        <th class="pr-3">"Player"</th>
                        <th class="pr-3">"Patch"</th>
                        <th class="pr-3">"Made"</th>
                        <th class="pr-3">"Killed"</th>
                        <th class="pr-3">"Kills per unit"</th>
                        <th class="pr-3" title="Minerals and vespene of the enemy units killed">"Value killed"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-white/5">{rows}</tbody>
            </table>
        </div>
    }
    .into_any()
}
