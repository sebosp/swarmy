//! A chart of an ability, unit or race per day, month or year, with a line where each patch
//! starts: how the players adapt to the changes.

use crate::components::charts::{ChartData, LineChart};
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

/// With more patches than this the lines have no text, hovering a period tells its patch.
const LABELED_PATCHES: usize = 10;

const CONTROL_CLASS: &str = "select select-bordered select-xs";

/// What the chart shows of a period: its name, value and suffix.
struct Measure {
    name: &'static str,
    value: fn(&UsagePoint) -> f64,
    suffix: &'static str,
    decimals: u32,
}

fn measures(subject: &UsageSubject) -> Vec<Measure> {
    let percent_used = Measure {
        name: "Share of games",
        value: UsagePoint::used_percent,
        suffix: "%",
        decimals: 0,
    };
    match subject {
        UsageSubject::Ability(_) => vec![
            Measure {
                name: "Uses per game",
                value: UsagePoint::per_game,
                suffix: "",
                decimals: 1,
            },
            percent_used,
        ],
        UsageSubject::Unit(_) => vec![
            Measure {
                name: "Made per game",
                value: UsagePoint::per_game,
                suffix: "",
                decimals: 1,
            },
            percent_used,
            Measure {
                name: "Kills per unit",
                value: |point| point.kills as f64 / point.uses.max(1) as f64,
                suffix: "",
                decimals: 2,
            },
            Measure {
                name: "Win rate of the games with it",
                value: UsagePoint::win_percent,
                suffix: "%",
                decimals: 0,
            },
        ],
        UsageSubject::Race(_) => vec![
            Measure {
                name: "Win rate against other races",
                value: UsagePoint::win_percent,
                suffix: "%",
                decimals: 0,
            },
            Measure {
                name: "Games",
                value: |point| point.player_games as f64,
                suffix: "",
                decimals: 0,
            },
        ],
    }
}

/// The chart of a measure: a line over the periods, a marker where each patch starts.
fn usage_chart(usage: &UsageOverTime, measure: &Measure) -> ChartData {
    let patch_at = |period: &str| {
        usage
            .patches
            .iter()
            .filter(|start| start.period == period)
            .map(|start| start.patch.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let tooltips = usage
        .points
        .iter()
        .map(|point| {
            let patch = patch_at(&point.period);
            if point.player_games == 0 {
                return format!("{}<br/>No games", point.period);
            }
            format!(
                "{}<br/>{}: {:.*}{}<br/>{} games{}",
                point.period,
                measure.name,
                measure.decimals as usize,
                (measure.value)(point),
                measure.suffix,
                point.player_games,
                if patch.is_empty() {
                    String::new()
                } else {
                    format!("<br/>Patch {patch} starts")
                },
            )
        })
        .collect();
    // The patches that start in the same period share a line.
    let mut starts: Vec<(String, String)> = vec![];
    for start in usage.patches.iter().filter(|start| {
        usage
            .points
            .iter()
            .any(|point| point.period == start.period)
    }) {
        match starts
            .iter_mut()
            .find(|(period, _)| *period == start.period)
        {
            Some((_, text)) => text.push_str(&format!(", {}", start.patch)),
            None => starts.push((start.period.clone(), start.patch.to_string())),
        }
    }
    let labeled = starts.len() <= LABELED_PATCHES;
    let markers = starts
        .into_iter()
        .map(|(period, text)| (period, if labeled { text } else { String::new() }))
        .collect();
    ChartData::new(
        usage
            .points
            .iter()
            .map(|point| point.period.clone())
            .collect(),
    )
    .with_series(
        measure.name,
        usage
            .points
            .iter()
            .map(|point| (measure.value)(point))
            .collect(),
    )
    .with_tooltips(tooltips)
    .with_markers(markers)
    .with_value_suffix(measure.suffix)
    .with_y_decimals(measure.decimals)
    .with_curve("straight")
}

/// The use of an ability or unit, or the games of a race, over the periods of the snapshot. The
/// measure and the period are chosen beside it, on one line.
#[component]
pub fn UsageOverTimeChart(subject: UsageSubject) -> impl IntoView {
    let grain = RwSignal::new(TimeGrain::Month);
    let measure = RwSignal::new(0usize);
    let usage = RwSignal::new(None::<Result<UsageOverTime, String>>);
    let measure_names: Vec<&'static str> = measures(&subject).iter().map(|m| m.name).collect();
    let fetched = subject.clone();
    Effect::new(move |_| {
        let query = UsageOverTimeQuery {
            subject: fetched.clone(),
            grain: grain.get(),
        };
        usage.set(None);
        spawn_local(async move {
            #[derive(serde::Serialize)]
            struct Args {
                query: UsageOverTimeQuery,
            }
            usage.set(Some(
                invoke_json::<UsageOverTime>("query_usage_over_time", &Args { query }).await,
            ));
        });
    });
    let chart_subject = subject.clone();
    view! {
        <div class="flex flex-col gap-1">
            <div class="flex items-center gap-2 text-xs text-muted">
                <label class="flex items-center gap-1">
                    "Show"
                    <select
                        class=CONTROL_CLASS
                        aria-label="What the chart shows"
                        on:change=move |ev| {
                            measure.set(event_target_value(&ev).parse().unwrap_or_default());
                        }
                    >
                        {measure_names
                            .into_iter()
                            .enumerate()
                            .map(|(idx, name)| view! { <option value=idx.to_string()>{name}</option> })
                            .collect_view()}
                    </select>
                </label>
                <label class="flex items-center gap-1">
                    "Per"
                    <select
                        class=CONTROL_CLASS
                        aria-label="The period of each point"
                        on:change=move |ev| {
                            grain.set(match event_target_value(&ev).as_str() {
                                "day" => TimeGrain::Day,
                                "year" => TimeGrain::Year,
                                _ => TimeGrain::Month,
                            });
                        }
                    >
                        <option value="month">"Month"</option>
                        <option value="day">"Day"</option>
                        <option value="year">"Year"</option>
                    </select>
                </label>
            </div>
            <p class="text-xs text-faint">"The dashed lines are the first games of each patch."</p>
            {move || match usage.get() {
                None => view! { <p class="text-xs text-muted">"Loading the periods..."</p> }.into_any(),
                Some(Err(err)) => view! { <p class="text-xs text-muted">{err}</p> }.into_any(),
                Some(Ok(usage)) if usage.points.len() < 2 => view! {
                    <p class="text-xs text-muted">"The games of the snapshot are of a single period, try a shorter one."</p>
                }
                .into_any(),
                Some(Ok(usage)) => {
                    let measures = measures(&chart_subject);
                    let chart = usage_chart(&usage, &measures[measure.get().min(measures.len() - 1)]);
                    view! { <LineChart data=chart /> }.into_any()
                }
            }}
        </div>
    }
}

/// The ability of the rows chosen with a select, then its chart.
#[component]
pub fn AbilitiesOverTime(rows: Vec<AbilityPatchUsage>) -> impl IntoView {
    let choices: Vec<(String, String)> = rows
        .iter()
        .filter_map(|row| {
            row.tracked()
                .map(|tracked| (tracked.ability.to_string(), tracked.label.to_string()))
        })
        .collect();
    let chosen = RwSignal::new(choices.first().map(|(ability, _)| ability.clone()));
    view! {
        <div class="flex flex-col gap-1 pt-2">
            <label class="flex items-center gap-1 text-xs text-muted">
                "Over time"
                <select
                    class=CONTROL_CLASS
                    aria-label="The ability of the chart"
                    on:change=move |ev| chosen.set(Some(event_target_value(&ev)))
                >
                    {choices
                        .into_iter()
                        .map(|(ability, label)| view! { <option value=ability>{label}</option> })
                        .collect_view()}
                </select>
            </label>
            {move || {
                chosen.get().map(|ability| view! {
                    <UsageOverTimeChart subject=UsageSubject::Ability(ability) />
                })
            }}
        </div>
    }
}
