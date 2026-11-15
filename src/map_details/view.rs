//! Leptos view for the details of a map: matchups, top players, recent games and terrain.

use super::resources::MapResourcesPanel;
use crate::components::brand_icon::{BrandIcon, native_title};
use crate::components::debug::Debuggable;
use crate::components::links::{PlayerLink, ReplayLink, ResultMarker};
use crate::components::loading::Loading;
use crate::components::map_cell::MapHighlightContext;
use crate::components::map_picture::MapPicture;
use crate::components::name_select::NameSelect;
use crate::navigation::use_navigation;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

const HEADING_CLASS: &str = "text-base/7 font-semibold text-base-content";
const TABLE_CLASS: &str = "min-w-full text-xs text-muted";

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
    // Without a map chosen, the most played one of the snapshot.
    if navigation.map_title.get_untracked().is_empty() {
        spawn_local(async move {
            if let Ok(maps) = invoke_json::<Vec<GamesWins>>("list_maps", &()).await {
                if let Some(top) = maps.into_iter().max_by_key(|map| map.games) {
                    if navigation.map_title.get_untracked().is_empty() {
                        navigation.map_title.set(top.label);
                    }
                }
            }
        });
    }
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
                <Loading message="Opening the most played map..." />
            </Show>
            <Show when=move || loading.get()>
                <Loading message="Collecting the games of the map..." />
            </Show>
            <ErrorMessage error />
            {move || details.get().map(|details| view! { <MapDetailsContent details /> })}
        </div>
    }
}

/// A number of the data, emphasized against the words around it.
fn figure(value: impl std::fmt::Display) -> impl IntoView {
    view! { <span class="font-medium text-base-content tabular-nums">{value.to_string()}</span> }
}

#[component]
fn MapDetailsContent(details: MapDetails) -> impl IntoView {
    let navigation = use_navigation();
    let cache_handles = details.cache_handles.clone();
    // The spot hovered in the tables, highlighted on the map.
    provide_context(MapHighlightContext::new());
    let bevy_status = RwSignal::new(String::new());
    let open_in_bevy = move |_ev: MouseEvent| {
        #[derive(serde::Serialize)]
        struct Args {
            cache_ids: String,
        }
        let args = Args {
            cache_ids: cache_handles.clone(),
        };
        bevy_status.set("Starting swarmy-bevy...".to_string());
        spawn_local(async move {
            match invoke_response("exec_swarmy_bevy_map_caches", &args).await {
                Ok(response) => bevy_status.set(response.message),
                Err(err) => bevy_status.set(err),
            }
        });
    };
    // The mirror matchups always have a winner and a loser, they are left out; the most played
    // first.
    let mut matchups: Vec<_> = details
        .matchups
        .iter()
        .filter(|matchup| !is_mirror_matchup(&matchup.label))
        .cloned()
        .collect();
    matchups.sort_by(|a, b| b.games.cmp(&a.games));
    // What the Debuggable blocks show, taken out of `details` first: each block is a closure.
    let summary = (
        details.num_games,
        details.min_date.clone(),
        details.max_date.clone(),
        details.avg_duration_seconds,
    );
    let summary_title = details.title.clone();
    let matchups_title = details.title.clone();
    let top_players = details.top_players.clone();
    let recent_games = details.recent_games.clone();
    let recent_title = details.title.clone();
    view! {
        <div class="flex items-start justify-between gap-4">
            <div>
                <h1 class="text-xl font-semibold text-base-content">{details.title.clone()}</h1>
                <Debuggable query="map_details::games_summary" view_source=crate::source!()>
                <p class="text-sm text-muted">
                    <a
                        href="#"
                        class="nav-link"
                        title="Open the games of this map in the Replay List"
                        on:click={
                            let map_title = summary_title.clone();
                            move |ev: MouseEvent| {
                                ev.prevent_default();
                                navigation.open_replays(&map_title, "");
                            }
                        }
                    >
                        {figure(summary.0)}
                        " games"
                    </a>
                    " from "
                    {figure(summary.1.clone())}
                    " to "
                    {figure(summary.2.clone())}
                    ", "
                    {figure(format_game_time(summary.3))}
                    " on average"
                </p>
                </Debuggable>
            </div>
            <div class="flex flex-col items-end gap-1">
                <div class="flex gap-2">
                    <button class="btn btn-primary btn-sm native-only" title=native_title("Open in Swarmy-Bevy") on:click=open_in_bevy>
                        <BrandIcon name="bevy" />
                        "Open in Swarmy-Bevy"
                    </button>
                </div>
                <span class="text-xs text-muted">{move || bevy_status.get()}</span>
            </div>
        </div>
        // The map stays in view while the sections beside it scroll, hovering a cell in them
        // highlights it on the map.
        <div class="grid grid-cols-[minmax(0,28rem)_minmax(0,1fr)] gap-6 items-start">
            <div class="sticky top-0 self-start pt-1">
                <MapPicture picture=details.picture.clone() />
            </div>
            <div class="space-y-5 min-w-0">
                <p class="text-xs text-muted whitespace-pre-line">{details.description.clone()}</p>
                // Side by side when they fit, one under the other on a narrow window.
                <div class="grid grid-cols-[repeat(auto-fit,minmax(16rem,1fr))] gap-6">
                    <Debuggable query="map_details::matchups" view_source=crate::source!()>
                    <table class=TABLE_CLASS>
                        <thead>
                            <tr class="text-left text-muted">
                                <th class="pr-2">"Matchup"</th>
                                <th class="pr-2 num">"Games"</th>
                                <th class="num" title="The win rate of the first race of the matchup">
                                    "First race favored"
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {matchups
                                .into_iter()
                                .map(|matchup| {
                                    let map_title = matchups_title.clone();
                                    let label = matchup.label.clone();
                                    view! {
                                        <tr
                                            class="cursor-pointer hover:bg-base-200"
                                            title="Open the games of this matchup on this map in the Replay List"
                                            on:click=move |_ev: MouseEvent| {
                                                navigation.open_replays(&map_title, &label)
                                            }
                                        >
                                            <td class="pr-2 text-base-content">
                                                <span class="nav-link">{matchup.label.clone()}</span>
                                            </td>
                                            <td class="pr-2 num">{matchup.games}</td>
                                            <td class="num">{win_rate(matchup.wins, matchup.games)}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                    </Debuggable>
                    <Debuggable query="map_details::top_players" view_source=crate::source!()>
                    <table class=TABLE_CLASS>
                        <thead>
                            <tr class="text-left text-muted">
                                <th class="pr-2">"Top players"</th>
                                <th class="pr-2 num">"Games"</th>
                                <th class="num">"Win rate"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {top_players
                                .iter()
                                .map(|player| {
                                    view! {
                                        <tr>
                                            <td class="pr-2">
                                                <PlayerLink player_name=player.label.clone() />
                                            </td>
                                            <td class="pr-2 num">{player.games}</td>
                                            <td class="num">{win_rate(player.wins, player.games)}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                    </Debuggable>
                </div>
                {details
                    .picture
                    .resources
                    .clone()
                    .map(|resources| view! { <MapResourcesPanel resources /> })}
                <Debuggable query="map_details::recent_games" view_source=crate::source!()>
                <div>
                    <h2 class=HEADING_CLASS>"Recent games"</h2>
                    <table class=TABLE_CLASS>
                        <thead>
                            <tr class="text-left text-muted">
                                <th class="pr-2">"ID"</th>
                                <th class="pr-2">"Date"</th>
                                <th title="The winner first">"Players"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {recent_games
                                .into_iter()
                                .map(|game| {
                                    // The winners first, then the others in their order.
                                    let mut players: Vec<(String, String)> = game
                                        .player_list
                                        .into_iter()
                                        .zip(game.player_results)
                                        .collect();
                                    players.sort_by_key(|(_, result)| result_order(result));
                                    let names: Vec<String> =
                                        players.iter().map(|(name, _)| name.clone()).collect();
                                    view! {
                                        <tr>
                                            <td class="pr-2">
                                                <ReplayLink
                                                    ext_fs_id=game.ext_fs_id
                                                    map_title=recent_title.clone()
                                                    players=names
                                                />
                                            </td>
                                            <td class="pr-2 tabular-nums">{game.replay_date.to_string()}</td>
                                            <td>
                                                {players
                                                    .into_iter()
                                                    .map(|(player_name, result)| {
                                                        view! {
                                                            <span class="mr-3">
                                                                <ResultMarker result />
                                                                <PlayerLink player_name />
                                                            </span>
                                                        }
                                                    })
                                                    .collect_view()}
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
                </Debuggable>
            </div>
        </div>
    }
}
