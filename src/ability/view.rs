//! One ability in the games of the snapshot: who uses it the most, its use over time, by patch
//! and by matchup, the games with the most uses, and when it is first used.

use crate::components::abilities::group_digits;
use crate::components::charts::{BarChart, ChartData, LineChart, CHART_COLORS};
use crate::components::links::{MapLink, PlayerLink, RaceLink, ReplayLink, UnitLink};
use crate::components::loading::Loading;
use crate::navigation::use_navigation;
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

const HEADING_CLASS: &str = "text-base/7 font-semibold text-base-content";
const TABLE_CLASS: &str = "min-w-full text-xs text-muted";

/// A number of the data, emphasized against the words around it.
fn figure(value: impl std::fmt::Display) -> impl IntoView {
    view! { <span class="font-medium text-base-content tabular-nums">{value.to_string()}</span> }
}

/// The game time of a first use, `-` when it was not used.
fn first_use(seconds: Option<u32>) -> String {
    seconds
        .map(format_game_time)
        .unwrap_or_else(|| "-".to_string())
}

#[component]
pub fn AbilityView() -> impl IntoView {
    let navigation = use_navigation();
    let detail = RwSignal::new(None::<AbilityDetail>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    Effect::new(move |_| {
        let ability = navigation.ability.get();
        if ability.is_empty() {
            return;
        }
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            #[derive(serde::Serialize)]
            struct Args {
                query: AbilityDetailQuery,
            }
            match invoke_json::<AbilityDetail>(
                "query_ability_detail",
                &Args {
                    query: AbilityDetailQuery { ability },
                },
            )
            .await
            {
                Ok(response) => detail.set(Some(response)),
                Err(err) => {
                    detail.set(None);
                    error.set(Some(err));
                }
            }
            loading.set(false);
        });
    });
    view! {
        <div class="p-2 space-y-4">
            <AbilitySelect />
            <Show when=move || navigation.ability.get().is_empty()>
                <p class="text-sm">
                    "Choose an ability above, or click on one in Replay Detail, Race Detail or the Patch Overview."
                </p>
            </Show>
            <Show when=move || loading.get()>
                <Loading message="Counting the uses in the games of the snapshot..." />
            </Show>
            <ErrorMessage error />
            <div class=move || if loading.get() { "opacity-40 pointer-events-none" } else { "" }>
                {move || detail.get().map(|detail| view! { <AbilityDetailContent detail /> })}
            </div>
        </div>
    }
}

/// The tracked abilities, by race.
#[component]
fn AbilitySelect() -> impl IntoView {
    let navigation = use_navigation();
    view! {
        <label class="flex items-center gap-2 text-sm text-base-content max-w-md">
            "Ability"
            <select
                class=SELECT_CLASS
                prop:value=move || navigation.ability.get()
                on:change=move |ev| navigation.open_ability(&event_target_value(&ev))
            >
                <option value="" disabled=true>"Choose an ability"</option>
                {["Terran", "Zerg", "Protoss"]
                    .into_iter()
                    .map(|race| {
                        view! {
                            <optgroup label=race>
                                {TRACKED_ABILITIES
                                    .iter()
                                    .filter(|tracked| tracked.race == race)
                                    .map(|tracked| {
                                        view! {
                                            <option value=tracked.ability>
                                                {format!("{} ({})", tracked.label, tracked.unit)}
                                            </option>
                                        }
                                    })
                                    .collect_view()}
                            </optgroup>
                        }
                    })
                    .collect_view()}
            </select>
        </label>
    }
}

/// The uses per game of each group as a chart, the tooltip with the share of the games it was
/// used in and the first use.
fn usage_chart_data(rows: &[AbilityGroupUsage]) -> ChartData {
    ChartData::new(rows.iter().map(|row| row.label.clone()).collect())
        .with_series(
            "Uses per game",
            rows.iter()
                .map(|row| (row.per_game() * 100.).round() / 100.)
                .collect(),
        )
        .with_colors(&[CHART_COLORS[0]])
        .with_tooltips(
            rows.iter()
                .map(|row| {
                    format!(
                        "{}: {:.2} uses per game, used in {:.0}% of {} games, first use at {} on average",
                        row.label,
                        row.per_game(),
                        row.used_percent(),
                        group_digits(row.player_games as u64),
                        first_use(row.avg_first_seconds),
                    )
                })
                .collect(),
        )
}

#[component]
fn AbilityDetailContent(detail: AbilityDetail) -> impl IntoView {
    let Some(tracked) = tracked_ability(&detail.ability) else {
        return view! { <p class="text-sm">{format!("{} is not a tracked ability", detail.ability)}</p> }
            .into_any();
    };
    let totals = detail.totals.clone();
    let fastest = detail.fastest.clone();
    view! {
        <div class="space-y-1">
            <h1 class="text-xl font-semibold text-base-content">{tracked.label}</h1>
            <p class="text-sm text-muted">
                {format!("{} of the ", tracked.cost.label())}
                <UnitLink unit_name=tracked.unit.to_string() />
                ", "
                <RaceLink race=tracked.race.to_string() />
                ". Used in "
                {figure(format!("{:.0}%", totals.used_percent()))}
                " of "
                {figure(group_digits(totals.player_games as u64))}
                {format!(" {} player games, ", tracked.race)}
                {figure(format!("{:.2}", totals.per_game()))}
                " uses per game. First used at "
                {figure(first_use(totals.avg_first_seconds))}
                " on average"
                {fastest
                    .map(|fastest| {
                        view! {
                            ", the fastest at "
                            {figure(first_use(fastest.first_seconds))}
                            " by "
                            <PlayerLink player_name=fastest.player_name.clone() />
                            " on "
                            <MapLink map_title=fastest.map_title.clone() />
                            " (replay "
                            <ReplayLink
                                ext_fs_id=fastest.ext_fs_id
                                map_title=fastest.map_title.clone()
                                players=vec![fastest.player_name.clone()]
                            />
                            ")"
                        }
                    })}
                "."
            </p>
        </div>
        <div class="grid grid-cols-2 gap-6">
            <div>
                <h2 class=HEADING_CLASS>"Over time"</h2>
                <p class="text-xs text-faint">"Uses per game of the race, by month of the games."</p>
                {if detail.by_month.len() > 1 {
                    view! { <LineChart data=usage_chart_data(&detail.by_month) /> }.into_any()
                } else {
                    view! { <p class="text-xs text-muted">"The games of the snapshot are of a single month."</p> }
                        .into_any()
                }}
            </div>
            <div>
                <h2 class=HEADING_CLASS>"By patch"</h2>
                <p class="text-xs text-faint">"Uses per game of the race, by balance data version."</p>
                <BarChart data=usage_chart_data(&detail.by_patch) />
            </div>
        </div>
        <div class="grid grid-cols-2 gap-6">
            <div>
                <h2 class=HEADING_CLASS>"By matchup"</h2>
                <UsageTable title="Matchup" rows=detail.by_matchup.clone() />
            </div>
            <div>
                <h2 class=HEADING_CLASS>"Top players"</h2>
                <p class="text-xs text-faint">
                    {format!("The most uses per game, with at least {ABILITY_RANK_MIN_GAMES} games of the race.")}
                </p>
                <table class=TABLE_CLASS>
                    <thead>
                        <tr class="text-left text-muted">
                            <th class="pr-3">"Player"</th>
                            <th class="pr-3 num">"Games"</th>
                            <th class="pr-3 num">"Uses"</th>
                            <th class="num" aria-sort="descending">"Per game"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {detail
                            .top_players
                            .iter()
                            .map(|player| {
                                view! {
                                    <tr>
                                        <td class="pr-3"><PlayerLink player_name=player.player_name.clone() /></td>
                                        <td class="pr-3 num">{player.games}</td>
                                        <td class="pr-3 num">{group_digits(player.uses as u64)}</td>
                                        <td class="num text-base-content">{format!("{:.1}", player.per_game())}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
        <div>
            <h2 class=HEADING_CLASS>"Top games"</h2>
            <p class="text-xs text-faint">"The games with the most uses by one player."</p>
            <table class=TABLE_CLASS>
                <thead>
                    <tr class="text-left text-muted">
                        <th class="pr-3">"Replay"</th>
                        <th class="pr-3">"Player"</th>
                        <th class="pr-3">"Matchup"</th>
                        <th class="pr-3">"Map"</th>
                        <th class="pr-3">"Date"</th>
                        <th class="pr-3 num" aria-sort="descending">"Uses"</th>
                        <th class="num">"First use"</th>
                    </tr>
                </thead>
                <tbody>
                    {detail
                        .top_games
                        .iter()
                        .map(|game| {
                            view! {
                                <tr>
                                    <td class="pr-3">
                                        <ReplayLink
                                            ext_fs_id=game.ext_fs_id
                                            map_title=game.map_title.clone()
                                            players=vec![game.player_name.clone()]
                                        />
                                    </td>
                                    <td class="pr-3"><PlayerLink player_name=game.player_name.clone() /></td>
                                    <td class="pr-3">{game.matchup.clone()}</td>
                                    <td class="pr-3"><MapLink map_title=game.map_title.clone() /></td>
                                    <td class="pr-3 tabular-nums">{game.date.clone()}</td>
                                    <td class="pr-3 num text-base-content">{game.uses}</td>
                                    <td class="num">{first_use(game.first_seconds)}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_any()
}

/// The use of the ability in groups of games, i.e. by matchup.
#[component]
fn UsageTable(title: &'static str, rows: Vec<AbilityGroupUsage>) -> impl IntoView {
    view! {
        <table class=TABLE_CLASS>
            <thead>
                <tr class="text-left text-muted">
                    <th class="pr-3">{title}</th>
                    <th class="pr-3 num" aria-sort="descending">"Games"</th>
                    <th class="pr-3 num" title="The games in which it was used at least once">"Used in"</th>
                    <th class="pr-3 num">"Per game"</th>
                    <th class="num" title="The average game time of the first use, in the games it was used">"First use"</th>
                </tr>
            </thead>
            <tbody>
                {rows
                    .into_iter()
                    .map(|row| {
                        view! {
                            <tr>
                                <td class="pr-3 text-base-content">{row.label.clone()}</td>
                                <td class="pr-3 num">{group_digits(row.player_games as u64)}</td>
                                <td class="pr-3 num">{format!("{:.0}%", row.used_percent())}</td>
                                <td class="pr-3 num text-base-content">{format!("{:.2}", row.per_game())}</td>
                                <td class="num">{first_use(row.avg_first_seconds)}</td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}
