//! A race in the games of the snapshot: its matchups, the maps that favour it, the units it makes
//! and the abilities it uses.

use crate::components::abilities::AbilityUsageRows;
use crate::components::charts::GamesWinsChart;
use crate::components::links::{MapLink, PlayerLink, UnitLink};
use crate::components::loading::Loading;
use crate::navigation::use_navigation;
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

const TABLE_CLASS: &str = "min-w-full text-xs text-gray-400";

/// The units shown, the most played first.
const UNITS_SHOWN: usize = 30;

fn win_percent(entry: &GamesWins) -> f64 {
    100. * entry.wins as f64 / entry.games.max(1) as f64
}

/// The win rate with the games, i.e. `54% of 120`, green above 50% and red below.
#[component]
fn WinRate(wins: usize, games: usize) -> impl IntoView {
    let percent = 100. * wins as f64 / games.max(1) as f64;
    let color = if games == 0 {
        "text-gray-500"
    } else if percent >= 52. {
        "text-green-400"
    } else if percent <= 48. {
        "text-red-400"
    } else {
        "text-gray-300"
    };
    view! {
        <span class=format!("font-mono {color}") title=format!("{wins} wins out of {games} games")>
            {format!("{percent:.0}% of {games}")}
        </span>
    }
}

fn section<V: IntoView + 'static>(
    title: &'static str,
    description: &'static str,
    content: V,
) -> impl IntoView {
    view! {
        <div class="rounded-md border border-white/15 bg-gray-800/40">
            <h2 class="px-2 py-1 text-base/7 font-semibold text-white border-b border-white/15 bg-gray-800/80 rounded-t-md">
                {title}
            </h2>
            <div class="p-2 space-y-1">
                <p class="text-xs text-gray-500">{description}</p>
                {content}
            </div>
        </div>
    }
}

#[component]
pub fn RaceDetailView() -> impl IntoView {
    let navigation = use_navigation();
    let detail = RwSignal::new(None::<RaceDetail>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    Effect::new(move |_| {
        let race = navigation.race_detail.get();
        loading.set(true);
        spawn_local(async move {
            match invoke_json::<RaceDetail>("query_race_detail", &RaceDetailQuery { race }).await {
                Ok(loaded) => {
                    error.set(None);
                    detail.set(Some(loaded));
                }
                Err(err) => {
                    detail.set(None);
                    error.set(Some(err));
                }
            }
            loading.set(false);
        });
    });
    view! {
        <div class="p-2 space-y-2 overflow-y-auto">
            <h1 class="text-xl font-semibold text-white">"Race Detail"</h1>
            <p class="text-xs text-gray-500">
                "The games of a race in the snapshot: its matchups, the maps it wins the most on, the units it makes and the abilities with energy or a cooldown it uses."
            </p>
            <div class="flex items-center gap-2">
                {RACE_NAMES
                    .iter()
                    .map(|race| view! {
                        <button
                            class=move || if navigation.race_detail.get() == *race { "btn btn-sm btn-primary" } else { "btn btn-sm btn-ghost" }
                            on:click=move |_| navigation.race_detail.set(race.to_string())
                        >
                            {*race}
                        </button>
                    })
                    .collect_view()}
                <a
                    href="#"
                    class="nav-link text-sm"
                    title="The units of the race that changed in each patch"
                    on:click=move |ev: leptos::ev::MouseEvent| {
                        ev.prevent_default();
                        navigation.open_race(&navigation.race_detail.get_untracked());
                    }
                >
                    "Patch changes of the race"
                </a>
            </div>
            <ErrorMessage error />
            <Show when=move || loading.get()>
                <Loading message="Reading the games of the race" />
            </Show>
            {move || detail.get().map(|detail| view! { <RaceDetailSections detail /> })}
        </div>
    }
}

#[component]
fn RaceDetailSections(detail: RaceDetail) -> impl IntoView {
    let letter = detail.race.chars().next().unwrap_or('?');
    let mirror = format!("{letter}v{letter}");
    let units: Vec<RaceUnitStats> = detail.units.iter().take(UNITS_SHOWN).cloned().collect();
    view! {
        <p class="text-sm text-gray-300">
            {format!("{} games of {} players.", detail.player_games, detail.race)}
        </p>
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-2">
            {section(
                "Matchups",
                "The 1v1 games against each race, the mirror is always 50%.",
                view! {
                    <table class=TABLE_CLASS>
                        <tbody>
                            {detail
                                .matchups
                                .iter()
                                .map(|matchup| view! {
                                    <tr>
                                        <td class="pr-3 text-white">{matchup.label.clone()}</td>
                                        <td><WinRate wins=matchup.wins games=matchup.games /></td>
                                    </tr>
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                },
            )}
            {section(
                "By patch",
                "The 1v1 games that are not mirrors, by the patch of the balance data closest to the game.",
                view! {
                    <table class=TABLE_CLASS>
                        <tbody>
                            {detail
                                .by_patch
                                .iter()
                                .map(|patch| view! {
                                    <tr>
                                        <td class="pr-3">{patch.label.clone()}</td>
                                        <td><WinRate wins=patch.wins games=patch.games /></td>
                                    </tr>
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                    <GamesWinsChart data=detail.by_patch.clone() />
                },
            )}
        </div>
        {section(
            "Maps",
            "The maps with at least 5 1v1 games that are not mirrors, the best win rate first. Few games make the win rates of a map vary a lot.",
            view! {
                <table class=TABLE_CLASS>
                    <thead>
                        <tr class="text-left text-gray-300">
                            <th class="pr-3">"Map"</th>
                            <th class="pr-3">"Overall"</th>
                            {detail
                                .matchups
                                .iter()
                                .filter(|matchup| matchup.label != mirror)
                                .map(|matchup| view! { <th class="pr-3">{matchup.label.clone()}</th> })
                                .collect_view()}
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-white/5">
                        {detail
                            .maps
                            .iter()
                            .map(|map| {
                                let cells = detail
                                    .matchups
                                    .iter()
                                    .filter(|matchup| matchup.label != mirror)
                                    .map(|matchup| {
                                        let on_map = map
                                            .matchups
                                            .iter()
                                            .find(|entry| entry.label == matchup.label)
                                            .cloned()
                                            .unwrap_or_default();
                                        view! { <td class="pr-3"><WinRate wins=on_map.wins games=on_map.games /></td> }
                                    })
                                    .collect_view();
                                view! {
                                    <tr>
                                        <td class="pr-3 py-1"><MapLink map_title=map.map_title.clone() /></td>
                                        <td class="pr-3 py-1"><WinRate wins=map.wins() games=map.games() /></td>
                                        {cells}
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            },
        )}
        {section(
            "Units",
            "The army units made, in how many games, how many per game they were made, their kills per unit of the units that cost resources and the win rate of the games they were made in.",
            view! {
                <table class=TABLE_CLASS>
                    <thead>
                        <tr class="text-left text-gray-300">
                            <th class="pr-3">"Unit"</th>
                            <th class="pr-3 text-right">"Games"</th>
                            <th class="pr-3 text-right">"Made per game"</th>
                            <th class="pr-3 text-right">"Kills per unit"</th>
                            <th class="pr-3">"Win rate"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-white/5">
                        {units
                            .into_iter()
                            .map(|unit| view! {
                                <tr>
                                    <td class="pr-3 py-1"><UnitLink unit_name=unit.unit.clone() /></td>
                                    <td class="pr-3 py-1 text-right font-mono">
                                        {format!("{:.0}%", 100. * unit.games as f64 / detail.player_games.max(1) as f64)}
                                    </td>
                                    <td class="pr-3 py-1 text-right font-mono text-white">{format!("{:.1}", unit.born_per_game())}</td>
                                    <td class="pr-3 py-1 text-right font-mono">{format!("{:.2}", unit.kills_per_unit())}</td>
                                    <td class="pr-3 py-1"><WinRate wins=unit.wins games=unit.games /></td>
                                </tr>
                            })
                            .collect_view()}
                    </tbody>
                </table>
            },
        )}
        {section(
            "Abilities",
            "The abilities with energy or a cooldown commanded in all the games of the race in the snapshot.",
            view! { <AbilityUsageRows rows=detail.abilities.clone() show_race=false /> },
        )}
        {section(
            "Players",
            "The players with the most games of the race.",
            view! {
                <table class=TABLE_CLASS>
                    <tbody>
                        {detail
                            .top_players
                            .iter()
                            .map(|player| view! {
                                <tr>
                                    <td class="pr-3"><PlayerLink player_name=player.label.clone() /></td>
                                    <td title=format!("{:.1}%", win_percent(player))><WinRate wins=player.wins games=player.games /></td>
                                </tr>
                            })
                            .collect_view()}
                    </tbody>
                </table>
            },
        )}
    }
}
