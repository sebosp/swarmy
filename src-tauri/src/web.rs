//! The browser version of swarmy, served by `swarmy --serve` with the same Leptos frontend of the
//! app: `public/web_bridge.js` answers its Tauri calls with the routes of `server`.
//!
//! Anyone that reaches the server can read the views. The commands that change or reveal the server
//! need the administrator key: the key file holds its sha512, it is read on each request so that
//! rotating the key needs no restart. See docs/Installation.md#browser-version.

use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use swarmy_common::{
    default_max_table_rows, default_reduce_motion, default_theme, default_zoom_percent,
    AppSettings, SnapshotStats,
};

/// The header of the browser with the administrator key.
pub const ADMIN_KEY_HEADER: &str = "x-swarmy-admin-key";

/// The bytes of a sha512 digest in hex.
const SHA512_HEX_LEN: usize = 128;

/// The commands of the backend (`commands::BACKEND_COMMANDS`) that need the administrator: they write in the server, or
/// tell what is private to it (the personal notes, the files it has).
pub const ADMIN_COMMANDS: &[&str] = &[
    // The credentials of the Battle.net application of the server.
    "get_battlenet_status",
    "set_battlenet_credentials",
    "basic_scan_replay_path",
    "optimize_replay_path",
    "get_new_replays_status",
    "get_replay_notes",
    "set_replay_notes",
    "list_replay_tags",
    "set_player_accounts",
    "find_duplicate_replays",
    "move_duplicate_replays",
    "set_progress_goals",
];

/// The commands of the backend that anyone can run, they read the snapshot. A new command
/// has to be in one of the lists, a test checks it, so that it is not public by accident.
pub const PUBLIC_COMMANDS: &[&str] = &[
    // The public profiles of Battle.net, with the credentials of the server.
    "query_battlenet_profile",
    "get_snapshot_metadata",
    "query_map_stats",
    "query_siege_spots",
    "query_map_layers",
    "query_replay_list",
    "list_replay_filters",
    "query_replay_detail",
    "query_adjacent_replay",
    "render_replay_report",
    "query_player_detail",
    "list_players",
    "query_head_to_head",
    "query_map_scout",
    "query_map_details",
    "list_maps",
    "query_unit_snapshot_stats",
    "query_ability_usage",
    "query_ability_detail",
    "query_race_detail",
    "query_usage_over_time",
    "query_progress",
];

/// The commands of the app that use no snapshot nor window, the server runs them for the browser.
pub const STATIC_COMMANDS: &[&str] = &[
    "list_unit_icons",
    "list_patch_units",
    "query_unit_overview",
    "query_patch_changes",
    "list_evolution_items",
    "query_patch_evolution",
];

/// The sha512 of a key, in lowercase hex as `sha512sum` writes it.
pub fn sha512_hex(key: &str) -> String {
    Sha512::digest(key.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The digest of a key file, the first word of it: the output of `sha512sum` works as it is.
pub fn parse_key_file(contents: &str) -> Option<String> {
    let digest = contents.split_whitespace().next()?.to_lowercase();
    (digest.len() == SHA512_HEX_LEN && digest.chars().all(|c| c.is_ascii_hexdigit()))
        .then_some(digest)
}

/// Compares two strings in a time that does not tell how much of them matches.
fn same_digest(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .fold(0u8, |diff, (l, r)| diff | (l ^ r))
            == 0
}

/// Whether the key is the one of the file. A file that is missing or malformed refuses every key.
pub fn key_matches(key_file: &Path, key: &str) -> bool {
    let digest = match std::fs::read_to_string(key_file) {
        Ok(contents) => parse_key_file(&contents),
        Err(err) => {
            tracing::error!("Unable to read the key file {}: {err}", key_file.display());
            None
        }
    };
    match digest {
        Some(digest) => same_digest(&digest, &sha512_hex(key)),
        None => {
            tracing::error!(
                "The key file {} has no sha512, every administrator request is refused",
                key_file.display()
            );
            false
        }
    }
}

/// The settings the browser may see: the appearance and the snapshot, not the paths or tokens of
/// the server. The snapshot statistics are the ones of `get_snapshot_metadata`.
pub fn public_settings(mut snapshot_stats: SnapshotStats) -> AppSettings {
    snapshot_stats.replay_path = String::new();
    AppSettings {
        snapshot_stats,
        zoom_percent: default_zoom_percent(),
        theme: default_theme(),
        reduce_motion: default_reduce_motion(),
        max_table_rows: default_max_table_rows(),
        ..AppSettings::default()
    }
}

/// The settings of the administrator: also what Scan and Optimize use, never the tokens.
pub fn admin_settings(settings: AppSettings, snapshot_stats: SnapshotStats) -> AppSettings {
    let replay_path = snapshot_stats.replay_path.clone();
    let mut admin = public_settings(snapshot_stats);
    admin.snapshot_stats.replay_path = replay_path;
    AppSettings {
        replay_path: settings.replay_path,
        cache_path: settings.cache_path,
        duplicates_path: settings.duplicates_path,
        disable_parallelism: settings.disable_parallelism,
        main_player_name: settings.main_player_name,
        ..admin
    }
}

/// Removes the directories of the server from a response for someone that is not its
/// administrator, the file names of the replays stay relative to the replay path.
pub fn strip_server_paths(message: String, settings: &AppSettings) -> String {
    [&settings.replay_path, &settings.cache_path]
        .into_iter()
        .filter(|path| !path.is_empty())
        .fold(message, |message, path| {
            let path = path.trim_end_matches('/');
            message.replace(&format!("{path}/"), "").replace(path, "")
        })
}

/// The files of the browser version: those of the frontend inside the binary, or a directory when
/// `--web-root` is set, i.e. while developing it.
#[derive(Debug, Default, Clone)]
pub struct WebAssets {
    pub embedded: HashMap<String, Vec<u8>>,
    pub root: Option<PathBuf>,
}

impl WebAssets {
    /// The frontend of the binary: an app built without it (`cargo build`) has none.
    pub fn from_context<R: tauri::Runtime>(context: &tauri::Context<R>) -> Self {
        let assets = context.assets();
        let keys: Vec<String> = assets.iter().map(|(key, _)| key.to_string()).collect();
        let embedded = keys
            .into_iter()
            .filter_map(|key| {
                let asset_key = tauri::utils::assets::AssetKey::from(key.as_str());
                let contents = assets.get(&asset_key)?.into_owned();
                Some((key, contents))
            })
            .collect();
        Self {
            embedded,
            root: None,
        }
    }

    /// The contents of the file of a URL path, `/` is the index. A path out of the root is refused.
    pub fn read(&self, url_path: &str) -> Option<Vec<u8>> {
        let path = match url_path.trim_start_matches('/') {
            "" => "index.html",
            // The browsers ask for it without a link.
            "favicon.ico" => "icon.ico",
            path => path,
        };
        if path
            .split('/')
            .any(|part| part == ".." || part.contains('\\'))
        {
            return None;
        }
        if let Some(root) = &self.root {
            return std::fs::read(root.join(path)).ok();
        }
        self.embedded.get(&format!("/{path}")).cloned()
    }
}

/// The content type of a file of the frontend.
pub fn content_type(url_path: &str) -> &'static str {
    match url_path.rsplit('.').next().unwrap_or_default() {
        "wasm" => "application/wasm",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "dds" => "image/vnd-ms.dds",
        "woff2" => "font/woff2",
        _ => "text/html; charset=utf-8",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::BACKEND_COMMANDS;

    #[test]
    fn it_classifies_every_command_of_the_backend() {
        for command in BACKEND_COMMANDS {
            assert!(
                ADMIN_COMMANDS.contains(command) != PUBLIC_COMMANDS.contains(command),
                "{command} must be in exactly one of ADMIN_COMMANDS and PUBLIC_COMMANDS"
            );
        }
        for command in ADMIN_COMMANDS.iter().chain(PUBLIC_COMMANDS) {
            assert!(
                BACKEND_COMMANDS.contains(command),
                "{command} does not exist"
            );
        }
    }

    /// Every command the frontend calls must work in the browser: the server runs it, or
    /// web_bridge.js answers it. A new one fails here until it is decided which.
    #[test]
    fn it_answers_every_command_of_the_frontend_in_the_browser() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let bridge = std::fs::read_to_string(root.join("public/web_bridge.js")).unwrap();
        let mut sources = vec![root.join("src")];
        let mut missing = vec![];
        while let Some(path) = sources.pop() {
            for entry in std::fs::read_dir(path).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    sources.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let source = std::fs::read_to_string(&path).unwrap();
                    for command in frontend_commands(&source) {
                        let quoted = format!("{command}:");
                        if !BACKEND_COMMANDS.contains(&command.as_str())
                            && !STATIC_COMMANDS.contains(&command.as_str())
                            && !bridge.contains(&quoted)
                        {
                            missing.push(format!("{command} in {}", path.display()));
                        }
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "Not answered in the browser: {missing:?}"
        );
    }

    /// The first string after each call of a backend command in the source of the frontend.
    fn frontend_commands(source: &str) -> Vec<String> {
        let mut commands = vec![];
        for call in [
            "invoke_json::<",
            "invoke_response(",
            "invoke_without_args(",
            "invoke(",
        ] {
            for part in source.split(call).skip(1) {
                let Some(start) = part.find('"') else {
                    continue;
                };
                // Only the first argument, not a string further away in the code.
                if part[..start].contains(';') || part[..start].contains('{') {
                    continue;
                }
                let name: String = part[start + 1..]
                    .chars()
                    .take_while(|c| *c != '"')
                    .collect();
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                    commands.push(name);
                }
            }
        }
        commands
    }

    #[test]
    fn it_finds_the_commands_of_a_source() {
        let source = r#"
            invoke_json::<Vec<u8>>("list_maps", &()).await;
            crate::invoke_without_args("get_ui_settings").await;
            invoke_response("zoom_step", &ZoomStepArgs { steps }).await;
        "#;
        let commands = frontend_commands(source);
        for command in ["list_maps", "get_ui_settings", "zoom_step"] {
            assert!(commands.contains(&command.to_string()), "{commands:?}");
        }
    }

    #[test]
    fn it_checks_the_key_against_the_file_on_each_call() {
        let dir = std::env::temp_dir().join(format!("swarmy-key-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("admin-key.sha512");
        assert!(
            !key_matches(&file, "first"),
            "without a file nothing passes"
        );
        // As `printf %s first | sha512sum > file` writes it.
        std::fs::write(&file, format!("{}  -\n", sha512_hex("first"))).unwrap();
        assert!(key_matches(&file, "first"));
        assert!(!key_matches(&file, "second"));
        // Rotating the key takes effect in the next request.
        std::fs::write(&file, sha512_hex("second")).unwrap();
        assert!(!key_matches(&file, "first"));
        assert!(key_matches(&file, "second"));
        std::fs::write(&file, "not a digest").unwrap();
        assert!(!key_matches(&file, "not"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn it_matches_sha512sum() {
        assert_eq!(
            sha512_hex("abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn it_hides_the_settings_and_paths_of_the_server() {
        let settings = AppSettings {
            replay_path: "/home/me/replays".to_string(),
            cache_path: "/home/me/caches/".to_string(),
            remote_backend_token: "secret".to_string(),
            main_player_name: "me".to_string(),
            ..AppSettings::default()
        };
        let stats = SnapshotStats {
            replay_path: "/home/me/replays".to_string(),
            num_games: 7,
            ..SnapshotStats::default()
        };
        let public = public_settings(stats.clone());
        assert!(public.replay_path.is_empty());
        assert!(public.cache_path.is_empty());
        assert!(public.snapshot_stats.replay_path.is_empty());
        assert_eq!(public.snapshot_stats.num_games, 7);
        assert!(public.remote_backend_token.is_empty());
        assert!(public.main_player_name.is_empty());
        // The defaults of the app, not the empty ones.
        assert_eq!(public.zoom_percent, default_zoom_percent());
        assert_eq!(public.theme, default_theme());
        assert_eq!(public.max_table_rows, default_max_table_rows());
        let admin = admin_settings(settings.clone(), stats);
        assert_eq!(admin.replay_path, "/home/me/replays");
        assert_eq!(admin.snapshot_stats.replay_path, "/home/me/replays");
        assert!(admin.remote_backend_token.is_empty());
        assert_eq!(
            strip_server_paths(
                r#"{"file":"/home/me/replays/GSL/a.SC2Replay","c":"/home/me/caches/x"}"#
                    .to_string(),
                &settings
            ),
            r#"{"file":"GSL/a.SC2Replay","c":"x"}"#
        );
    }

    #[test]
    fn it_serves_only_the_files_of_the_root() {
        let dir = std::env::temp_dir().join(format!("swarmy-web-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("public")).unwrap();
        std::fs::write(dir.join("index.html"), "index").unwrap();
        std::fs::write(dir.join("public/a.js"), "a").unwrap();
        let assets = WebAssets {
            embedded: HashMap::new(),
            root: Some(dir.clone()),
        };
        assert_eq!(assets.read("/").unwrap(), b"index");
        assert_eq!(assets.read("/public/a.js").unwrap(), b"a");
        assert!(assets.read("/../secret").is_none());
        assert!(assets.read("/public/../../secret").is_none());
        assert!(assets.read("/missing").is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
