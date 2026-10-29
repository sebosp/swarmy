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
                <p class="text-sm">
                    "Choose a map above or by clicking on its title in the Map List or Replay List."
                </p>
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
                    on:click=move |_ev: MouseEvent| trigger_swarmy_bevy_exec_on_caches(
                        &cache_handles,
                    )
                >
                    <Icon icon=CUBE weight=IconWeight::Light />
                    "Open in Swarmy-Bevy"
                </button>
            </div>
        </div>
        <div class="grid grid-cols-2 gap-4">
            <div>
                <p class="text-xs text-gray-400 whitespace-pre-line">
                    {details.description.clone()}
                </p>
                <MapPicture details=details.clone() />
                {details
                    .resources
                    .clone()
                    .map(|resources| view! { <MapResourcesPanel resources /> })}
            </div>
            <div class="space-y-4">
                <div>
                    <h2 class=HEADING_CLASS>"Race vs race"</h2>
                    <table class=TABLE_CLASS>
                        <thead>
                            <tr class="text-left text-gray-300">
                                <th class="pr-2">"Matchup"</th>
                                <th class="pr-2">"Games"</th>
                                <th title="The win rate of the first race of the matchup">
                                    "First race wins"
                                </th>
                            </tr>
                        </thead>
                        <tbody>
                            {details
                                .matchups
                                .iter()
                                .map(|matchup| {
                                    let mirror = is_mirror_matchup(&matchup.label);
                                    let map_title = details.title.clone();
                                    let label = matchup.label.clone();
                                    // Mirror matchups always have a winner and a loser.
                                    view! {
                                        <tr
                                            class="cursor-pointer hover:bg-gray-800"
                                            title="Open the games of this matchup on this map in the Replay List"
                                            on:click=move |_ev: MouseEvent| {
                                                navigation.open_replays(&map_title, &label)
                                            }
                                        >
                                            <td class="pr-2 text-white">
                                                <span class="nav-link">{matchup.label.clone()}</span>
                                            </td>
                                            <td class="pr-2">{matchup.games}</td>
                                            <td>
                                                {if mirror {
                                                    "-".to_string()
                                                } else {
                                                    win_rate(matchup.wins, matchup.games)
                                                }}
                                            </td>
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
                                .map(|player| {
                                    view! {
                                        <tr>
                                            <td class="pr-2">
                                                <PlayerLink player_name=player.label.clone() />
                                            </td>
                                            <td class="pr-2">{player.games}</td>
                                            <td>{win_rate(player.wins, player.games)}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
        <MapLayersPanel
            map_title=details.title.clone()
            cache_handles=details.cache_handles.clone()
        />
        <SiegeSpotsPanel
            map_title=details.title.clone()
            cache_handles=details.cache_handles.clone()
        />
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
                        .map(|game| {
                            view! {
                                <tr>
                                    <td class="pr-2">
                                        <ReplayLink ext_fs_id=game.ext_fs_id />
                                    </td>
                                    <td class="pr-2">{game.replay_date.to_string()}</td>
                                    <td>
                                        {game
                                            .player_list
                                            .into_iter()
                                            .zip(game.player_results)
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
    }
}

/// The spots shown in the table, the most kills first.
const SIEGE_SPOTS_SHOWN: usize = 8;

/// The fights, proxies, creep tumors and early buildings of the games of the map, with a button
/// to see them as bars on the map in swarmy-bevy.
#[component]
fn MapLayersPanel(map_title: String, cache_handles: String) -> impl IntoView {
    #[derive(serde::Serialize)]
    struct Args {
        map_title: String,
    }
    #[derive(serde::Serialize)]
    struct BevyArgs {
        map_title: String,
        cache_ids: String,
    }
    let layers = RwSignal::new(None::<Result<MapLayers, String>>);
    let status = RwSignal::new(String::new());
    let query_title = map_title.clone();
    spawn_local(async move {
        layers.set(Some(
            invoke_json::<MapLayers>(
                "query_map_layers",
                &Args {
                    map_title: query_title,
                },
            )
            .await,
        ));
    });
    let open = move |_ev: MouseEvent| {
        let args = BevyArgs {
            map_title: map_title.clone(),
            cache_ids: cache_handles.clone(),
        };
        status.set("Starting swarmy-bevy...".to_string());
        spawn_local(async move {
            match invoke_response("exec_swarmy_bevy_map_layers", &args).await {
                Ok(response) => status.set(response.message),
                Err(err) => status.set(err),
            }
        });
    };
    view! {
        <div>
            <div class="flex items-center gap-2">
                <h2 class=HEADING_CLASS>"Map lessons"</h2>
                <button
                    class="btn btn-primary btn-xs"
                    title="The layers as bars on the map, each in its color, the keys 1 to 4 show or hide them"
                    on:click=open
                >
                    <Icon icon=CUBE weight=IconWeight::Light />
                    "Show in swarmy-bevy"
                </button>
                <span class="text-xs text-gray-400">{move || status.get()}</span>
            </div>
            {move || match layers.get() {
                None => {
                    view! { <Loading message="Finding the fights, proxies and creep" /> }.into_any()
                }
                Some(Err(err)) => view! { <p class="text-xs text-gray-400">{err}</p> }.into_any(),
                Some(Ok(layers)) => {
                    view! {
                        <table class=TABLE_CLASS>
                            <tbody>
                                {layers
                                    .layers
                                    .iter()
                                    .map(|layer| {
                                        let total: usize = layer
                                            .spots
                                            .iter()
                                            .map(|spot| spot.count)
                                            .sum();
                                        let top = layer
                                            .spots
                                            .first()
                                            .map(|spot| {
                                                format!(
                                                    "the most at cells ({}, {}): {} in {} games",
                                                    spot.x * layers.spot_cells,
                                                    spot.y * layers.spot_cells,
                                                    spot.count,
                                                    spot.games,
                                                )
                                            })
                                            .unwrap_or_else(|| "none".to_string());
                                        view! {
                                            <tr title=layer.description.clone()>
                                                <td class="pr-3 text-gray-300">{layer.label.clone()}</td>
                                                <td class="pr-3 text-right font-mono">{total}</td>
                                                <td>{top}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}

/// Where the Siege Tanks sieged and killed the most in the games of the map, with a button to see
/// all the spots as bars on the map in swarmy-bevy.
#[component]
fn SiegeSpotsPanel(map_title: String, cache_handles: String) -> impl IntoView {
    #[derive(serde::Serialize)]
    struct Args {
        map_title: String,
    }
    #[derive(serde::Serialize)]
    struct BevyArgs {
        map_title: String,
        cache_ids: String,
    }
    let spots = RwSignal::new(None::<Result<SiegeSpots, String>>);
    let status = RwSignal::new(String::new());
    let query_title = map_title.clone();
    spawn_local(async move {
        spots.set(Some(
            invoke_json::<SiegeSpots>(
                "query_siege_spots",
                &Args {
                    map_title: query_title,
                },
            )
            .await,
        ));
    });
    let open = move |_ev: MouseEvent| {
        let args = BevyArgs {
            map_title: map_title.clone(),
            cache_ids: cache_handles.clone(),
        };
        status.set("Starting swarmy-bevy...".to_string());
        spawn_local(async move {
            match invoke_response("exec_swarmy_bevy_siege_spots", &args).await {
                Ok(response) => status.set(response.message),
                Err(err) => status.set(err),
            }
        });
    };
    view! {
        <div>
            <div class="flex items-center gap-2">
                <h2 class=HEADING_CLASS>"Siege Tank spots"</h2>
                <button
                    class="btn btn-primary btn-xs"
                    title="All the spots as bars on the map: the height is the kills, red the most kills per siege"
                    on:click=open
                >
                    <Icon icon=CUBE weight=IconWeight::Light />
                    "Show in swarmy-bevy"
                </button>
                <span class="text-xs text-gray-400">{move || status.get()}</span>
            </div>
            <p class="text-xs text-gray-500">
                {format!(
                    "Where the Siege Tanks sieged, in squares of {SIEGE_SPOT_CELLS}x{SIEGE_SPOT_CELLS} cells, and the enemy units that cost resources they killed until they unsieged.",
                )}
            </p>
            {move || match spots.get() {
                None => view! { <Loading message="Finding the sieges" /> }.into_any(),
                Some(Err(err)) => view! { <p class="text-xs text-gray-400">{err}</p> }.into_any(),
                Some(Ok(spots)) if spots.spots.is_empty() => {
                    view! {
                        <p class="text-xs text-gray-400">
                            "No Siege Tank sieged in the games of this map."
                        </p>
                    }
                        .into_any()
                }
                Some(Ok(spots)) => {
                    view! {
                        <p class="text-xs text-gray-300">
                            {format!(
                                "{} sieges in {} games, {} kills.",
                                spots.sieges,
                                spots.games,
                                spots.kills,
                            )}
                        </p>
                        <table class=TABLE_CLASS>
                            <thead>
                                <tr class="text-left text-gray-300">
                                    <th class="pr-3">"Cells"</th>
                                    <th class="pr-3 text-right">"Kills"</th>
                                    <th class="pr-3 text-right">"Sieges"</th>
                                    <th class="pr-3 text-right">"Kills per siege"</th>
                                    <th class="text-right">"Games"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {spots
                                    .spots
                                    .iter()
                                    .take(SIEGE_SPOTS_SHOWN)
                                    .map(|spot| {
                                        let (x, y) = (
                                            spot.x * spots.spot_cells,
                                            spot.y * spots.spot_cells,
                                        );
                                        view! {
                                            <tr>
                                                <td class="pr-3 font-mono">{format!("({x}, {y})")}</td>
                                                <td class="pr-3 text-right font-mono text-white">
                                                    {spot.kills}
                                                </td>
                                                <td class="pr-3 text-right font-mono">{spot.sieges}</td>
                                                <td class="pr-3 text-right font-mono">
                                                    {format!("{:.1}", spot.kills_per_siege())}
                                                </td>
                                                <td class="text-right font-mono">{spot.games}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}
