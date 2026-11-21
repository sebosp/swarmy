//! Replay List: the games of the snapshot filtered and sorted by the viewer, with the notes, tags
//! and messages of the personal database, and the dropdowns of its filters. Each statistic is a
//! query file of this directory (AGENTS.md, "A query is a file"); this file runs them and puts the
//! answer together.

pub mod games;
pub mod protocol_versions;
pub mod years;

use crate::data::{IpcSnapshot, QueryMeta};
use crate::host::SwarmyHost;
use crate::personal_db::{
    db_error, db_path, filter_replays, list_tags, messages, notes_summaries, with_db,
};
use crate::replay_rerun::viewer::{ensure_viewer_running, stream_replay_to_viewer};
use crate::run_query;
use crate::settings::load_stored_app_settings;
use polars::prelude::len;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use swarmy_common::*;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_opener::OpenerExt;

/// The queries of the view, for the debug links of the panels.
pub const QUERIES: [QueryMeta; 1] = [games::META];

/// The queries of the dropdowns of the filters.
pub const FILTER_QUERIES: [QueryMeta; 2] = [protocol_versions::META, years::META];

pub async fn query_replay_list(
    host: &dyn SwarmyHost,
    query: ReplayListQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let db = db_path(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(query_replays_with_notes(
                &db,
                &app_config.replay_path,
                query,
            ))
    })
    .await
}

/// The replays of the query, filtered and annotated with the notes, tags and messages of the
/// personal database.
fn query_replays_with_notes(
    db: &Path,
    replay_path: &str,
    query: ReplayListQuery,
) -> Result<ReplayList, SwarmyError> {
    let replays = with_db(db, |conn| {
        let mut replays = filter_replays(conn, &query.tag, query.min_rating).map_err(db_error)?;
        let message = query.message.trim();
        if !message.is_empty() {
            // The messages of the new replays are read the first time they are searched.
            messages::index_messages(conn, replay_path)?;
            let found = messages::search_messages(conn, message).map_err(db_error)?;
            replays = Some(match replays {
                Some(replays) => replays.intersection(&found).cloned().collect(),
                None => found,
            });
        }
        Ok(replays)
    })?;
    let groups = with_db(db, |conn| {
        crate::personal_db::player_groups(conn).map_err(db_error)
    })
    .unwrap_or_default();
    let mut list = try_query_replay_list(replay_path, query, replays.as_ref(), &groups)?;
    let sha256s: Vec<String> = list
        .replays
        .iter()
        .map(|replay| replay.replay_sha256.clone())
        .collect();
    let summaries = with_db(db, |conn| notes_summaries(conn, &sha256s).map_err(db_error))?;
    for replay in &mut list.replays {
        if let Some((rating, tags)) = summaries.get(&replay.replay_sha256) {
            replay.rating = *rating;
            replay.tags = tags.clone();
        }
    }
    Ok(list)
}

/// The games of the query: how many match, and the first [`REPLAY_LIST_LIMIT`] in the order
/// asked, with the best ability rank of each game when the ability usage is computed.
pub fn try_query_replay_list(
    replay_path: &str,
    query: ReplayListQuery,
    replays: Option<&HashSet<String>>,
    groups: &HashMap<String, String>,
) -> Result<ReplayList, SwarmyError> {
    let source = IpcSnapshot::open(replay_path)?;
    tracing::info!(
        "Querying the replay list from {}: {query:?}",
        source.ipc_path()
    );
    let games = games::plan(&source, groups, &query, replays)?;
    let total = games
        .clone()
        .select([len().alias("games")])
        .collect()?
        .column("games")?
        .u32()?
        .get(0)
        .unwrap_or(0) as usize;
    let mut replays = games::shape(games.limit(REPLAY_LIST_LIMIT).collect()?)?;
    // Without the ability usage the list is still shown.
    match crate::ability_usage::ability_usage_cached(source.ipc_path()) {
        Ok(usage) => {
            if let Err(err) =
                crate::ability_usage::annotate_replay_list(source.ipc_path(), &mut replays, &usage)
            {
                tracing::warn!("No game ranks for the replay list: {err:?}");
            }
        }
        Err(err) => tracing::warn!("No ability usage for the replay list: {err:?}"),
    }
    Ok(ReplayList { total, replays })
}

/// The patches, years and tags of the snapshot, for the dropdowns of the replay list.
pub async fn list_replay_filters(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let db = db_path(host)?;
    run_query(move || {
        let filters = (|| {
            let source = IpcSnapshot::open(&app_config.replay_path)?;
            Ok::<_, SwarmyError>(ReplayListFilters {
                protocol_versions: protocol_versions::shape(
                    protocol_versions::plan(&source)?.collect()?,
                )?,
                years: years::shape(years::plan(&source)?.collect()?)?,
                tags: with_db(&db, |conn| list_tags(conn).map_err(db_error))?,
            })
        })();
        ApiResponseBuilder::new()
            .with_sources(FILTER_QUERIES.map(QuerySource::from))
            .process_result(filters)
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn copy_path_to_clipboard(
    app_handle: tauri::AppHandle,
    data: String,
) -> Result<ApiResponse, SwarmyError> {
    let res = ApiResponseBuilder::new();
    tracing::info!("Writing {data} to clipboard.",);
    app_handle.clipboard().write_text(data).unwrap();
    Ok(res
        .with_message("Succesfully wrote to clipboard.".to_string())
        .with_success()
        .build())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_folder(
    app_handle: tauri::AppHandle,
    folder: String,
) -> Result<ApiResponse, SwarmyError> {
    tracing::info!("Requesting open on folder: {}", folder);
    let res = match app_handle.opener().open_path(folder, None::<&str>) {
        Ok(_) => ApiResponseBuilder::new()
            .with_message("Succesfully called open.".to_string())
            .with_success()
            .build(),
        Err(err) => {
            tracing::error!("Unable to open folder: {:?}", err);
            ApiResponseBuilder::new()
                .with_message(format!("Error requesting open: {:?}", err))
                .with_failure()
                .build()
        }
    };
    Ok(res)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn connect_swarmy_rerun(
    app_handle: tauri::AppHandle,
    replay_file_name: String,
) -> Result<ApiResponse, SwarmyError> {
    tracing::info!(
        "Requesting open rerun on replay_file_name: {}",
        replay_file_name
    );
    let app_config = load_stored_app_settings(&app_handle)?;
    // With a remote backend the replay is on its computer: it is copied here with its map caches,
    // as for swarmy-bevy.
    let (stream_file_name, cache_path) = if app_config.is_remote() {
        let dir = crate::remote::local_files_dir(&app_handle, &app_config)?;
        let replay =
            crate::remote::download_replay(&app_config, &replay_file_name, &dir.join("replays"))
                .await?;
        let cache_ids = crate::swarmy_bevy::replay_cache_ids(&replay)?;
        let caches =
            crate::swarmy_bevy::map_caches_dir(&app_handle, &app_config, &cache_ids).await?;
        (replay.display().to_string(), caches)
    } else {
        (replay_file_name.clone(), app_config.cache_path.clone())
    };
    ensure_viewer_running(&app_handle).await?;
    let streamed = tauri::async_runtime::spawn_blocking(move || {
        // SwarmyError is not Send, so it crosses the thread boundary as a String.
        stream_replay_to_viewer(&stream_file_name, &cache_path).map_err(String::from)
    })
    .await
    .map_err(|e| SwarmyError::Other(format!("Rerun streaming task failed: {e}")))?;
    let res = match streamed {
        Ok(total_events) => ApiResponseBuilder::new()
            .with_message(format!(
                "Succesfully sent {total_events} events to rerun for {replay_file_name}."
            ))
            .with_success()
            .build(),
        Err(err) => {
            tracing::error!("Unable to call rerun: {:?}", err);
            ApiResponseBuilder::new()
                .with_message(format!("Error calling rerun: {:?}", err))
                .with_failure()
                .build()
        }
    };
    Ok(res)
}
