//! The chat messages of the replays of the snapshot, read once per replay into the personal
//! database so that the replay list can search them.

use super::db_error;
use crate::data::*;
use duckdb::{params, Connection};
use polars::prelude::*;
use rayon::prelude::*;
use s2protocol::message_events::ReplayMessageEvent;
use std::collections::{HashMap, HashSet};
use swarmy_common::*;

/// The game loops per second at Faster speed.
const GAME_LOOPS_PER_SECOND: f64 = 22.4;

/// A replay of the snapshot whose messages are not indexed yet.
struct PendingReplay {
    sha256: String,
    file_name: String,
    /// The player names by the user id of the messages.
    players: HashMap<i64, String>,
}

struct Message {
    seconds: i32,
    player: String,
    text: String,
}

fn read_messages(replay: &PendingReplay) -> Result<Vec<Message>, SwarmyError> {
    let file_contents = s2protocol::read_file(&std::path::PathBuf::from(&replay.file_name))?;
    let (_, mpq) = s2protocol::parser::parse(&file_contents).map_err(|err| {
        SwarmyError::Other(format!("Unable to parse {}: {err:?}", replay.file_name))
    })?;
    let events = s2protocol::read_message_events(&replay.file_name, &mpq, &file_contents)?;
    let mut game_loop = 0;
    let mut messages = vec![];
    for event in events {
        // The message events store the loops since the previous message.
        game_loop += event.delta;
        let ReplayMessageEvent::EChat(chat) = event.event;
        messages.push(Message {
            seconds: (game_loop.max(0) as f64 / GAME_LOOPS_PER_SECOND) as i32,
            player: replay
                .players
                .get(&event.user_id)
                .cloned()
                .unwrap_or_else(|| format!("User {}", event.user_id)),
            text: chat.m_string,
        });
    }
    Ok(messages)
}

/// The replays of the snapshot that are not in the index yet.
fn pending_replays(
    conn: &Connection,
    replay_path: &str,
) -> Result<Vec<PendingReplay>, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(replay_path)?)?;
    let details = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .select([
            col("ext_fs_sha256"),
            col("ext_fs_file_name"),
            col("lobby_slot_user_id").cast(DataType::Int64),
            col("player_name"),
        ])
        .collect()?;
    let indexed: HashSet<String> = (|| {
        let mut stmt = conn.prepare("SELECT sha256 FROM indexed_replays")?;
        stmt.query_map([], |row| row.get::<_, String>(0))?
            .collect::<duckdb::Result<_>>()
    })()
    .map_err(db_error)?;
    let mut pending: HashMap<String, PendingReplay> = HashMap::new();
    let user_ids = details.column("lobby_slot_user_id")?.i64()?.clone();
    for (((sha256, file_name), user_id), player) in str_values(&details, "ext_fs_sha256")?
        .into_iter()
        .zip(str_values(&details, "ext_fs_file_name")?)
        .zip(user_ids.iter())
        .zip(str_values(&details, "player_name")?)
    {
        if indexed.contains(&sha256) {
            continue;
        }
        let replay = pending
            .entry(sha256.clone())
            .or_insert_with(|| PendingReplay {
                sha256,
                file_name,
                players: HashMap::new(),
            });
        if let Some(user_id) = user_id {
            replay.players.insert(user_id, player);
        }
    }
    Ok(pending.into_values().collect())
}

/// Reads the messages of the replays of the snapshot that are not in the index yet, returns how
/// many replays were read.
pub fn index_messages(conn: &mut Connection, replay_path: &str) -> Result<usize, SwarmyError> {
    let pending = pending_replays(conn, replay_path)?;
    if pending.is_empty() {
        return Ok(0);
    }
    tracing::info!("Reading the messages of {} replays", pending.len());
    // The replays that can not be read, i.e. moved, are not indexed, they are read again later.
    let read: Vec<(String, Vec<Message>)> = pending
        .par_iter()
        .filter_map(|replay| match read_messages(replay) {
            Ok(messages) => Some((replay.sha256.clone(), messages)),
            Err(err) => {
                tracing::warn!(
                    "Unable to read the messages of {}: {err:?}",
                    replay.file_name
                );
                None
            }
        })
        .collect();
    (|| {
        let tx = conn.transaction()?;
        {
            let mut indexed = tx.appender("indexed_replays")?;
            let mut messages = tx.appender("replay_messages")?;
            for (sha256, replay_messages) in &read {
                indexed.append_row(params![sha256])?;
                for message in replay_messages {
                    messages.append_row(params![
                        sha256,
                        message.seconds,
                        message.player,
                        message.text
                    ])?;
                }
            }
        }
        tx.commit()
    })()
    .map_err(db_error)?;
    Ok(read.len())
}

/// The replays with a message that contains the text, ignoring the case.
pub fn search_messages(conn: &Connection, text: &str) -> duckdb::Result<HashSet<String>> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT sha256 FROM replay_messages WHERE message ILIKE ? ESCAPE '\\'")?;
    let pattern = format!(
        "%{}%",
        text.replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let found = stmt
        .query_map(params![pattern], |row| row.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    Ok(found)
}
