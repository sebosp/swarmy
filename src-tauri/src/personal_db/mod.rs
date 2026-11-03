//! The personal database of the user, a DuckDB file in the app data directory: the notes, ratings
//! and tags of the replays, and the chat messages of the replays to search them.
//!
//! The replays are identified by the sha256 of their file, the ext_fs_id changes when a snapshot is
//! generated again. The database can be opened with DuckDB, i.e. from a notebook, and joined with
//! the snapshot on `ext_fs_sha256`.

pub mod messages;

use crate::host::SwarmyHost;
use duckdb::{params, Connection};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use swarmy_common::*;

/// The file of the database in the app data directory.
const DB_FILE_NAME: &str = "swarmy.duckdb";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS replay_notes (
    sha256 VARCHAR PRIMARY KEY,
    rating TINYINT,
    note VARCHAR NOT NULL DEFAULT '',
    updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);
CREATE TABLE IF NOT EXISTS replay_tags (
    sha256 VARCHAR NOT NULL,
    tag VARCHAR NOT NULL,
    PRIMARY KEY (sha256, tag)
);
-- The replays whose messages are in replay_messages, also the ones without messages.
CREATE TABLE IF NOT EXISTS indexed_replays (
    sha256 VARCHAR PRIMARY KEY
);
-- The accounts (region-realm-id) grouped under a player, i.e. the accounts of Clem.
CREATE TABLE IF NOT EXISTS player_accounts (
    account VARCHAR PRIMARY KEY,
    player VARCHAR NOT NULL
);
CREATE TABLE IF NOT EXISTS replay_messages (
    sha256 VARCHAR NOT NULL,
    seconds INTEGER NOT NULL,
    player VARCHAR NOT NULL,
    message VARCHAR NOT NULL
);
";

/// The errors of the database, SwarmyError is defined in swarmy-common without DuckDB.
pub fn db_error(err: duckdb::Error) -> SwarmyError {
    SwarmyError::Other(format!("Personal database: {err}"))
}

/// The commands use the database one at a time in this process.
static DB_LOCK: Mutex<()> = Mutex::new(());

/// How long a command waits for another process to release the database.
const DB_LOCK_WAIT: Duration = Duration::from_secs(10);

/// Opens the database at the path, creating its tables.
pub fn open_db(path: &Path) -> Result<Connection, SwarmyError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path).map_err(db_error)?;
    conn.execute_batch(SCHEMA).map_err(db_error)?;
    Ok(conn)
}

pub fn db_path(host: &dyn SwarmyHost) -> Result<PathBuf, SwarmyError> {
    Ok(host.data_dir()?.join(DB_FILE_NAME))
}

/// Runs a function with the database. It is opened for each function and closed after, DuckDB
/// locks the file while it is open: the desktop app and a swarmy started with `--serve` on the
/// same computer share the database, the one that finds it locked waits for the other.
pub fn with_db<T>(
    path: &Path,
    f: impl FnOnce(&mut Connection) -> Result<T, SwarmyError>,
) -> Result<T, SwarmyError> {
    let _guard = DB_LOCK
        .lock()
        .map_err(|_| SwarmyError::Other("The personal database lock is poisoned".to_string()))?;
    let start = Instant::now();
    let mut conn = loop {
        match open_db(path) {
            Ok(conn) => break conn,
            Err(err) if is_locked(&err) && start.elapsed() < DB_LOCK_WAIT => {
                tracing::debug!("Waiting for the personal database: {err}");
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(err) => return Err(err),
        }
    };
    f(&mut conn)
}

/// Whether another process has the database open.
fn is_locked(err: &SwarmyError) -> bool {
    matches!(err, SwarmyError::Other(message) if message.contains("lock"))
}

/// The player each grouped account belongs to, by account.
pub fn player_groups(conn: &Connection) -> duckdb::Result<HashMap<String, String>> {
    let mut statement = conn.prepare("SELECT account, player FROM player_accounts")?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

/// The grouped accounts, by account: empty when the database cannot be read, the players are then
/// their accounts.
pub fn read_player_groups(host: &dyn SwarmyHost) -> HashMap<String, String> {
    db_path(host)
        .and_then(|path| with_db(&path, |conn| player_groups(conn).map_err(db_error)))
        .unwrap_or_else(|err| {
            tracing::warn!("No player groups: {err:?}");
            HashMap::new()
        })
}

/// Groups an account under a player, or removes it from its group.
pub fn change_player_account(
    conn: &Connection,
    change: &PlayerAccountChange,
) -> duckdb::Result<()> {
    if change.grouped {
        conn.execute(
            "INSERT OR REPLACE INTO player_accounts (account, player) VALUES (?, ?)",
            duckdb::params![change.account, change.player],
        )?;
    } else {
        conn.execute(
            "DELETE FROM player_accounts WHERE account = ?",
            duckdb::params![change.account],
        )?;
    }
    Ok(())
}

/// Groups the accounts under a player: the accounts of a player, i.e. Clem's of each region and
/// of the tournaments, count as one player in the views.
pub async fn set_player_accounts(
    host: &dyn SwarmyHost,
    changes: Vec<PlayerAccountChange>,
) -> Result<ApiResponse, SwarmyError> {
    with_db(&db_path(host)?, |conn| {
        for change in &changes {
            change_player_account(conn, change).map_err(db_error)?;
        }
        Ok(())
    })?;
    Ok(ApiResponseBuilder::new().process_result(Ok(changes.len())))
}

/// The notes, rating and tags of a replay.
pub fn get_notes(conn: &Connection, sha256: &str) -> duckdb::Result<ReplayNotes> {
    let mut notes = conn
        .query_row(
            "SELECT rating, note FROM replay_notes WHERE sha256 = ?",
            params![sha256],
            |row| {
                Ok(ReplayNotes {
                    sha256: sha256.to_string(),
                    rating: row.get::<_, Option<u8>>(0)?,
                    note: row.get(1)?,
                    tags: vec![],
                })
            },
        )
        .unwrap_or_else(|_| ReplayNotes {
            sha256: sha256.to_string(),
            ..Default::default()
        });
    let mut stmt = conn.prepare("SELECT tag FROM replay_tags WHERE sha256 = ? ORDER BY tag")?;
    notes.tags = stmt
        .query_map(params![sha256], |row| row.get::<_, String>(0))?
        .collect::<Result<_, _>>()?;
    Ok(notes)
}

/// Saves the notes, rating and tags of a replay, replacing the previous ones.
pub fn set_notes(conn: &mut Connection, notes: &ReplayNotes) -> duckdb::Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT OR REPLACE INTO replay_notes (sha256, rating, note, updated_at)
         VALUES (?, ?, ?, current_timestamp)",
        params![notes.sha256, notes.rating, notes.note],
    )?;
    tx.execute(
        "DELETE FROM replay_tags WHERE sha256 = ?",
        params![notes.sha256],
    )?;
    let tags: HashSet<String> = notes
        .tags
        .iter()
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect();
    for tag in tags {
        tx.execute(
            "INSERT INTO replay_tags (sha256, tag) VALUES (?, ?)",
            params![notes.sha256, tag],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// The tags used, with their number of replays, the most used first.
pub fn list_tags(conn: &Connection) -> duckdb::Result<Vec<GamesWins>> {
    let mut stmt = conn.prepare(
        "SELECT tag, count(*) FROM replay_tags GROUP BY tag ORDER BY count(*) DESC, tag",
    )?;
    let tags = stmt
        .query_map([], |row| {
            Ok(GamesWins {
                label: row.get(0)?,
                games: row.get::<_, i64>(1)? as usize,
                wins: 0,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(tags)
}

/// The replays with the tag and at least the rating, none for no filter.
pub fn filter_replays(
    conn: &Connection,
    tag: &str,
    min_rating: Option<u8>,
) -> duckdb::Result<Option<HashSet<String>>> {
    let mut sets: Vec<HashSet<String>> = vec![];
    if !tag.is_empty() {
        let mut stmt = conn.prepare("SELECT sha256 FROM replay_tags WHERE tag = ?")?;
        sets.push(
            stmt.query_map(params![tag], |row| row.get::<_, String>(0))?
                .collect::<Result<_, _>>()?,
        );
    }
    if let Some(min_rating) = min_rating {
        let mut stmt = conn.prepare("SELECT sha256 FROM replay_notes WHERE rating >= ?")?;
        sets.push(
            stmt.query_map(params![min_rating], |row| row.get::<_, String>(0))?
                .collect::<Result<_, _>>()?,
        );
    }
    Ok(sets
        .into_iter()
        .reduce(|a, b| a.intersection(&b).cloned().collect()))
}

/// The rating and tags of a replay.
pub type NotesSummary = (Option<u8>, Vec<String>);

/// The rating and tags of the replays, for the replay list.
pub fn notes_summaries(
    conn: &Connection,
    sha256s: &[String],
) -> duckdb::Result<HashMap<String, NotesSummary>> {
    let wanted: HashSet<&String> = sha256s.iter().collect();
    let mut summaries: HashMap<String, NotesSummary> = HashMap::new();
    let mut stmt = conn.prepare("SELECT sha256, rating FROM replay_notes")?;
    for row in stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<u8>>(1)?))
    })? {
        let (sha256, rating) = row?;
        if wanted.contains(&sha256) {
            summaries.entry(sha256).or_default().0 = rating;
        }
    }
    let mut stmt = conn.prepare("SELECT sha256, tag FROM replay_tags ORDER BY tag")?;
    for row in stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })? {
        let (sha256, tag) = row?;
        if wanted.contains(&sha256) {
            summaries.entry(sha256).or_default().1.push(tag);
        }
    }
    Ok(summaries)
}

pub async fn get_replay_notes(
    host: &dyn SwarmyHost,
    sha256: String,
) -> Result<ApiResponse, SwarmyError> {
    let result = with_db(&db_path(host)?, |conn| {
        get_notes(conn, &sha256).map_err(db_error)
    });
    Ok(ApiResponseBuilder::new().process_result(result))
}

pub async fn set_replay_notes(
    host: &dyn SwarmyHost,
    notes: ReplayNotes,
) -> Result<ApiResponse, SwarmyError> {
    let result = with_db(&db_path(host)?, |conn| {
        set_notes(conn, &notes).map_err(db_error)?;
        get_notes(conn, &notes.sha256).map_err(db_error)
    });
    Ok(ApiResponseBuilder::new().process_result(result))
}

pub async fn list_replay_tags(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let result = with_db(&db_path(host)?, |conn| list_tags(conn).map_err(db_error));
    Ok(ApiResponseBuilder::new().process_result(result))
}

/// The number of tags of each replay, used by the tests.
#[cfg(test)]
fn tag_counts(conn: &Connection) -> std::collections::BTreeMap<String, usize> {
    list_tags(conn)
        .unwrap()
        .into_iter()
        .map(|tag| (tag.label, tag.games))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_groups_the_accounts_of_a_player() {
        let dir = std::env::temp_dir().join(format!("swarmy-db-accounts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let conn = open_db(&dir.join(DB_FILE_NAME)).unwrap();
        let change = |account: &str, grouped: bool| PlayerAccountChange {
            player: "Clem".to_string(),
            account: account.to_string(),
            grouped,
        };
        change_player_account(&conn, &change("2-1-3141896", true)).unwrap();
        change_player_account(&conn, &change("1-1-4516738", true)).unwrap();
        // Grouping again is not an error.
        change_player_account(&conn, &change("1-1-4516738", true)).unwrap();
        change_player_account(&conn, &change("2-1-3141896", false)).unwrap();
        let groups = player_groups(&conn).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups["1-1-4516738"], "Clem");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn it_keeps_the_notes_ratings_and_tags() {
        let dir = std::env::temp_dir().join(format!("swarmy-db-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut conn = open_db(&dir.join(DB_FILE_NAME)).unwrap();
        let notes = ReplayNotes {
            sha256: "abc".to_string(),
            rating: Some(4),
            note: "Great comeback".to_string(),
            tags: vec![
                "finals".to_string(),
                " comeback ".to_string(),
                String::new(),
            ],
        };
        set_notes(&mut conn, &notes).unwrap();
        let saved = get_notes(&conn, "abc").unwrap();
        assert_eq!(saved.rating, Some(4));
        assert_eq!(saved.note, "Great comeback");
        assert_eq!(saved.tags, vec!["comeback", "finals"]);
        set_notes(
            &mut conn,
            &ReplayNotes {
                sha256: "def".to_string(),
                rating: Some(2),
                tags: vec!["finals".to_string()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(tag_counts(&conn)["finals"], 2);
        let finals = filter_replays(&conn, "finals", Some(3)).unwrap().unwrap();
        assert_eq!(finals, HashSet::from(["abc".to_string()]));
        assert_eq!(filter_replays(&conn, "", None).unwrap(), None);
        let summaries = notes_summaries(&conn, &["abc".to_string()]).unwrap();
        assert_eq!(summaries["abc"].0, Some(4));
        assert!(!summaries.contains_key("def"));
        // The database checkpoints when it is closed, its directory must still exist then.
        conn.close().map_err(|(_, err)| err).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
