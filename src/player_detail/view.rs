//! Races, results over time and protocol versions, units and opponents of a player.

use super::compare::PlayerComparison;
use super::insights::PlayerInsightsView;
use crate::components::charts::GamesWinsChart;
use crate::components::links::{
    MapLink, PatchLink, PlayerLink, RaceLink, ReplayLink, ResultMarker,
};
use crate::components::loading::Loading;
use crate::components::name_select::NameSelect;
use crate::components::units::UnitsTable;
use crate::navigation::use_navigation;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

/// Units produced less than this are not considered for the most effective unit.
const MIN_PRODUCED_FOR_EFFECTIVENESS: usize = 10;

/// The rows shown in the units table.
const TOP_UNITS: usize = 15;

const HEADING_CLASS: &str = "text-base/7 font-semibold text-white";
const TABLE_CLASS: &str = "min-w-full text-xs text-gray-400";

fn win_rate(wins: usize, games: usize) -> String {
    if games == 0 {
        return "-".to_string();
    }
    format!("{:.0}%", wins as f64 * 100. / games as f64)
}

#[component]
pub fn PlayerDetailView() -> impl IntoView {
    let navigation = use_navigation();
    let detail = RwSignal::new(None::<PlayerDetail>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    // The opponent the matches are filtered by, empty for all.
    let opponent = RwSignal::new(String::new());
    // The player compared with, empty for none.
    let compare_with = RwSignal::new(String::new());
    Effect::new(move |_| {
        let player_name = navigation.player_name.get();
        if player_name.is_empty() {
            return;
        }
        loading.set(true);
        error.set(None);
        opponent.set(String::new());
        spawn_local(async move {
            match invoke_json::<PlayerDetail>(
                "query_player_detail",
                &PlayerDetailQuery { player_name },
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
            <div class="flex flex-wrap gap-4 items-end">
                <NameSelect
                    label="Player"
                    command="list_players"
                    value=navigation.player_name
                    on_select=Callback::new(move |name: String| navigation.open_player(&name))
                />
                <NameSelect
                    label="Compare with"
                    command="list_players"
                    value=compare_with
                    on_select=Callback::new(move |name: String| compare_with.set(name))
                    empty_label="Nobody"
                />
            </div>
            <Show when=move || navigation.player_name.get().is_empty()>
                <p class="text-sm">"Choose a player above or by clicking on its name in any view."</p>
            </Show>
            <Show when=move || loading.get()>
                <Loading message="Analysing every game of the player, the first time takes a few seconds..." />
            </Show>
            <ErrorMessage error />
            {move || detail.get().map(|detail| view! {
                <PlayerHeader detail=detail.clone() />
                {
                    let player = detail.clone();
                    move || {
                        let other = compare_with.get();
                        (!other.is_empty() && other != player.player_name).then(|| view! {
                            <PlayerComparison player=player.clone() opponent_name=other />
                        })
                    }
                }
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <h2 class=HEADING_CLASS>"Games and wins per month"</h2>
                        <GamesWinsChart data=detail.by_month.clone() />
                    </div>
                    <div>
                        <h2 class=HEADING_CLASS>"Games and wins per protocol version"</h2>
                        <GamesWinsChart data=detail.by_version.clone() />
                    </div>
                </div>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <h2 class=HEADING_CLASS>"Units"</h2>
                        <UnitsTable
                            units=detail.units.clone()
                            min_produced=MIN_PRODUCED_FOR_EFFECTIVENESS
                            max_rows=TOP_UNITS
                        />
                    </div>
                    <OpponentsTable opponents=detail.opponents.clone() opponent />
                </div>
                <PlayerInsightsView insights=detail.insights.clone() />
                <MatchesTable matches=detail.matches.clone() opponent />
            })}
        </div>
    }
}

#[component]
fn PlayerHeader(detail: PlayerDetail) -> impl IntoView {
    let races = detail
        .races
        .iter()
        .map(|race| {
            format!(
                "{} {} games ({})",
                race.label,
                race.games,
                win_rate(race.wins, race.games)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    view! {
        <div>
            <h1 class="text-xl font-semibold text-white">
                {clean_player_name_clan_escapes(&detail.player_name)}
            </h1>
            <p class="text-sm text-gray-400">
                {format!(
                    "{} games, {} wins, {} losses ({}) · {}",
                    detail.games,
                    detail.wins,
                    detail.games - detail.wins,
                    win_rate(detail.wins, detail.games),
                    races,
                )}
            </p>
        </div>
    }
}

#[component]
fn OpponentsTable(opponents: Vec<OpponentRecord>, opponent: RwSignal<String>) -> impl IntoView {
    view! {
        <div>
            <h2 class=HEADING_CLASS>"Opponents"</h2>
            <p class="text-xs text-gray-500">"Click on the record to filter the matches below."</p>
            <table class=TABLE_CLASS>
                <thead>
                    <tr class="text-left text-gray-300">
                        <th class="pr-2">"Opponent"</th>
                        <th class="pr-2">"Games"</th>
                        <th class="pr-2">"W - L"</th>
                        <th>"Win rate"</th>
                    </tr>
                </thead>
                <tbody>
                    {opponents
                        .into_iter()
                        .map(|record| {
                            let name = record.opponent.clone();
                            let selected = {
                                let name = name.clone();
                                move || opponent.get() == name
                            };
                            view! {
                                <tr class:bg-gray-800=selected>
                                    <td class="pr-2"><PlayerLink player_name=record.opponent.clone() /></td>
                                    <td class="pr-2">{record.games}</td>
                                    <td class="pr-2">
                                        <a
                                            href="#"
                                            class="hover:text-indigo-400"
                                            title="Show the matches against this opponent"
                                            on:click=move |ev: MouseEvent| {
                                                ev.prevent_default();
                                                if opponent.get() == name {
                                                    opponent.set(String::new());
                                                } else {
                                                    opponent.set(name.clone());
                                                }
                                            }
                                        >
                                            <span class="text-green-500">{record.wins}</span>
                                            " - "
                                            <span class="text-red-500">{record.games - record.wins}</span>
                                        </a>
                                    </td>
                                    <td>{win_rate(record.wins, record.games)}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn MatchesTable(matches: Vec<PlayerMatch>, opponent: RwSignal<String>) -> impl IntoView {
    let visible_matches = move || {
        let opponent = opponent.get();
        matches
            .iter()
            .filter(|game| opponent.is_empty() || game.opponent == opponent)
            .cloned()
            .collect::<Vec<_>>()
    };
    view! {
        <div>
            <h2 class=HEADING_CLASS>
                "Matches"
                {move || {
                    let opponent = opponent.get();
                    (!opponent.is_empty())
                        .then(|| format!(" against {}", clean_player_name_clan_escapes(&opponent)))
                }}
            </h2>
            <table class=TABLE_CLASS>
                <thead>
                    <tr class="text-left text-gray-300">
                        <th class="pr-2">"ID"</th>
                        <th class="pr-2">"Date"</th>
                        <th class="pr-2">"Protocol"</th>
                        <th class="pr-2">"Map"</th>
                        <th class="pr-2">"Race"</th>
                        <th>"Opponent"</th>
                    </tr>
                </thead>
                <tbody>
                    {move || visible_matches()
                        .into_iter()
                        .map(|game| view! {
                            <tr>
                                <td class="pr-2"><ReplayLink ext_fs_id=game.ext_fs_id /></td>
                                <td class="pr-2">{game.date}</td>
                                <td class="pr-2"><PatchLink version=game.protocol_version /></td>
                                <td class="pr-2 text-white"><MapLink map_title=game.map_title /></td>
                                <td class="pr-2">
                                    <ResultMarker result=if game.won { "Win" } else { "Loss" }.to_string() />
                                    <RaceLink race=game.race />
                                </td>
                                <td>
                                    <PlayerLink player_name=game.opponent />
                                    " ("
                                    <RaceLink race=game.opponent_race />
                                    ")"
                                </td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}
