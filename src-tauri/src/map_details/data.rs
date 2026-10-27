//! Statistics of the games played in a map.

use super::resources::{map_image, map_resources};
use crate::data::*;
use crate::replay_list::data::try_query_replay_list;
use crate::replay_rerun::terrain::{load_map_cache, Terrain};
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

/// The players shown, by number of games.
pub const TOP_PLAYERS: usize = 10;

/// The games shown, by date.
pub const RECENT_GAMES: usize = 20;

fn add_game(counts: &mut BTreeMap<String, GamesWins>, label: String, won: bool) {
    let entry = counts.entry(label.clone()).or_insert_with(|| GamesWins {
        label,
        ..Default::default()
    });
    entry.games += 1;
    entry.wins += won as usize;
}

pub fn try_query_map_details(
    replay_path: String,
    cache_path: String,
    map_title: String,
) -> Result<MapDetails, SwarmyError> {
    let sanitized_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&sanitized_path)?;
    let details = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .filter(col("title").eq(lit(map_title.clone())))
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("player_race"),
            col("player_result"),
            col("cache_handles"),
            col("ext_datetime").dt().to_string("%Y-%m-%d").alias("date"),
        ])
        .collect()?;
    if details.height() == 0 {
        return Err(SwarmyError::Other(format!(
            "No games found for map {map_title}"
        )));
    }
    let ext_fs_ids = u64_values(&details, "ext_fs_id")?;
    let names = str_values(&details, "player_name")?;
    let races: Vec<String> = str_values(&details, "player_race")?
        .iter()
        .map(|race| normalize_race(race))
        .collect();
    let results = str_values(&details, "player_result")?;
    let dates = str_values(&details, "date")?;
    let cache_handles = str_values(&details, "cache_handles")?.remove(0);

    let mut games: BTreeMap<u64, Vec<(String, bool)>> = BTreeMap::new();
    let mut players = BTreeMap::new();
    for idx in 0..details.height() {
        let won = results[idx] == "Win";
        games
            .entry(ext_fs_ids[idx])
            .or_default()
            .push((races[idx].clone(), won));
        add_game(&mut players, names[idx].clone(), won);
    }
    let mut matchups = BTreeMap::new();
    for game in games.values() {
        let [(race, won), (other_race, other_won)] = game.as_slice() else {
            // Only 1v1 games have a matchup.
            continue;
        };
        // ZvT is the same matchup as TvZ, the races are sorted (PvT, PvZ, TvZ) and the wins are
        // the ones of the first race.
        let ((race, won), other_race) = if race_letter(race) <= race_letter(other_race) {
            ((race, won), other_race)
        } else {
            ((other_race, other_won), race)
        };
        let won = *won && race_letter(race) != race_letter(other_race);
        add_game(&mut matchups, matchup_label(race, other_race), won);
    }
    let mut top_players: Vec<GamesWins> = players.into_values().collect();
    top_players.sort_by(|a, b| b.games.cmp(&a.games).then(b.wins.cmp(&a.wins)));
    top_players.truncate(TOP_PLAYERS);

    let game_ids: Vec<u64> = games.keys().copied().collect();
    let durations = scan_snapshot_ipc(&ipc_path, STATS_IPC)?
        .filter(is_in_replays(&game_ids))
        .group_by([col("ext_fs_id")])
        .agg([game_seconds().max()])
        .select([col("game_seconds").mean()])
        .collect()?;
    let avg_duration_seconds = durations
        .column("game_seconds")?
        .f64()?
        .get(0)
        .unwrap_or_default() as u32;

    let replay_query = ReplayListQuery {
        map_title: map_title.clone(),
        ..ReplayListQuery::newest_first()
    };
    let mut recent_games: Vec<ReplayListEntry> =
        try_query_replay_list(replay_path, replay_query, None)?
            .replays
            .into_iter()
            .filter(|game| game.map_title == map_title)
            .collect();
    recent_games.truncate(RECENT_GAMES);

    let handles: Vec<String> = cache_handles.split(',').map(str::to_string).collect();
    let map_cache = load_map_cache(&cache_path, &handles)
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
        terrain: map_cache
            .as_ref()
            .map(|map_cache| Terrain::from(map_cache).minimap()),
        map_size: map_cache.as_ref().map_or((0., 0.), |map_cache| {
            (
                map_cache.map_info.cell_width as f32,
                map_cache.map_info.cell_height as f32,
            )
        }),
        minimap: map_cache.as_ref().and_then(map_image),
        resources: map_cache
            .as_ref()
            .map(|map_cache| map_resources(&map_cache.placed_objects)),
        title: map_title,
        num_games: games.len(),
        min_date: dates.iter().min().cloned().unwrap_or_default(),
        max_date: dates.iter().max().cloned().unwrap_or_default(),
        avg_duration_seconds,
        cache_handles,
        matchups: matchups.into_values().collect(),
        top_players,
        recent_games,
    })
}

/// The maps of the snapshot by games played, for the map dropdown.
pub fn try_list_maps(replay_path: String) -> Result<Vec<GamesWins>, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let maps = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .group_by([col("title")])
        .agg([col("ext_fs_id")
            .n_unique()
            .cast(DataType::UInt64)
            .alias("games")])
        .sort(["title"], SortMultipleOptions::default())
        .collect()?;
    Ok(str_values(&maps, "title")?
        .into_iter()
        .zip(u64_values(&maps, "games")?)
        .map(|(label, games)| GamesWins {
            label,
            games: games as usize,
            wins: 0,
        })
        .collect())
}
