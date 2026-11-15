//! Map Details: the games played on a map (summary, matchups, top players, recent games), its
//! picture and resources from the map cache, and the maps of the snapshot for the map field.
//! Each statistic is a query file of this directory (AGENTS.md, "A query is a file"); this file
//! runs them and puts the answer together.

pub mod games_summary;
pub mod maps;
pub mod matchups;
pub mod recent_games;
pub mod resources;
pub mod top_players;

use crate::data::{IpcSnapshot, QueryMeta};
use crate::host::SwarmyHost;
use crate::replay_rerun::terrain::{Terrain, load_map_cache};
use crate::run_query;
use crate::settings::load_stored_app_settings;
use resources::{map_image, map_resources};
use s2protocol::cache_handles::MapCache;
use swarmy_common::*;

/// The queries of the view, for the debug links of the panels.
pub const QUERIES: [QueryMeta; 4] = [
    games_summary::META,
    matchups::META,
    top_players::META,
    recent_games::META,
];

pub async fn query_map_details(
    host: &dyn SwarmyHost,
    map_title: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(try_query_map_details(
                &app_config.replay_path,
                &app_config.cache_path,
                &map_title,
            ))
    })
    .await
}

/// The statistics of the games of a map and its map cache.
pub fn try_query_map_details(
    replay_path: &str,
    cache_path: &str,
    map_title: &str,
) -> Result<MapDetails, SwarmyError> {
    let source = IpcSnapshot::open(replay_path)?;
    let summary = games_summary::shape(
        games_summary::plan(&source, map_title)?.collect()?,
        map_title,
    )?;
    let matchups = matchups::shape(matchups::plan(&source, map_title)?.collect()?)?;
    let top_players = top_players::shape(top_players::plan(&source, map_title)?.collect()?)?;
    let recent_games = recent_games::shape(
        recent_games::plan(&source, map_title)?.collect()?,
        map_title,
    )?;

    let map_cache = open_map_cache(cache_path, &summary.cache_handles)
        .inspect_err(|err| tracing::warn!("No map cache for {map_title}: {err:?}"))
        .ok();
    Ok(MapDetails {
        description: map_cache
            .as_ref()
            .map(|map_cache| {
                map_cache
                    .document_header
                    .description_long
                    .replace("<n/>", "\n")
            })
            .unwrap_or_default(),
        map_size: map_cache.as_ref().map_or((0., 0.), |map_cache| {
            (
                map_cache.map_info.cell_width as f32,
                map_cache.map_info.cell_height as f32,
            )
        }),
        picture: map_cache.as_ref().map(map_picture).unwrap_or_default(),
        title: map_title.to_string(),
        num_games: summary.games,
        min_date: summary.min_date,
        max_date: summary.max_date,
        avg_duration_seconds: summary.avg_duration_seconds,
        cache_handles: summary.cache_handles,
        matchups,
        top_players,
        recent_games,
    })
}

/// The map cache of the comma separated cache handles of a game.
pub fn open_map_cache(cache_path: &str, cache_handles: &str) -> Result<MapCache, SwarmyError> {
    let handles: Vec<String> = cache_handles.split(',').map(str::to_string).collect();
    load_map_cache(cache_path, &handles)
}

/// The picture of a map cache: its minimap, its terrain levels and its resources.
pub fn map_picture(map_cache: &MapCache) -> MapPicture {
    MapPicture {
        minimap: map_image(map_cache),
        terrain: Some(Terrain::from(map_cache).minimap()),
        resources: Some(map_resources(&map_cache.placed_objects)),
    }
}

/// The maps of the snapshot, for the map field.
pub async fn list_maps(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources([QuerySource::from(maps::META)])
            .process_result(try_list_maps(&app_config.replay_path))
    })
    .await
}

pub fn try_list_maps(replay_path: &str) -> Result<Vec<GamesWins>, SwarmyError> {
    let source = IpcSnapshot::open(replay_path)?;
    maps::shape(maps::plan(&source)?.collect()?)
}
