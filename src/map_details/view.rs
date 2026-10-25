//! Leptos view for the details of a map: matchups, top players, recent games and terrain.

use super::resources::{MapPicture, MapResourcesPanel};
use crate::components::links::{PlayerLink, ReplayLink, ResultMarker};
use crate::components::loading::Loading;
use crate::components::name_select::NameSelect;
use crate::map_stats::actions::trigger_swarmy_bevy_exec_on_caches;
use crate::navigation::use_navigation;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, BINOCULARS, CUBE};
use swarmy_common::*;

const HEADING_CLASS: &str = "text-base/7 font-semibold text-white";
const TABLE_CLASS: &str = "min-w-full text-xs text-gray-400";

fn win_rate(wins: usize, games: usize) -> String {
    if games == 0 {
        return "-".to_string();
    }
    format!("{:.0}%", wins as f64 * 100. / games as f64)
}

#[component]
pub fn MapDetailsView() -> impl IntoView {
    let navigation = use_navigation();
    let details = RwSignal::new(None::<MapDetails>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    Effect::new(move |_| {
        let map_title = navigation.map_title.get();
        if map_title.is_empty() {
            return;
        }
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke_json::<MapDetails>("query_map_details", &MapDetailsQuery { map_title })
                .await
            {
                Ok(response) => details.set(Some(response)),
                Err(err) => {
                    details.set(None);
                    error.set(Some(err));
                }
            }
            loading.set(false);
        });
    });
    view! {
        <div class="p-2 space-y-4">
            <NameSelect
                label="Map"
                command="list_maps"
                value=navigation.map_title
                on_select=Callback::new(move |title: String| navigation.open_map(&title))
            />
            <Show when=move || navigation.map_title.get().is_empty()>
                <p class="text-sm">"Choose a map above or by clicking on its title in the Map List or Replay List."</p>
            </Show>
            <Show when=move || loading.get()>
                <Loading message="Collecting the games of the map..." />
            </Show>
            <ErrorMessage error />
            {move || details.get().map(|details| view! { <MapDetailsContent details /> })}
        </div>
    }
}

#[component]
fn MapDetailsContent(details: MapDetails) -> impl IntoView {
    let navigation = use_navigation();
    let title = details.title.clone();
    let cache_handles = details.cache_handles.clone();
    view! {
        <div class="flex items-start justify-between">
            <div>
                <h1 class="text-xl font-semibold text-white">{details.title.clone()}</h1>
                <p class="text-sm text-gray-400">
                    {format!(
                        "{} games from {} to {} · {} on average",
                        details.num_games,
                        details.min_date,
                        details.max_date,
                        format_game_time(details.avg_duration_seconds),
                    )}
                </p>
            </div>
            <div class="flex gap-2">
                <button
                    class="btn btn-primary btn-sm"
                    title="Scout this map"
                    on:click=move |_ev: MouseEvent| navigation.scout_map(&title)
                >
                    <Icon icon=BINOCULARS weight=IconWeight::Light />
                    "Scout this map"
                </button>
                <button
                    class="btn btn-primary btn-sm"
                    title="Open in Swarmy-Bevy"
                    on:click=move |_ev: MouseEvent| trigger_swarmy_bevy_exec_on_caches(&cache_handles)
                >
                    <Icon icon=CUBE weight=IconWeight::Light />
                    "Open in Swarmy-Bevy"
                </button>
            </div>
        </div>
        <div class="grid grid-cols-2 gap-4">
            <div>
                <p class="text-xs text-gray-400 whitespace-pre-line">{details.description.clone()}</p>
                <MapPicture details=details.clone() />
                {details.resources.clone().map(|resources| view! { <MapResourcesPanel resources /> })}
            </div>
            <div class="space-y-4">
                <div>
                    <h2 class=HEADING_CLASS>"Race vs race"</h2>
                    <table class=TABLE_CLASS>
                        <thead>
                            <tr class="text-left text-gray-300">
                                <th class="pr-2">"Matchup"</th>
                                <th class="pr-2">"Games"</th>
                                <th title="The win rate of the first race of the matchup">"First race wins"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {details
                                .matchups
                                .iter()
                                .map(|matchup| {
                                    // Mirror matchups always have a winner and a loser.
                                    let mirror = is_mirror_matchup(&matchup.label);
                                    let map_title = details.title.clone();
                                    let label = matchup.label.clone();
                                    view! {
                                        <tr
                                            class="cursor-pointer hover:bg-gray-800"
                                            title="Open the games of this matchup on this map in the Replay List"
                                            on:click=move |_ev: MouseEvent| navigation.open_replays(&map_title, &label)
                                        >
                                            <td class="pr-2 text-white">
                                                <span class="nav-link">{matchup.label.clone()}</span>
                                            </td>
                                            <td class="pr-2">{matchup.games}</td>
                                            <td>{if mirror { "-".to_string() } else { win_rate(matchup.wins, matchup.games) }}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
                <div>
                    <h2 class=HEADING_CLASS>"Top players"</h2>
                    <table class=TABLE_CLASS>
                        <thead>
                            <tr class="text-left text-gray-300">
                                <th class="pr-2">"Player"</th>
                                <th class="pr-2">"Games"</th>
                                <th>"Win rate"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {details
                                .top_players
                                .iter()
                                .map(|player| view! {
                                    <tr>
                                        <td class="pr-2"><PlayerLink player_name=player.label.clone() /></td>
                                        <td class="pr-2">{player.games}</td>
                                        <td>{win_rate(player.wins, player.games)}</td>
                                    </tr>
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
        <div>
            <h2 class=HEADING_CLASS>"Recent games"</h2>
            <table class=TABLE_CLASS>
                <thead>
                    <tr class="text-left text-gray-300">
                        <th class="pr-2">"ID"</th>
                        <th class="pr-2">"Date"</th>
                        <th>"Players"</th>
                    </tr>
                </thead>
                <tbody>
                    {details
                        .recent_games
                        .into_iter()
                        .map(|game| view! {
                            <tr>
                                <td class="pr-2"><ReplayLink ext_fs_id=game.ext_fs_id /></td>
                                <td class="pr-2">{game.replay_date.to_string()}</td>
                                <td>
                                    {game
                                        .player_list
                                        .into_iter()
                                        .zip(game.player_results)
                                        .map(|(player_name, result)| view! {
                                            <span class="mr-3">
                                                <ResultMarker result />
                                                <PlayerLink player_name />
                                            </span>
                                        })
                                        .collect_view()}
                                </td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}
