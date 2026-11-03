//! Leptos view for the details of a map: matchups, top players, recent games and terrain.

use super::resources::{MapPicture, MapResourcesPanel};
use crate::components::brand_icon::BrandIcon;
use crate::components::links::{PlayerLink, ReplayLink, ResultMarker};
use crate::components::loading::Loading;
use crate::components::map_cell::{MapCell, MapHighlightContext};
use crate::components::name_select::NameSelect;
use crate::navigation::use_navigation;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, BINOCULARS};
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
    let title = details.title.clone();
    let cache_handles = details.cache_handles.clone();
    // The spot hovered in the tables, highlighted on the map.
    provide_context(MapHighlightContext(RwSignal::new(None)));
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
    view! {
        <div class="flex items-start justify-between gap-4">
            <div>
                <h1 class="text-xl font-semibold text-base-content">{details.title.clone()}</h1>
                <p class="text-sm text-muted">
                    <a
                        href="#"
                        class="nav-link"
                        title="Open the games of this map in the Replay List"
                        on:click={
                            let map_title = details.title.clone();
                            move |ev: MouseEvent| {
                                ev.prevent_default();
                                navigation.open_replays(&map_title, "");
                            }
                        }
                    >
                        {figure(details.num_games)}
                        " games"
                    </a>
                    " from "
                    {figure(details.min_date.clone())}
                    " to "
                    {figure(details.max_date.clone())}
                    ", "
                    {figure(format_game_time(details.avg_duration_seconds))}
                    " on average"
                </p>
            </div>
            <div class="flex flex-col items-end gap-1">
                <div class="flex gap-2">
                    <button
                        class="btn btn-primary btn-sm"
                        title="Scout this map"
                        on:click=move |_ev: MouseEvent| navigation.scout_map(&title)
                    >
                        <Icon icon=BINOCULARS weight=IconWeight::Light />
                        "Scout this map"
                    </button>
                    <button class="btn btn-primary btn-sm" title="Open in Swarmy-Bevy" on:click=open_in_bevy>
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
                <MapPicture details=details.clone() />
            </div>
            <div class="space-y-5 min-w-0">
                <p class="text-xs text-muted whitespace-pre-line">{details.description.clone()}</p>
                <div class="grid grid-cols-2 gap-6">
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
                                    let map_title = details.title.clone();
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
                    <table class=TABLE_CLASS>
                        <thead>
                            <tr class="text-left text-muted">
                                <th class="pr-2">"Top players"</th>
                                <th class="pr-2 num">"Games"</th>
                                <th class="num">"Win rate"</th>
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
                                            <td class="pr-2 num">{player.games}</td>
                                            <td class="num">{win_rate(player.wins, player.games)}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </div>
                {details
                    .resources
                    .clone()
                    .map(|resources| view! { <MapResourcesPanel resources /> })}
                <MapLayersPanel
                    map_title=details.title.clone()
                    cache_handles=details.cache_handles.clone()
                />
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
                            {details
                                .recent_games
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
                                                    map_title=details.title.clone()
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
                <LongRangeSpotsPanel
                    map_title=details.title.clone()
                    cache_handles=details.cache_handles.clone()
                />
            </div>
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
                    title="The layers as bars on the map, each in its color, the keys F1 to F4 show or hide them"
                    on:click=open
                >
                    <BrandIcon name="bevy" />
                    "Show in swarmy-bevy"
                </button>
                <span class="text-xs text-muted">{move || status.get()}</span>
            </div>
            {move || match layers.get() {
                None => {
                    view! { <Loading message="Finding the fights, proxies and creep" /> }.into_any()
                }
                Some(Err(err)) => view! { <p class="text-xs text-muted">{err}</p> }.into_any(),
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
                                        let cells = layers.spot_cells as f32;
                                        let top = layer
                                            .spots
                                            .first()
                                            .map(|spot| {
                                                view! {
                                                    "the most at "
                                                    <MapCell
                                                        x=spot.x as f32 * cells
                                                        y=spot.y as f32 * cells
                                                        size=cells
                                                    />
                                                    ": "
                                                    {figure(spot.count)}
                                                    " in "
                                                    {figure(spot.games)}
                                                    " games"
                                                }
                                                    .into_any()
                                            })
                                            .unwrap_or_else(|| "none".into_any());
                                        view! {
                                            <tr title=layer.description.clone()>
                                                <td class="pr-3 text-muted">{layer.label.clone()}</td>
                                                <td class="pr-3 num">{total}</td>
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

/// Where the long range units stood and killed the most in the games of the map, with a button to
/// see all the spots as bars on the map in swarmy-bevy. A preview: only the Siege Tanks so far.
#[component]
fn LongRangeSpotsPanel(map_title: String, cache_handles: String) -> impl IntoView {
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
                <h2 class=HEADING_CLASS>
                    "Long Range Unit Spots "
                    <span class="badge badge-sm badge-outline font-normal">"preview"</span>
                </h2>
                <button
                    class="btn btn-primary btn-xs"
                    title="All the spots as bars on the map: the height is the kills, red the most kills per siege"
                    on:click=open
                >
                    <BrandIcon name="bevy" />
                    "Show in swarmy-bevy"
                </button>
                <span class="text-xs text-muted">{move || status.get()}</span>
            </div>
            <p class="text-xs text-faint">
                {format!(
                    "Where the Siege Tanks sieged, in squares of {SIEGE_SPOT_CELLS}x{SIEGE_SPOT_CELLS} cells, and the enemy units that cost resources they killed until they unsieged. \
                    This is a preview: more long range units (Lurkers, Liberators, Tempests...) and more of what they did are being added.",
                )}
            </p>
            {move || match spots.get() {
                None => view! { <Loading message="Finding the sieges" /> }.into_any(),
                Some(Err(err)) => view! { <p class="text-xs text-muted">{err}</p> }.into_any(),
                Some(Ok(spots)) if spots.spots.is_empty() => {
                    view! {
                        <p class="text-xs text-muted">
                            "No Siege Tank sieged in the games of this map."
                        </p>
                    }
                        .into_any()
                }
                Some(Ok(spots)) => {
                    view! {
                        <p class="text-xs text-muted">
                            {figure(spots.sieges)}
                            " sieges in "
                            {figure(spots.games)}
                            " games, "
                            {figure(spots.kills)}
                            " kills."
                        </p>
                        <table class=TABLE_CLASS>
                            <thead>
                                <tr class="text-left text-muted">
                                    <th class="pr-3">"Cell"</th>
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
                                        let cells = spots.spot_cells as f32;
                                        view! {
                                            <tr>
                                                <td class="pr-3">
                                                    <MapCell
                                                        x=spot.x as f32 * cells
                                                        y=spot.y as f32 * cells
                                                        size=cells
                                                    />
                                                </td>
                                                <td class="pr-3 num text-base-content">
                                                    {spot.kills}
                                                </td>
                                                <td class="pr-3 num">{spot.sieges}</td>
                                                <td class="pr-3 num">
                                                    {format!("{:.1}", spot.kills_per_siege())}
                                                </td>
                                                <td class="num">{spot.games}</td>
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
