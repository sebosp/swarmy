//! The progress of the main player of Config over their last games, with goals: the workers at
//! 06:00, the income at 08:00, the seconds supply blocked and the army at 10:00.

use crate::components::charts::{ChartData, LineChart};
use crate::components::links::{MapLink, ReplayLink, ResultMarker};
use crate::components::loading::Loading;
use crate::navigation::use_navigation;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::BTreeMap;
use swarmy_common::*;

const TABLE_CLASS: &str = "min-w-full text-xs text-gray-400";

/// The games averaged by the rolling average of the charts.
const ROLLING_GAMES: usize = 10;

/// The recent games compared to the ones before them.
const RECENT_GAMES: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Opponents {
    All,
    Humans,
    Computers,
}

impl Opponents {
    fn keeps(&self, game: &ProgressGame) -> bool {
        match self {
            Self::All => true,
            Self::Humans => !game.vs_computer,
            Self::Computers => game.vs_computer,
        }
    }
}

fn win_rate(games: &[&ProgressGame]) -> String {
    if games.is_empty() {
        return "-".to_string();
    }
    let wins = games.iter().filter(|game| game.result == "Win").count();
    format!(
        "{:.0}% of {}",
        100. * wins as f64 / games.len() as f64,
        games.len()
    )
}

/// The average of the metric in the games, the games without it are left out.
fn average(games: &[&ProgressGame], key: &str) -> Option<f64> {
    let values: Vec<f64> = games
        .iter()
        .filter_map(|game| game.values.get(key).copied())
        .collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

#[component]
pub fn ProgressView() -> impl IntoView {
    let navigation = use_navigation();
    let report = RwSignal::new(None::<ProgressReport>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(true);
    let opponents = RwSignal::new(Opponents::All);
    let goals = RwSignal::new(BTreeMap::<String, f64>::new());
    let saved = RwSignal::new(None::<String>);
    spawn_local(async move {
        #[derive(serde::Serialize)]
        struct NoArgs {}
        match invoke_json::<ProgressReport>("query_progress", &NoArgs {}).await {
            Ok(loaded) => {
                goals.set(loaded.goals.clone());
                report.set(Some(loaded));
            }
            Err(err) => error.set(Some(err)),
        }
        loading.set(false);
    });
    let save_goals = move |_: MouseEvent| {
        #[derive(serde::Serialize)]
        struct Args {
            goals: BTreeMap<String, f64>,
        }
        let goals = goals.get_untracked();
        spawn_local(async move {
            match invoke_response("set_progress_goals", &Args { goals }).await {
                Ok(_) => saved.set(Some("Goals saved".to_string())),
                Err(err) => saved.set(Some(err)),
            }
        });
    };
    view! {
        <div class="p-2 space-y-3">
            <h1 class="text-xl font-semibold text-white">"Progress"</h1>
            <Show when=move || loading.get()>
                <Loading message="Reading your games..." />
            </Show>
            <ErrorMessage error />
            {move || {
                report
                    .get()
                    .map(|report| {
                        if report.player.is_empty() {
                            return view! {
                                <p class="text-sm text-gray-300">
                                    "Set your name in "
                                    <a
                                        href="#"
                                        class="nav-link"
                                        on:click=move |ev: MouseEvent| {
                                            ev.prevent_default();
                                            navigation.active_page.set("Config".to_string());
                                        }
                                    >
                                        "Config"
                                    </a>
                                    " (Main Player Name) to follow your progress over your games."
                                </p>
                            }
                                .into_any();
                        }
                        let player = report.player.clone();
                        let all_games = report.games.clone();
                        view! {
                            <p class="text-xs text-gray-400">
                                {format!(
                                    "The last {} games of {player} in the snapshot, the oldest on the left. Set a goal for each number and work on one at a time.",
                                    all_games.len(),
                                )}
                            </p>
                            <div class="flex items-center gap-2 text-xs text-gray-300">
                                "Opponents"
                                <select
                                    class="select select-xs"
                                    on:change=move |ev| {
                                        opponents
                                            .set(
                                                match event_target_value(&ev).as_str() {
                                                    "humans" => Opponents::Humans,
                                                    "computers" => Opponents::Computers,
                                                    _ => Opponents::All,
                                                },
                                            )
                                    }
                                >
                                    <option value="all">"All"</option>
                                    <option value="humans">"Humans"</option>
                                    <option value="computers">"Computers"</option>
                                </select>
                                <button class="btn btn-primary btn-xs" on:click=save_goals>
                                    "Save goals"
                                </button> <span class="text-gray-500">{move || saved.get()}</span>
                            </div>
                            {move || {
                                let filter = opponents.get();
                                let games: Vec<&ProgressGame> = all_games
                                    .iter()
                                    .filter(|game| filter.keeps(game))
                                    .collect();
                                let split = games.len().saturating_sub(RECENT_GAMES);
                                let (before, recent) = games.split_at(split);
                                let before = &before[before.len().saturating_sub(RECENT_GAMES)..];
                                view! {
                                    <table class=TABLE_CLASS>
                                        <thead>
                                            <tr class="text-left text-gray-300">
                                                <th class="pr-2"></th>
                                                <th class="pr-2">"Goal"</th>
                                                <th class="pr-2">
                                                    {format!("The {RECENT_GAMES} games before")}
                                                </th>
                                                <th class="pr-2">
                                                    {format!("The last {RECENT_GAMES} games")}
                                                </th>
                                            </tr>
                                        </thead>
                                        <tbody class="font-mono">
                                            <tr>
                                                <td class="pr-2 font-sans">"Wins"</td>
                                                <td></td>
                                                <td class="pr-2">{win_rate(before)}</td>
                                                <td class="pr-2">{win_rate(recent)}</td>
                                            </tr>
                                            {PROGRESS_METRICS
                                                .iter()
                                                .map(|metric| {
                                                    let fmt = |value: Option<f64>| {
                                                        value
                                                            .map(|value| format!("{value:.0}"))
                                                            .unwrap_or_else(|| "-".to_string())
                                                    };
                                                    let key = metric.key;
                                                    view! {
                                                        <tr title=metric.description>
                                                            <td class="pr-2 font-sans">{metric.label}</td>
                                                            <td class="pr-2">
                                                                <input
                                                                    type="number"
                                                                    class="input input-xs w-20"
                                                                    prop:value=move || {
                                                                        goals
                                                                            .get()
                                                                            .get(key)
                                                                            .map(|goal| goal.to_string())
                                                                            .unwrap_or_default()
                                                                    }
                                                                    on:change=move |ev| {
                                                                        let value = event_target_value(&ev)
                                                                            .parse::<f64>()
                                                                            .unwrap_or(0.);
                                                                        goals
                                                                            .update(|goals| {
                                                                                goals.insert(key.to_string(), value);
                                                                            });
                                                                    }
                                                                />
                                                            </td>
                                                            <td class="pr-2">{fmt(average(before, key))}</td>
                                                            <td class="pr-2">{fmt(average(recent, key))}</td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        </tbody>
                                    </table>
                                    <div class="grid grid-cols-1 lg:grid-cols-2 gap-2">
                                        {PROGRESS_METRICS
                                            .iter()
                                            .map(|metric| {
                                                view! {
                                                    <MetricChart
                                                        metric=*metric
                                                        games=games.iter().map(|game| (*game).clone()).collect()
                                                        goal=goals.get().get(metric.key).copied()
                                                    />
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                    <GamesTable games=games
                                        .iter()
                                        .rev()
                                        .map(|game| (*game).clone())
                                        .collect() />
                                }
                            }}
                        }
                            .into_any()
                    })
            }}
        </div>
    }
}

/// A metric of every game, its rolling average and the goal.
#[component]
fn MetricChart(
    metric: ProgressMetric,
    games: Vec<ProgressGame>,
    goal: Option<f64>,
) -> impl IntoView {
    let values: Vec<Option<f64>> = games
        .iter()
        .map(|game| game.values.get(metric.key).copied())
        .collect();
    let rolling: Vec<Option<f64>> = (0..values.len())
        .map(|idx| {
            let window: Vec<f64> = values[idx.saturating_sub(ROLLING_GAMES - 1)..=idx]
                .iter()
                .flatten()
                .copied()
                .collect();
            (!window.is_empty()).then(|| window.iter().sum::<f64>() / window.len() as f64)
        })
        .collect();
    let tooltips = games
        .iter()
        .zip(&values)
        .map(|(game, value)| {
            format!(
                "{} {} {}: {}",
                game.date,
                game.map_title,
                game.result,
                value
                    .map(|value| format!("{value:.0}"))
                    .unwrap_or_else(|| "-".to_string())
            )
        })
        .collect();
    let mut data = ChartData::new(games.iter().map(|game| game.date.clone()).collect())
        .with_optional_series(metric.label, values)
        .with_optional_series(&format!("Average of {ROLLING_GAMES} games"), rolling)
        .with_tooltips(tooltips)
        .with_curve("straight");
    if let Some(goal) = goal {
        data = data.with_optional_series("Goal", vec![Some(goal); games.len()]);
    }
    view! {
        <div class="rounded-md border border-white/15 bg-gray-800/40 p-2">
            <h3 class="text-sm font-semibold text-white" title=metric.description>
                {metric.label}
                {metric.lower_is_better.then_some(" (lower is better)")}
            </h3>
            <LineChart data />
        </div>
    }
}

#[component]
fn GamesTable(games: Vec<ProgressGame>) -> impl IntoView {
    view! {
        <div class="rounded-md border border-white/15 bg-gray-800/40 scroll-section max-h-96 p-2">
            <table class=TABLE_CLASS>
                <thead class="sticky top-0 bg-gray-800">
                    <tr class="text-left text-gray-300">
                        <th class="pr-2">"Date"</th>
                        <th class="pr-2">"Map"</th>
                        <th class="pr-2">"Opponents"</th>
                        {PROGRESS_METRICS
                            .iter()
                            .map(|metric| view! { <th class="pr-2">{metric.label}</th> })
                            .collect_view()}
                    </tr>
                </thead>
                <tbody>
                    {games
                        .into_iter()
                        .map(|game| {
                            let opponents = game
                                .opponents
                                .iter()
                                .map(|name| clean_player_name_clan_escapes(name))
                                .collect::<Vec<_>>()
                                .join(", ");
                            view! {
                                <tr>
                                    <td class="pr-2 font-mono">
                                        <ResultMarker result=game.result.clone() />
                                        <ReplayLink ext_fs_id=game.ext_fs_id />
                                        {format!(" {}", game.date)}
                                    </td>
                                    <td class="pr-2">
                                        <MapLink map_title=game.map_title.clone() />
                                    </td>
                                    <td class="pr-2">{opponents}</td>
                                    {PROGRESS_METRICS
                                        .iter()
                                        .map(|metric| {
                                            view! {
                                                <td class="pr-2 font-mono">
                                                    {game
                                                        .values
                                                        .get(metric.key)
                                                        .map(|value| format!("{value:.0}"))
                                                        .unwrap_or_else(|| "-".to_string())}
                                                </td>
                                            }
                                        })
                                        .collect_view()}
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}
