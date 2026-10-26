//! The replays that are copies of another replay, by the sha256 of their contents. They are moved
//! to the duplicate replays folder of the settings, outside of the replay path, keeping the
//! directories they were found in. Nothing is deleted.

use crate::data::*;
use crate::settings::load_stored_app_settings;
use polars::prelude::*;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use swarmy_common::*;

/// The replays found are counted up to this many files, like the Optimize scan.
const MAX_FILES: usize = 1_000_000;
const MAX_DEPTH: usize = 8;

/// The replays of the snapshot by sha256, and the file names in the snapshot.
fn snapshot_replays(
    replay_path: &str,
) -> Result<(HashMap<String, String>, HashSet<String>), SwarmyError> {
    let ipc_path = Path::new(replay_path).join(IPC_DIR);
    if !ipc_path.is_dir() {
        return Ok(Default::default());
    }
    let details = scan_snapshot_ipc(&ipc_path.to_string_lossy(), DETAILS_IPC)?
        .select([col("ext_fs_sha256"), col("ext_fs_file_name")])
        .unique(None, UniqueKeepStrategy::First)
        .collect()?;
    let sha256s = str_values(&details, "ext_fs_sha256")?;
    let file_names = str_values(&details, "ext_fs_file_name")?;
    let mut by_sha256 = HashMap::new();
    for (sha256, file_name) in sha256s.into_iter().zip(&file_names) {
        by_sha256.entry(sha256).or_insert_with(|| file_name.clone());
    }
    Ok((by_sha256, file_names.into_iter().collect()))
}

/// The replays of the replay path that have the same contents as a replay of the snapshot, or as
/// another replay found before them.
pub fn find_duplicates(replay_path: &str) -> Result<Vec<DuplicateReplay>, SwarmyError> {
    let (mut originals, in_snapshot) = snapshot_replays(replay_path)?;
    let mut candidates: Vec<PathBuf> =
        s2protocol::get_matching_files(PathBuf::from(replay_path), MAX_FILES, MAX_DEPTH)?
            .into_iter()
            .filter(|path| !in_snapshot.contains(path.to_str().unwrap_or_default()))
            .collect();
    // The first copy found of a replay that is not in the snapshot is the original.
    candidates.sort();
    let hashed: Vec<(PathBuf, String)> = candidates
        .into_par_iter()
        .filter_map(|path| {
            let contents = std::fs::read(&path).ok()?;
            Some((path, sha256::digest(contents.as_slice())))
        })
        .collect();
    let mut duplicates = vec![];
    for (path, sha256) in hashed {
        let file = path.to_string_lossy().to_string();
        match originals.get(&sha256) {
            Some(original) => duplicates.push(DuplicateReplay {
                file,
                original: original.clone(),
            }),
            None => {
                originals.insert(sha256, file);
            }
        }
    }
    Ok(duplicates)
}

/// Where a duplicate goes in the duplicates folder, the same directories as in the replay path.
fn duplicate_destination(replay_path: &Path, folder: &Path, file: &Path) -> Option<PathBuf> {
    let relative = file.strip_prefix(replay_path).ok()?;
    let mut destination = folder.join(relative);
    // A file already in the duplicates folder is not replaced, the new one gets a number.
    let mut copy = 1;
    while destination.exists() {
        let stem = file.file_stem()?.to_string_lossy();
        let extension = file.extension()?.to_string_lossy();
        destination = folder
            .join(relative.parent()?)
            .join(format!("{stem} ({copy}).{extension}"));
        copy += 1;
    }
    Some(destination)
}

/// Moves a file, copying it when the duplicates folder is on another file system.
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir)?;
    }
    if std::fs::rename(from, to).is_err() {
        std::fs::copy(from, to)?;
        std::fs::remove_file(from)?;
    }
    Ok(())
}

/// Moves the duplicates of the replay path to the duplicates folder, returns the files moved.
pub fn move_duplicates(replay_path: &str, folder: &str) -> Result<usize, SwarmyError> {
    if folder.trim().is_empty() {
        return Err(SwarmyError::Other(
            "Set the Duplicate Replays Folder in Config first".to_string(),
        ));
    }
    let replay_dir = std::fs::canonicalize(replay_path)?;
    std::fs::create_dir_all(folder)?;
    let duplicates_dir = std::fs::canonicalize(folder)?;
    if duplicates_dir.starts_with(&replay_dir) {
        return Err(SwarmyError::Other(format!(
            "The duplicate replays folder {} must be outside of the replay path {}",
            duplicates_dir.display(),
            replay_dir.display()
        )));
    }
    let mut moved = 0;
    for duplicate in find_duplicates(replay_path)? {
        let file = PathBuf::from(&duplicate.file);
        let Some(destination) =
            duplicate_destination(Path::new(replay_path), &duplicates_dir, &file)
        else {
            tracing::warn!("{} is not in the replay path", file.display());
            continue;
        };
        move_file(&file, &destination)?;
        tracing::info!("Moved {} to {}", file.display(), destination.display());
        moved += 1;
    }
    Ok(moved)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn find_duplicate_replays(
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse, SwarmyError> {
    let settings = load_stored_app_settings(&app_handle)?;
    crate::run_query(move || {
        ApiResponseBuilder::new().process_result(find_duplicates(&settings.replay_path))
    })
    .await
}

#[tauri::command(rename_all = "snake_case")]
pub async fn move_duplicate_replays(
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse, SwarmyError> {
    let settings = load_stored_app_settings(&app_handle)?;
    let replay_path = settings.replay_path.clone();
    let response = crate::run_query(move || {
        ApiResponseBuilder::new().process_result(move_duplicates(
            &settings.replay_path,
            &settings.duplicates_path,
        ))
    })
    .await;
    // The moved replays are no longer skipped by Optimize nor new.
    crate::replay_watcher::record_skipped_replays(&replay_path);
    crate::replay_watcher::refresh(&app_handle);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_moves_the_duplicates_to_their_folder_with_their_directories() {
        let root = std::env::temp_dir().join(format!("swarmy-duplicates-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let replays = root.join("replays");
        let folder = root.join("duplicates");
        std::fs::create_dir_all(replays.join("a")).unwrap();
        std::fs::create_dir_all(replays.join("b/c")).unwrap();
        std::fs::write(replays.join("a/game.SC2Replay"), b"game one").unwrap();
        std::fs::write(replays.join("b/c/game.SC2Replay"), b"game one").unwrap();
        std::fs::write(replays.join("b/other.SC2Replay"), b"game two").unwrap();
        let replay_path = replays.to_string_lossy().to_string();
        let duplicates = find_duplicates(&replay_path).unwrap();
        assert_eq!(duplicates.len(), 1);
        assert!(duplicates[0].file.ends_with("b/c/game.SC2Replay"));
        assert!(duplicates[0].original.ends_with("a/game.SC2Replay"));
        // The duplicates folder can not be in the replay path.
        let inside = replays.join("duplicates").to_string_lossy().to_string();
        assert!(move_duplicates(&replay_path, &inside).is_err());
        let moved = move_duplicates(&replay_path, &folder.to_string_lossy()).unwrap();
        assert_eq!(moved, 1);
        assert!(folder.join("b/c/game.SC2Replay").is_file());
        assert!(!replays.join("b/c/game.SC2Replay").exists());
        assert!(replays.join("a/game.SC2Replay").is_file());
        assert!(find_duplicates(&replay_path).unwrap().is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
