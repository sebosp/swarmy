//! Patch Evolution: how the costs of units, and of what they train, build, research and upgrade,
//! change over the patches, as lines of a chart. The queries can be saved in the settings.

use crate::components::abilities::fetch_ability_usage;
use crate::components::charts::{ChartData, LineChart};
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, PLUS, TRASH};
use swarmy_common::*;

/// The colors of the lines, assigned by the position of the series: tailwind indigo, emerald,
/// amber, rose, sky, fuchsia, lime, orange.
const SERIES_COLORS: [&str; 8] = [
    "var(--chart-1)",
    "var(--chart-2)",
    "var(--chart-3)",
    "var(--chart-4)",
    "var(--chart-5)",
    "var(--chart-6)",
    "var(--chart-7)",
    "var(--chart-8)",
];

#[derive(serde::Serialize)]
struct UnitArgs {
    unit_name: String,
}

#[derive(serde::Serialize)]
struct SeriesArgs {
    series: Vec<EvolutionSeries>,
}

#[derive(serde::Serialize)]
struct SaveArgs {
    saved: SavedEvolution,
}

fn series_color(idx: usize) -> &'static str {
    SERIES_COLORS[idx % SERIES_COLORS.len()]
}

/// A series of the chart: its unit, item and metric, with its color.
#[component]
fn SeriesEditor(
    idx: usize,
    series: RwSignal<Vec<EvolutionSeries>>,
    units: RwSignal<Vec<PatchUnitName>>,
) -> impl IntoView {
    let items = RwSignal::new(Vec::<String>::new());
    let current = move || series.read().get(idx).cloned().unwrap_or_default();
    // The values are set once the options exist, the selected attribute of the options is not
    // enough when the editors are created again.
    let unit_select = NodeRef::<leptos::html::Select>::new();
    let item_select = NodeRef::<leptos::html::Select>::new();
    let metric_select = NodeRef::<leptos::html::Select>::new();
    Effect::new(move |_| {
        let current = current();
        units.track();
        items.track();
        if let Some(select) = unit_select.get() {
            select.set_value(&current.unit);
        }
        if let Some(select) = item_select.get() {
            select.set_value(&current.item);
        }
        if let Some(select) = metric_select.get() {
            select.set_value(current.metric.label());
        }
    });
    // The items of the unit, loaded when the unit changes.
    Effect::new(move |_| {
        let unit_name = current().unit;
        if unit_name.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(loaded) =
                invoke_json::<Vec<String>>("list_evolution_items", &UnitArgs { unit_name }).await
            {
                items.set(loaded);
            }
        });
    });
    let update = move |change: &dyn Fn(&mut EvolutionSeries)| {
        series.update(|series| {
            if let Some(entry) = series.get_mut(idx) {
                change(entry);
            }
        })
    };
    // The race first, then its units: the race of the unit of the line.
    let race = RwSignal::new(String::new());
    Effect::new(move |_| {
        let unit_name = current().unit;
        if let Some(found) = units.with(|units| {
            units
                .iter()
                .find(|unit| unit.name == unit_name)
                .map(|unit| unit.race.clone())
        }) {
            race.set(found);
        }
    });
    // A race moves the line to its first unit, unless the unit is of it.
    let pick_race = Callback::new(move |race: String| {
        let unit_name = current().unit;
        let first = units.with_untracked(|units| {
            let of_race = |unit: &&PatchUnitName| unit.race == race;
            if units
                .iter()
                .filter(of_race)
                .any(|unit| unit.name == unit_name)
            {
                None
            } else {
                units.iter().find(of_race).map(|unit| unit.name.clone())
            }
        });
        if let Some(first) = first {
            update(&|entry| {
                entry.unit = first.clone();
                entry.item = EVOLUTION_UNIT_ITEM.to_string();
            });
        }
    });
    view! {
        <div class="flex flex-wrap items-center gap-2">
            <span class="inline-block w-3 h-3 rounded-full" style=format!("background: {}", series_color(idx))></span>
            <crate::components::race_buttons::RaceButtons value=race on_pick=pick_race />
            <select
                class=SELECT_CLASS
                node_ref=unit_select
                on:change=move |ev| {
                    let unit = event_target_value(&ev);
                    update(&|entry| {
                        entry.unit = unit.clone();
                        entry.item = EVOLUTION_UNIT_ITEM.to_string();
                    })
                }
            >
                {move || {
                    let race = race.get();
                    units.get().into_iter().filter(|unit| unit.race == race).map(|unit| {
                        let name = unit.name.clone();
                        view! {
                            <option value=unit.name.clone() prop:selected=move || current().unit == name>
                                {unit.name.clone()}
                            </option>
                        }
                    }).collect_view()
                }}
            </select>
            <select
                class=SELECT_CLASS
                node_ref=item_select
                on:change=move |ev| {
                    let item = event_target_value(&ev);
                    update(&|entry| entry.item = item.clone())
                }
            >
                {move || items.get().into_iter().map(|item| {
                    let value = item.clone();
                    view! {
                        <option value=item.clone() prop:selected=move || current().item == value>
                            {item.clone()}
                        </option>
                    }
                }).collect_view()}
            </select>
            <select
                class=SELECT_CLASS
                node_ref=metric_select
                on:change=move |ev| {
                    let metric = event_target_value(&ev);
                    let metric = EvolutionMetric::ALL
                        .into_iter()
                        .find(|metric_value| metric_value.label() == metric)
                        .unwrap_or_default();
                    update(&|entry| entry.metric = metric)
                }
            >
                {EvolutionMetric::ALL.into_iter().map(|metric| {
                    view! {
                        <option value=metric.label() prop:selected=move || current().metric == metric>
                            {metric.label()}
                        </option>
                    }
                }).collect_view()}
            </select>
            <button
                class="btn btn-xs btn-ghost"
                title="Remove the line"
                on:click=move |_| series.update(|series| {
                    if idx < series.len() {
                        series.remove(idx);
                    }
                })
            >
                <Icon icon=TRASH weight=IconWeight::Light />
            </button>
        </div>
    }
}

#[component]
pub fn PatchEvolutionView() -> impl IntoView {
    let units = RwSignal::new(Vec::<PatchUnitName>::new());
    let series = RwSignal::new(Vec::<EvolutionSeries>::new());
    let evolution = RwSignal::new(None::<PatchEvolution>);
    let saved = RwSignal::new(Vec::<SavedEvolution>::new());
    let name = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    spawn_local(async move {
        if let Ok(list) = invoke_json::<PatchUnitList>("list_patch_units", &()).await {
            // The first line, the minerals of the Marine, shows how it works.
            if series.get_untracked().is_empty() {
                series.set(vec![EvolutionSeries {
                    unit: "Marine".to_string(),
                    item: EVOLUTION_UNIT_ITEM.to_string(),
                    metric: EvolutionMetric::Minerals,
                }]);
            }
            units.set(list.units);
        }
        if let Ok(loaded) = invoke_json::<Vec<SavedEvolution>>("list_saved_evolutions", &()).await {
            saved.set(loaded);
        }
    });
    // The chart follows the series.
    Effect::new(move |_| {
        let query = series.get();
        if query.is_empty() {
            evolution.set(None);
            return;
        }
        spawn_local(async move {
            match invoke_json::<PatchEvolution>(
                "query_patch_evolution",
                &SeriesArgs { series: query },
            )
            .await
            {
                Ok(loaded) => evolution.set(Some(loaded)),
                Err(err) => status.set(err),
            }
        });
    });
    let save = move |_| {
        let saved_query = SavedEvolution {
            name: name.get().trim().to_string(),
            series: series.get(),
        };
        if saved_query.name.is_empty() {
            status.set("Name the query to save it".to_string());
            return;
        }
        spawn_local(async move {
            match invoke_json::<Vec<SavedEvolution>>(
                "save_evolution",
                &SaveArgs { saved: saved_query },
            )
            .await
            {
                Ok(all) => {
                    saved.set(all);
                    status.set("Saved".to_string());
                }
                Err(err) => status.set(err),
            }
        });
    };
    let chart = move || {
        let evolution = evolution.get()?;
        let query = series.get();
        let mut chart = ChartData::new(evolution.versions.iter().map(u32::to_string).collect())
            .with_owned_colors(
                (0..query.len())
                    .map(|idx| series_color(idx).to_string())
                    .collect(),
            )
            // The costs change from one patch to the next, they are not interpolated.
            .with_curve("stepline");
        for (entry, values) in query.iter().zip(evolution.values) {
            chart = chart.with_optional_series(&entry.label(), values);
        }
        Some(view! { <LineChart data=chart class="h-96" /> })
    };
    view! {
        <div class="p-2 space-y-3">
            <h1 class="text-xl font-semibold text-base-content">"Patch Evolution"</h1>
            <p class="text-sm text-muted">
                "The costs of units, and of what they train, build, research and upgrade, over the patches of the balance data. A line per unit, item and measure: minerals, vespene, supply, time, whether it exists and its number of requirements."
            </p>
            <div class="flex flex-wrap items-center gap-2">
                <select
                    class=SELECT_CLASS
                    on:change=move |ev| {
                        let chosen = event_target_value(&ev);
                        if let Some(query) = saved.get().into_iter().find(|query| query.name == chosen) {
                            name.set(query.name);
                            series.set(query.series);
                        }
                    }
                >
                    <option value="">"Saved queries..."</option>
                    {move || saved.get().into_iter().map(|query| view! {
                        <option value=query.name.clone()>{query.name.clone()}</option>
                    }).collect_view()}
                </select>
                <input
                    class="input input-sm bg-base-200 text-base-content border border-base-content/20"
                    placeholder="Name of the query"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                />
                <button class="btn btn-sm btn-primary" on:click=save title="Saves the lines in the settings">"Save"</button>
                <button
                    class="btn btn-sm btn-ghost"
                    title="Removes the saved query with this name"
                    on:click=move |_| {
                        let removed = SavedEvolution { name: name.get(), series: vec![] };
                        spawn_local(async move {
                            if let Ok(all) = invoke_json::<Vec<SavedEvolution>>("save_evolution", &SaveArgs { saved: removed }).await {
                                saved.set(all);
                                status.set("Removed".to_string());
                            }
                        });
                    }
                >
                    "Remove"
                </button>
                <span class="text-sm text-faint">{move || status.get()}</span>
            </div>
            <div class="space-y-1">
                {move || (0..series.read().len()).map(|idx| view! { <SeriesEditor idx series units /> }).collect_view()}
            </div>
            <button
                class="btn btn-sm btn-accent"
                title="Adds a line"
                on:click=move |_| series.update(|series| {
                    let last = series.last().cloned().unwrap_or(EvolutionSeries {
                        unit: "Marine".to_string(),
                        item: EVOLUTION_UNIT_ITEM.to_string(),
                        metric: EvolutionMetric::Minerals,
                    });
                    series.push(EvolutionSeries { metric: EvolutionMetric::Vespene, ..last });
                })
            >
                <Icon icon=PLUS weight=IconWeight::Bold />
                "Add a line"
            </button>
            {chart}
            <AbilityEvolution />
        </div>
    }
}

/// The uses per game of the abilities with energy or a cooldown of a race over the patches of the
/// snapshot games, a line per ability.
#[component]
fn AbilityEvolution() -> impl IntoView {
    let usage = RwSignal::new(None::<Result<AbilityUsageTable, String>>);
    let race = RwSignal::new("Zerg".to_string());
    // The abilities shown, all when empty: a click shows only that one, others add to it, as in
    // Grafana.
    let shown = RwSignal::new(Vec::<&'static str>::new());
    spawn_local(async move {
        usage.set(Some(fetch_ability_usage().await));
    });
    let abilities = move || -> Vec<&'static TrackedAbility> {
        let race = race.get();
        usage.with(|usage| match usage {
            Some(Ok(usage)) => race_abilities(&race)
                .filter(|tracked| usage.usage.iter().any(|row| row.ability == tracked.ability))
                .collect(),
            _ => vec![],
        })
    };
    let is_shown =
        move |ability: &str| shown.with(|shown| shown.is_empty() || shown.contains(&ability));
    let toggle = move |ability: &'static str, count: usize| {
        shown.update(|shown| {
            if shown.is_empty() {
                shown.push(ability);
            } else if let Some(idx) = shown.iter().position(|other| *other == ability) {
                shown.remove(idx);
            } else {
                shown.push(ability);
            }
            if shown.len() == count {
                shown.clear();
            }
        })
    };
    let legend = move || {
        let all = abilities();
        let count = all.len();
        all.into_iter()
            .enumerate()
            .map(|(idx, tracked)| {
                let ability = tracked.ability;
                view! {
                    <button
                        type="button"
                        class="inline-flex items-center gap-1 rounded px-1 text-xs hover:bg-base-300"
                        class:opacity-40=move || !is_shown(ability)
                        aria-pressed=move || is_shown(ability).to_string()
                        title=format!("{} of the {}: click to show only it, click others to add them", tracked.label, tracked.unit)
                        on:click=move |_| toggle(ability, count)
                    >
                        <span class="inline-block w-2.5 h-2.5 rounded-full" style=format!("background: {}", series_color(idx))></span>
                        <span class="text-base-content">{tracked.label}</span>
                    </button>
                }
            })
            .collect_view()
    };
    let chart = move || {
        let usage = match usage.get()? {
            Ok(usage) => usage,
            Err(err) => {
                return Some(
                    view! {
                        <p class="text-xs text-muted">{format!("No snapshot to count the abilities: {err}")}</p>
                    }
                    .into_any(),
                )
            }
        };
        // The color of an ability is its place among the abilities of the race, whatever is shown.
        let all = abilities();
        let visible: Vec<(usize, &TrackedAbility)> = all
            .iter()
            .enumerate()
            .filter(|(_, tracked)| is_shown(tracked.ability))
            .map(|(idx, tracked)| (idx, *tracked))
            .collect();
        let mut chart = ChartData::new(usage.patches.iter().map(u32::to_string).collect())
            .with_owned_colors(
                visible
                    .iter()
                    .map(|(idx, _)| series_color(*idx).to_string())
                    .collect(),
            )
            .without_legend();
        for (_, tracked) in visible {
            let values = usage
                .patches
                .iter()
                .map(|patch| usage.get(*patch, tracked.ability).map(|row| row.per_game()))
                .collect();
            chart = chart.with_optional_series(tracked.label, values);
        }
        Some(view! { <LineChart data=chart class="h-[32rem]" /> }.into_any())
    };
    view! {
        <div class="space-y-1 pt-2">
            <h2 class="text-base font-semibold text-base-content">"Ability use per game"</h2>
            <p class="text-sm text-muted">
                "The abilities with energy or a cooldown commanded per game by the players of the race, in the games of the snapshot by patch. A patch without a point has no game in which the ability was used. Click an ability to show only it, then others to add them."
            </p>
            <crate::components::race_buttons::RaceButtons
                value=race
                on_pick=Callback::new(move |_| shown.set(vec![]))
            />
            <p class="flex flex-wrap items-center gap-x-1">{legend}</p>
            {chart}
        </div>
    }
}
