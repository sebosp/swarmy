//! The statistics from the analysis of every game of a player.

use crate::components::charts::{ChartData, LineChart};
use crate::components::links::UnitLink;
use crate::components::section::Section;
use crate::*;
use leptos::prelude::*;
use swarmy_common::*;

const TABLE_CLASS: &str = "min-w-full text-xs text-gray-400";

/// The rows shown in the units and upgrades tables.
const TOP_ROWS: usize = 10;

fn percent(part: usize, total: usize) -> String {
    if total == 0 {
        return "-".to_string();
    }
    format!("{:.0}%", part as f64 * 100. / total as f64)
}

fn optional_time(seconds: Option<f32>) -> String {
    seconds
        .map(|seconds| format_game_time(seconds as u32))
        .unwrap_or_else(|| "-".to_string())
}

/// The value of the player, green when above the snapshot average for the same matchup.
fn compared(value: f32, snapshot: f32) -> impl IntoView {
    let better = value >= snapshot;
    view! {
        <span class:text-green-400=better class:text-red-400=!better>{format!("{value:.0}")}</span>
        <span class="text-gray-500">{format!(" / {snapshot:.0}")}</span>
    }
}

#[component]
pub fn PlayerInsightsView(insights: PlayerInsights) -> impl IntoView {
    view! {
        <p class="text-xs text-gray-500">
            {format!("From the analysis of {} games, each replay is read once and kept in memory.", insights.analyzed_games)}
        </p>
        <div class="grid grid-cols-3 gap-4">
            <Section title="Win rates" height="h-64">
                <WinRates title="Matchup" data=insights.by_matchup.clone() />
                <WinRates title="Game length" data=insights.by_length.iter().map(|entry| GamesWins { label: entry.label.chars().skip(3).collect(), ..entry.clone() }).collect() />
                <WinRates title="Map" data=insights.by_map.clone() />
            </Section>
            <Section title="Tendencies" height="h-64">
                <table class=TABLE_CLASS>
                    <tbody>
                        <tr>
                            <td class="pr-2">"Recent form"</td>
                            <td>
                                {insights
                                    .recent_form
                                    .iter()
                                    .map(|won| {
                                        if *won {
                                            view! { <span class="text-green-500">"▲"</span> }.into_any()
                                        } else {
                                            view! { <span class="text-red-500">"▼"</span> }.into_any()
                                        }
                                    })
                                    .collect_view()}
                            </td>
                        </tr>
                        <tr title="Games won after the army value was below 70% of the enemy one after 05:00">
                            <td class="pr-2">"Comebacks"</td>
                            <td>{format!("{} won out of {} games behind", insights.comeback_wins, insights.behind_games)}</td>
                        </tr>
                        <tr title="A production structure far from the start location before 04:00">
                            <td class="pr-2">"Proxies"</td>
                            <td>{format!("{} games ({})", insights.proxy_games, percent(insights.proxy_games, insights.analyzed_games))}</td>
                        </tr>
                        <tr title="Both sides lost at least two bases">
                            <td class="pr-2">"Base trades"</td>
                            <td>{format!("{} games", insights.base_trade_games)}</td>
                        </tr>
                        <tr>
                            <td class="pr-2">"First scout"</td>
                            <td>
                                {optional_time(insights.avg_first_scout_seconds)}
                                {format!(" on average, with {}", insights.first_scout_units.iter().take(3).map(|unit| format!("{} ({})", unit.unit, unit.count)).collect::<Vec<_>>().join(", "))}
                            </td>
                        </tr>
                        <tr>
                            <td class="pr-2">"First kill"</td>
                            <td>{optional_time(insights.avg_first_kill_seconds)}" on average"</td>
                        </tr>
                        <tr>
                            <td class="pr-2">"Bases at 10:00"</td>
                            <td>{format!("{:.1} on average", insights.avg_bases_at_10)}</td>
                        </tr>
                    </tbody>
                </table>
            </Section>
            <Section title="Macro, per game" height="h-64">
                <table class=TABLE_CLASS>
                    <tbody>
                        <tr><td class="pr-2">"Supply blocked"</td><td class="font-mono">{format_game_time(insights.avg_supply_blocked_seconds as u32)}</td></tr>
                        <tr><td class="pr-2">"Floating resources"</td><td class="font-mono">{format!("{} · {:.0} unspent", format_game_time(insights.avg_floating_seconds as u32), insights.avg_unspent)}</td></tr>
                        <tr><td class="pr-2">"Commands per minute"</td><td>{format!("{:.0}", insights.avg_commands_per_minute)}</td></tr>
                        <tr><td class="pr-2">"Injects · Chronos · MULEs"</td><td>{format!("{:.1} · {:.1} · {:.1}", insights.avg_injects, insights.avg_chrono_boosts, insights.avg_mules)}</td></tr>
                        <tr><td class="pr-2">"Resources lost · killed"</td><td>{format!("{} · {}", format_resources(insights.avg_resources_lost as u32), format_resources(insights.avg_resources_killed as u32))}</td></tr>
                        <tr><td class="pr-2">"Mining time lost"</td><td class="font-mono">{format_game_time(insights.avg_mining_seconds_lost as u32)}</td></tr>
                        {insights
                            .production_idle
                            .iter()
                            .map(|idle| view! {
                                <tr>
                                    <td class="pr-2">{format!("{} idle", idle.structure)}</td>
                                    <td class="font-mono">{format_game_time(idle.idle_seconds)}</td>
                                </tr>
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </Section>
        </div>

        <Section title="Economy against the snapshot, same matchup (player / snapshot average)" height="h-48">
            <table class=TABLE_CLASS>
                <thead class="sticky top-0 bg-gray-800">
                    <tr class="text-left text-gray-300">
                        <th class="pr-2">"Matchup"</th>
                        <th class="pr-2">"Time"</th>
                        <th class="pr-2">"Games"</th>
                        <th class="pr-2">"Workers"</th>
                        <th class="pr-2">"Supply"</th>
                        <th class="pr-2">"Income/min"</th>
                        <th>"Army value"</th>
                    </tr>
                </thead>
                <tbody class="font-mono">
                    {insights
                        .benchmarks
                        .iter()
                        .map(|benchmark| view! {
                            <tr>
                                <td class="pr-2 font-sans">{benchmark.matchup.clone()}</td>
                                <td class="pr-2">{format_game_time(benchmark.minute * 60)}</td>
                                <td class="pr-2">{format!("{} / {}", benchmark.games, benchmark.snapshot_games)}</td>
                                <td class="pr-2">{compared(benchmark.workers, benchmark.snapshot_workers)}</td>
                                <td class="pr-2">{compared(benchmark.supply, benchmark.snapshot_supply)}</td>
                                <td class="pr-2">{compared(benchmark.income, benchmark.snapshot_income)}</td>
                                <td>{compared(benchmark.army_value, benchmark.snapshot_army_value)}</td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
        </Section>

        <div class="grid grid-cols-2 gap-4">
            <Section title="Openings by matchup" height="h-56">
                <ChoiceTable shares=insights.openings_by_matchup.clone() units=false />
            </Section>
            <Section title="Openings by map" height="h-56">
                <ChoiceTable shares=insights.openings_by_map.clone() units=false />
            </Section>
            <Section title="First army unit by matchup" height="h-56">
                <ChoiceTable shares=insights.first_army_units.clone() units=true />
            </Section>
            <Section title="Most produced army unit by map" height="h-56">
                <ChoiceTable shares=insights.favourite_units_by_map.clone() units=true />
            </Section>
        </div>

        <div class="grid grid-cols-3 gap-4">
            <Section title="Army at 10:00, on average" height="h-56">
                <table class=TABLE_CLASS>
                    <tbody>
                        {insights
                            .army_at_10
                            .iter()
                            .take(TOP_ROWS)
                            .map(|unit| view! {
                                <tr>
                                    <td class="pr-2 text-white"><UnitLink unit_name=unit.unit.clone() /></td>
                                    <td>{format!("{:.1}", unit.average)}</td>
                                </tr>
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </Section>
            <Section title="Killed by" height="h-56">
                <CountsTable counts=insights.killed_by.clone() units=true />
            </Section>
            <Section title="First upgrade" height="h-56">
                <CountsTable counts=insights.upgrade_priorities.clone() units=false />
            </Section>
        </div>

        <RatingChart ratings=insights.ratings.clone() />
    }
}

#[component]
fn WinRates(title: &'static str, data: Vec<GamesWins>) -> impl IntoView {
    view! {
        <table class=format!("{TABLE_CLASS} mb-2")>
            <thead>
                <tr class="text-left text-gray-300">
                    <th class="pr-2">{title}</th>
                    <th class="pr-2">"Games"</th>
                    <th>"Win rate"</th>
                </tr>
            </thead>
            <tbody>
                {data
                    .into_iter()
                    .map(|entry| view! {
                        <tr>
                            <td class="pr-2 text-white">{entry.label}</td>
                            <td class="pr-2">{entry.games}</td>
                            <td>{percent(entry.wins, entry.games)}</td>
                        </tr>
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

/// i.e. "on Killswitch the player goes for SpawningPool → RoachWarren 80% of the games".
#[component]
fn ChoiceTable(shares: Vec<ChoiceShare>, units: bool) -> impl IntoView {
    view! {
        <table class=TABLE_CLASS>
            <thead class="sticky top-0 bg-gray-800">
                <tr class="text-left text-gray-300">
                    <th class="pr-2">"Context"</th>
                    <th class="pr-2">"Choice"</th>
                    <th class="pr-2">"Share"</th>
                    <th>"Win rate"</th>
                </tr>
            </thead>
            <tbody>
                {shares
                    .into_iter()
                    .map(|share| view! {
                        <tr>
                            <td class="pr-2">{share.context}</td>
                            <td class="pr-2 text-white">
                                {if units {
                                    view! { <UnitLink unit_name=share.choice /> }.into_any()
                                } else {
                                    share.choice.into_any()
                                }}
                            </td>
                            <td class="pr-2">{format!("{} ({}/{})", percent(share.games, share.context_games), share.games, share.context_games)}</td>
                            <td>{percent(share.wins, share.games)}</td>
                        </tr>
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

#[component]
fn CountsTable(counts: Vec<UnitCount>, units: bool) -> impl IntoView {
    view! {
        <table class=TABLE_CLASS>
            <tbody>
                {counts
                    .into_iter()
                    .take(TOP_ROWS)
                    .map(|count| view! {
                        <tr>
                            <td class="pr-2 text-white">
                                {if units {
                                    view! { <UnitLink unit_name=count.unit /> }.into_any()
                                } else {
                                    count.unit.into_any()
                                }}
                            </td>
                            <td>{count.count}</td>
                        </tr>
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

/// The ladder rating stored in the replays, pro games played on LAN usually have none.
#[component]
fn RatingChart(ratings: Vec<RatingPoint>) -> impl IntoView {
    if ratings.len() < 2 {
        return ().into_any();
    }
    let min = ratings
        .iter()
        .map(|point| point.rating)
        .min()
        .unwrap_or_default();
    let max = ratings
        .iter()
        .map(|point| point.rating)
        .max()
        .unwrap_or_default()
        .max(min + 1);
    let chart = ChartData::new(ratings.iter().map(|point| point.date.clone()).collect())
        .with_series(
            "MMR",
            ratings.iter().map(|point| point.rating as f64).collect(),
        );
    view! {
        <Section title="Rating" height="h-40">
            <p class="text-xs text-gray-400">
                {format!("From {} to {}, between {min} and {max}", ratings[0].date, ratings[ratings.len() - 1].date)}
            </p>
            <LineChart data=chart class="h-28" />
        </Section>
    }
    .into_any()
}
