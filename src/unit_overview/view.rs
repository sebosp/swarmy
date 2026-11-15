//! A unit with its costs, life, weapons, etc in every patch, the changes from the previous patch
//! are marked: red up when the unit costs more, green down when it costs less.

use crate::components::abilities::group_digits;
use crate::components::brand_icon::{BrandIcon, native_title};
use crate::components::debug::Debuggable;
use crate::components::links::{MapLink, PatchLink, PlayerLink, ReplayLink, UnitLink};
use crate::components::loading::Loading;
use crate::components::patch::ChangeMarker;
use crate::components::unit_icon::{AbilityList, PropertyLabel, UnitIcon};
use crate::components::usage_over_time::UsageOverTimeChart;
use crate::navigation::use_navigation;
use crate::replay_list::actions::{trigger_swarmy_bevy_on_replay, trigger_swarmy_rerun_on_replay};
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

const RACES: [&str; 4] = ["Terran", "Zerg", "Protoss", "Neutral"];

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
    // The race first, then its units: the race of the unit shown, else the one clicked.
    let race = RwSignal::new(String::new());
    Effect::new(move |_| {
        let name = navigation.unit_name.get();
        if let Some(found) = unit_list.with(|list| {
            list.units
                .iter()
                .find(|unit| unit.name == name)
                .map(|unit| unit.race.clone())
        }) {
            race.set(found);
        }
    });
    let pick_race = Callback::new(move |race: String| {
        let current = navigation.unit_name.get_untracked();
        let first = unit_list.with_untracked(|list| {
            let of_race = |unit: &&PatchUnitName| unit.race == race;
            if list
                .units
                .iter()
                .filter(of_race)
                .any(|unit| unit.name == current)
            {
                None
            } else {
                list.units
                    .iter()
                    .find(of_race)
                    .map(|unit| unit.name.clone())
            }
        });
        if let Some(first) = first {
            navigation.unit_name.set(first);
        }
    });
    let options = move || {
        let race = race.get();
        let selected = navigation.unit_name.get();
        unit_list
            .get()
            .units
            .into_iter()
            .filter(|unit| unit.race == race)
            .map(|unit| {
                let is_selected = unit.name == selected;
                let label = unit.name.clone();
                view! { <option value=unit.name selected=is_selected>{label}</option> }
            })
            .collect_view()
    };
    view! {
        <div class="p-2 space-y-2">
            <h1 class="text-xl font-semibold text-base-content">"Unit Overview"</h1>
            <crate::components::race_buttons::RaceButtons value=race races=&RACES on_pick=pick_race />
            <div class="flex items-center gap-2">
                <label class="text-sm/6 font-medium text-base-content" for="unit-overview-unit">"Unit"</label>
                <select
                    id="unit-overview-unit"
                    class="select select-sm w-64 bg-base-200 text-base-content border border-base-content/20"
                    on:change=move |ev| navigation.unit_name.set(event_target_value(&ev))
                >
                    {options}
                </select>
                <span class="ml-auto">
                    {move || unit.get().map(|unit| view! { <UnitIcon unit_name=unit.name size="w-12 h-12" /> })}
                </span>
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
                    // Unchanged values are shown once, the next columns stay empty.
                    if previous.is_some() && value.change == PatchChange::Same {
                        return view! {
                            <td class="pr-3 py-1" title=format!("Unchanged: {}", value.value)></td>
                        }
                        .into_any();
                    }
                    let title = previous
                        .filter(|_| value.change != PatchChange::Same)
                        .map(|previous| format!("Was {}", previous.value))
                        .unwrap_or_default();
                    // Requirements and producers are units too.
                    let is_unit_list = matches!(row.label.as_str(), "Requires" | "Produced by");
                    let is_abilities = row.label == "Abilities";
                    let content = if is_abilities && value.value != "-" {
                        view! { <AbilityList abilities=value.value.clone() /> }.into_any()
                    } else if is_unit_list && value.value != "-" {
                        value
                            .value
                            .split(", ")
                            .filter(|name| !name.is_empty())
                            .map(|name| {
                                view! {
                                    <span class="mr-1">
                                        <UnitLink unit_name=name.to_string() />
                                    </span>
                                }
                            })
                            .collect_view()
                            .into_any()
                    } else {
                        value.value.clone().into_any()
                    };
                    view! {
                        <td class="pr-3 py-1" class:whitespace-nowrap=!is_abilities title=title>
                            <ChangeMarker kind change=value.change />
                            {content}
                        </td>
                    }
                    .into_any()
                })
                .collect_view();
            let label = row.label.clone();
            view! {
                <tr class=if show_label { "border-t border-base-content/20" } else { "" }>
                    <td class="pr-3 py-1 text-base-content whitespace-nowrap">
                        {show_label.then(|| view! { <PropertyLabel label /> })}
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
        <div class="overflow-x-auto">
            <table class="min-w-full text-xs text-muted">
                <thead>
                    <tr class="text-left text-muted">
                        <th class="pr-3">"Patch"</th>
                        <th></th>
                        {headers}
                    </tr>
                </thead>
                <tbody class="divide-y divide-base-content/5">{rows}</tbody>
            </table>
        </div>
        <UpgradeTrees trees=unit.upgrade_trees.clone() />
    }
}

/// A row of the tree of the upgrades: the branch lines, the level and what it needs.
struct UpgradeRow {
    branches: String,
    node: UpgradeNode,
}

/// The levels of the upgrades as rows of a tree, the lines before each level as in the Opening
/// of the Replay view.
fn upgrade_rows(nodes: &[UpgradeNode], prefix: &str, roots: bool, rows: &mut Vec<UpgradeRow>) {
    let count = nodes.len();
    for (idx, node) in nodes.iter().enumerate() {
        let last = idx + 1 == count;
        let (branch, below) = match (roots, last) {
            (true, _) => ("", ""),
            (false, true) => ("└─ ", "   "),
            (false, false) => ("├─ ", "│  "),
        };
        rows.push(UpgradeRow {
            branches: format!("{prefix}{branch}"),
            node: UpgradeNode {
                children: vec![],
                ..node.clone()
            },
        });
        upgrade_rows(&node.children, &format!("{prefix}{below}"), false, rows);
    }
}

/// The upgrades of the unit in the latest patch as a dependency tree: each level under the level
/// it requires, with its cost and the structure it needs.
#[component]
fn UpgradeTrees(trees: Vec<UpgradeNode>) -> impl IntoView {
    if trees.is_empty() {
        return ().into_any();
    }
    let mut rows = vec![];
    upgrade_rows(&trees, "", true, &mut rows);
    view! {
        <h2 class="text-base/7 font-semibold text-base-content">"Upgrades"</h2>
        <table class="text-xs text-muted">
            <tbody>
                {rows
                    .into_iter()
                    .map(|row| {
                        let needs = row.node.requires_units.clone();
                        view! {
                            <tr>
                                <td class="pr-3 py-0.5 text-base-content whitespace-nowrap">
                                    <span class="font-mono whitespace-pre text-faint">{row.branches}</span>
                                    <UnitIcon unit_name=row.node.id.clone() size="w-5 h-5" />
                                    {row.node.id.clone()}
                                </td>
                                <td class="py-0.5 whitespace-nowrap">
                                    {(!needs.is_empty())
                                        .then(|| {
                                            view! {
                                                "requires "
                                                {needs
                                                    .into_iter()
                                                    .map(|unit| view! { <span class="mr-1"><UnitLink unit_name=unit /></span> })
                                                    .collect_view()}
                                            }
                                        })}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
    .into_any()
}

/// The morphs of the unit in the games (`unit_stats::forms`): what it changes into and back, how
/// often, one change per line, and the stats of each form in the latest balance data side by
/// side.
#[component]
fn UnitForms(forms: Vec<UnitForm>, form_changes: Vec<UnitFormChange>) -> impl IntoView {
    if form_changes.is_empty() {
        return ().into_any();
    }
    view! {
        <Debuggable query="patch_overview::unit_stats::forms" view_source=crate::source!()>
        <h2 class="text-base/7 font-semibold text-base-content">"Morphs"</h2>
        <table class="text-xs text-muted">
            <tbody>
                {form_changes
                    .into_iter()
                    .map(|change| {
                        view! {
                            <tr>
                                <td class="pr-3 py-0.5 whitespace-nowrap">
                                    <UnitLink unit_name=change.from />
                                    " → "
                                    <UnitLink unit_name=change.to />
                                </td>
                                <td class="py-0.5 num text-faint whitespace-nowrap">
                                    {format!("{} times", group_digits(change.changes as u64))}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
        <table class="min-w-full text-xs text-muted">
            <thead>
                <tr class="text-left text-muted">
                    <th class="pr-3">"Morph"</th>
                    <th class="pr-3 num">"Life"</th>
                    <th class="pr-3 num">"Armor"</th>
                    <th class="pr-3 num">"Speed"</th>
                    <th class="pr-3" title="The weapons with their range">"Weapons"</th>
                    <th>"Abilities"</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-base-content/5">
                {forms
                    .into_iter()
                    .map(|form| {
                        view! {
                            <tr>
                                <td class="pr-3 py-1 text-base-content whitespace-nowrap"><UnitLink unit_name=form.name /></td>
                                <td class="pr-3 py-1 num">{form.life}</td>
                                <td class="pr-3 py-1 num">{form.armor}</td>
                                <td class="pr-3 py-1 num">{form.speed}</td>
                                <td class="pr-3 py-1 whitespace-nowrap">{form.weapons}</td>
                                <td class="py-1"><AbilityList abilities=form.abilities inline=true /></td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
        </Debuggable>
    }
    .into_any()
}

/// A label and a value, i.e. "Units made" "20,788".
fn stat(label: &'static str, value: String, title: &'static str) -> impl IntoView {
    view! {
        <div class="rounded-md border border-base-content/10 px-3 py-2" title=title>
            <div class="text-xs text-muted">{label}</div>
            <div class="text-lg font-semibold text-base-content">{value}</div>
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
        <ErrorMessage error />
        {move || match stats.get() {
            None if error.get().is_none() => {
                view! { <Loading message="Reading the snapshot" /> }.into_any()
            }
            None => ().into_any(),
            Some(stats) => view! {
                <UnitForms forms=stats.forms.clone() form_changes=stats.form_changes.clone() />
                <h2 class="text-base/7 font-semibold text-base-content">"Stats"</h2>
                <UnitSnapshotStatsView stats />
            }
                .into_any(),
        }}
    }
}

#[component]
fn UnitSnapshotStatsView(stats: UnitSnapshotStats) -> impl IntoView {
    let games_percent = 100. * stats.games_with_unit as f64 / stats.games.max(1) as f64;
    // The strings of the panel before the view: each Debuggable takes its children by value.
    let games = format!(
        "{} of {} ({games_percent:.0}%)",
        stats.games_with_unit, stats.games
    );
    let born = stats.born.to_string();
    let died = stats.died.to_string();
    let kills = stats.kills.to_string();
    let kills_per_unit = format!("{:.2}", stats.kills_per_unit());
    let value_per_cost = format!("{:.2}", stats.value_per_cost());
    let threshold = format!(
        "The players that made at least {EXCEPTIONAL_MIN_BORN} and killed at least {:.2} enemy \
         units per unit, two standard deviations above the average of {:.2}.",
        stats.kills_per_unit_threshold, stats.kills_per_unit_mean,
    );
    view! {
        <Debuggable query="patch_overview::unit_stats::totals" view_source=crate::source!()>
        <div class="grid grid-cols-2 md:grid-cols-6 gap-2">
            {stat("Games", games, "The games in which at least one player made the unit")}
            {stat("Units made", born, "Burrowing, sieging, etc is the same unit")}
            {stat("Died", died, "The units that died")}
            {stat("Killed", kills, "The enemy units that cost resources killed by the unit")}
            {stat("Kills per unit", kills_per_unit, "The enemy units killed for each unit made")}
            {stat(
                "Value killed per cost",
                value_per_cost,
                "The minerals and vespene of the enemy units killed for each mineral and vespene spent on the unit, at the latest patch cost",
            )}
        </div>
        </Debuggable>
        <h3 class="text-sm font-semibold text-base-content">"Use over time"</h3>
        <UsageOverTimeChart subject=UsageSubject::Unit(stats.unit_name) />
        <h3 class="text-sm font-semibold text-base-content">"Exceptional games"</h3>
        <Debuggable query="patch_overview::unit_stats::exceptional" view_source=crate::source!()>
        <p class="text-xs text-faint">{threshold}</p>
        <ExceptionalGames games=stats.exceptional />
        </Debuggable>
    }
}

#[component]
fn ExceptionalGames(games: Vec<ExceptionalUnitGame>) -> impl IntoView {
    if games.is_empty() {
        return view! { <p class="text-xs text-faint">"No exceptional games."</p> }.into_any();
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
                    <td class="pr-3 py-1 num">{game.born}</td>
                    <td class="pr-3 py-1 num">{game.kills}</td>
                    <td class="pr-3 py-1 num text-success">{format!("{:.2}", game.kills_per_unit())}</td>
                    <td class="pr-3 py-1 num">{game.value_killed}</td>
                    <td class="py-1 whitespace-nowrap">
                        <button
                            class="btn btn-primary btn-xs b-0 native-only"
                            title=native_title("Load Rerun Preview")
                            on:click=move |_ev: MouseEvent| trigger_swarmy_rerun_on_replay(&rerun_location)
                        >
                            <BrandIcon name="rerun" />
                        </button>
                        <button
                            class="btn btn-primary btn-xs b-0 native-only"
                            title=native_title("Load in swarmy-bevy: the units born and died over time on the map")
                            on:click=move |_ev: MouseEvent| trigger_swarmy_bevy_on_replay(&bevy_location)
                        >
                            <BrandIcon name="bevy" />
                        </button>
                    </td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <div class="overflow-x-auto">
            <table class="min-w-full text-xs text-muted">
                <thead>
                    <tr class="text-left text-muted">
                        <th class="pr-3">"Replay"</th>
                        <th class="pr-3">"Date"</th>
                        <th class="pr-3">"Map"</th>
                        <th class="pr-3">"Player"</th>
                        <th class="pr-3">"Patch"</th>
                        <th class="pr-3 num">"Made"</th>
                        <th class="pr-3 num">"Killed"</th>
                        <th class="pr-3 num">"Kills per unit"</th>
                        <th class="pr-3 num" title="Minerals and vespene of the enemy units killed">"Value killed"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody class="divide-y divide-base-content/5">{rows}</tbody>
            </table>
        </div>
    }
    .into_any()
}
