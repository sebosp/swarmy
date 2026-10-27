//! Patch Evolution: how the costs of units, and of what they train, build, research and upgrade,
//! change over the patches, as lines of a chart. The queries can be saved in the settings.

use crate::components::charts::{ChartData, LineChart};
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, PLUS, TRASH};
use swarmy_common::*;

/// The colors of the lines, assigned by the position of the series: tailwind indigo, emerald,
/// amber, rose, sky, fuchsia, lime, orange.
const SERIES_COLORS: [&str; 8] = [
    "#6366f1", "#10b981", "#f59e0b", "#f43f5e", "#0ea5e9", "#d946ef", "#84cc16", "#f97316",
];

const SELECT_CLASS: &str = "select select-sm bg-gray-800 text-gray-200 border border-white/20";

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
    view! {
        <div class="flex items-center gap-2">
            <span class="inline-block w-3 h-3 rounded-full" style=format!("background: {}", series_color(idx))></span>
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
                {move || units.get().into_iter().map(|unit| {
                    let name = unit.name.clone();
                    view! {
                        <option value=unit.name.clone() prop:selected=move || current().unit == name>
                            {format!("{} ({})", unit.name, unit.race)}
                        </option>
                    }
                }).collect_view()}
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
            <h1 class="text-xl font-semibold text-white">"Patch Evolution"</h1>
            <p class="text-sm text-gray-400">
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
                    class="input input-sm bg-gray-800 text-gray-200 border border-white/20"
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
                <span class="text-sm text-gray-500">{move || status.get()}</span>
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
        </div>
    }
}
