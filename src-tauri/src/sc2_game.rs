//! Plays a replay in StarCraft II itself, offline: no Battle.net, no login, while the overlay
//! (`sc2_replay` of the frontend) shows the statistics of the moment.
//!
//! The game is started with its API (`-listen`, as python-sc2 does), the replay through the API
//! with real time off: a thread of this module steps it at the speed chosen in the overlay (play,
//! pause, speed, jumps; back, the replay starts again). The game's own controls do not work while
//! the API drives a replay (2026-11-21: in real time its pause stuck and its timeline crashed the
//! game with "e_success", stepped too): the overlay has the controls. The game refuses to draw
//! during a replay ("Not allowed during replays"): the overlay is a window of swarmy on top.
//!
//! - Windows: the game of the StarCraft II folder (`sc2_game_dir`) is started directly.
//! - Linux: the Windows game of a Wine prefix (the one Battle.net is installed in, i.e. through
//!   Lutris) is started by `umu-run` with a Proton. Proton's container only sees the prefix and
//!   the home folder: the replay is copied under `drive_c/swarmy-replays`.
//!
//! - The game started is the version of the replay (`Versions/Base<build>`): Battle.net downloads
//!   an old version the first time a replay of it is opened in the game. An old version needs its
//!   data version (`-dataVersion`, the table of python-sc2 in `assets/sc2_data_versions.json`), it
//!   stops on "installation is damaged" otherwise.
//! - The map caches of the Map Caches Path are copied into the cache of Battle.net, where the game
//!   looks for them. A game on a local map (AI Arena) has no cache: its `.SC2Map` is looked for in
//!   the `Maps` folder of the game and sent with the request.
//! - Battle.net, its launch options and the account are never touched.
//!
//! The messages of the API are protobuf (s2clientprotocol `sc2api.proto`), the few fields used
//! here are encoded by hand.

use swarmy_common::*;
use tauri::AppHandle;

/// The label of the overlay window.
pub const OVERLAY_WINDOW: &str = "sc2-overlay";

fn answer(result: Result<String, String>) -> ApiResponse {
    let (status, message) = match result {
        Ok(message) => (true, message),
        Err(message) => {
            tracing::warn!("StarCraft II: {message}");
            (false, message)
        }
    };
    ApiResponseBuilder::new()
        .with_status(status)
        .with_message(message)
        .build()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn unsupported() -> Result<String, String> {
    Err("Watching a replay in StarCraft II is on Linux and Windows only".to_string())
}

/// Plays the replay in StarCraft II, in its own window, with the overlay on top: the game's own
/// controls move the replay, the overlay follows. `ext_fs_id` is the replay in the snapshot, for
/// the overlay. A failure is an answer with its message, shown next to the button.
#[tauri::command(rename_all = "snake_case")]
pub async fn exec_sc2_replay(
    app_handle: AppHandle,
    replay_file_name: String,
    ext_fs_id: Option<u64>,
) -> Result<ApiResponse, SwarmyError> {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let result = launch::exec_sc2_replay(app_handle.clone(), replay_file_name, ext_fs_id).await;
    // The overlay comes with the game: swarmy is behind it, its window is hard to reach then.
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if result.is_ok()
        && let Err(err) = open_overlay(&app_handle)
    {
        tracing::warn!("{err}");
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let result = {
        let _ = (app_handle, replay_file_name, ext_fs_id);
        unsupported()
    };
    Ok(answer(result))
}

/// The state of the replay being played, the JSON of [`Sc2ReplayState`] (not `active` without
/// one).
#[tauri::command(rename_all = "snake_case")]
pub async fn get_sc2_replay_state() -> Result<ApiResponse, SwarmyError> {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let state = launch::state();
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let state = Sc2ReplayState::default();
    Ok(answer(
        serde_json::to_string(&state).map_err(|err| err.to_string()),
    ))
}

/// The width of the overlay open (its height follows what it shows), and its size collapsed to
/// its button.
const OVERLAY_WIDTH: f64 = 480.;
/// The height of the overlay when it opens; the user resizes it from its edges or its grip.
const OVERLAY_HEIGHT: f64 = 280.;
/// The smallest overlay open: the controls and a chart.
const OVERLAY_MIN_SIZE: (f64, f64) = (320., 160.);
/// The size the user gave the overlay (logical pixels), kept while it is collapsed.
static OVERLAY_SIZE: std::sync::Mutex<Option<(f64, f64)>> = std::sync::Mutex::new(None);
const OVERLAY_COLLAPSED_SIZE: (f64, f64) = (18., 44.);

/// Opens the overlay on top of the game: a small window without borders, always on top, that
/// shows the statistics of the moment of the replay; it is moved by dragging it and collapses to
/// a button. A window over the whole screen that the clicks go through was invisible on KDE
/// Wayland (2026-11-21): it went to another screen or drew nothing, and took the clicks of swarmy.
/// Only where the game runs: Android has no such window (the APK of 0.2.6 did not build).
#[cfg(any(target_os = "linux", target_os = "windows"))]
fn open_overlay(app_handle: &AppHandle) -> Result<String, String> {
    use tauri::Manager;
    // The page shows the window once it has something to show (`set_sc2_overlay_layout`).
    if app_handle.get_webview_window(OVERLAY_WINDOW).is_some() {
        return Ok("Overlay open".to_string());
    }
    let builder = tauri::WebviewWindowBuilder::new(
        app_handle,
        OVERLAY_WINDOW,
        tauri::WebviewUrl::App("index.html#/sc2-overlay".into()),
    )
    .title("swarmy overlay")
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .visible(false)
    .resizable(true)
    .decorations(false)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .transparent(true);
    builder
        .build()
        .map(|_| "Overlay open on top of the game, drag it where you want it".to_string())
        .map_err(|err| format!("Unable to open the overlay: {err}"))
}

/// Shows or hides the overlay, collapses it to its button or opens it again at the size the user
/// gave it (resized with the mouse, the page does not size it). Its right side stays in place
/// where the system lets a window move itself (Windows, X11; Wayland places it itself).
#[tauri::command(rename_all = "snake_case")]
pub async fn set_sc2_overlay_layout(
    app_handle: AppHandle,
    visible: bool,
    collapsed: bool,
) -> Result<ApiResponse, SwarmyError> {
    use tauri::Manager;
    let Some(window) = app_handle.get_webview_window(OVERLAY_WINDOW) else {
        return Ok(answer(Err("The overlay is closed".to_string())));
    };
    if !visible {
        return Ok(answer(
            window
                .hide()
                .map(|_| String::new())
                .map_err(|err| err.to_string()),
        ));
    }
    let scale = window.scale_factor().unwrap_or(1.);
    let before = window.outer_position().ok().zip(window.outer_size().ok());
    let current = window
        .inner_size()
        .ok()
        .map(|size| (size.width as f64 / scale, size.height as f64 / scale));
    let is_collapsed = |(width, _): (f64, f64)| width <= OVERLAY_COLLAPSED_SIZE.0 + 1.;
    let mut kept = OVERLAY_SIZE.lock().unwrap_or_else(|err| err.into_inner());
    if let Some(size) = current.filter(|size| !is_collapsed(*size)) {
        *kept = Some(size);
    }
    let (width, height) = if collapsed {
        OVERLAY_COLLAPSED_SIZE
    } else {
        kept.unwrap_or((OVERLAY_WIDTH, OVERLAY_HEIGHT))
    };
    drop(kept);
    // Always resizable: on Linux a window made not resizable keeps the size it has then, the
    // collapsed overlay stayed as big as the open one (2026-11-21). The smallest size follows.
    let min_size = if collapsed {
        OVERLAY_COLLAPSED_SIZE
    } else {
        OVERLAY_MIN_SIZE
    };
    let result = window
        .set_min_size(Some(tauri::LogicalSize::new(min_size.0, min_size.1)))
        .and_then(|_| window.set_size(tauri::LogicalSize::new(width, height)))
        .and_then(|_| window.show())
        .map_err(|err| err.to_string());
    if let Some((position, size)) = before
        && (size.width as f64 - width * scale).abs() > 1.
    {
        let right = position.x + size.width as i32;
        let x = right - (width * scale) as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, position.y));
    }
    Ok(answer(result.map(|_| String::new())))
}

/// Play, pause, speed or a jump in time, for the replay being played.
#[tauri::command(rename_all = "snake_case")]
pub async fn sc2_replay_control(control: Sc2ReplayControl) -> Result<ApiResponse, SwarmyError> {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    let result = launch::control(control);
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    let result = {
        let _ = control;
        unsupported()
    };
    Ok(answer(result))
}

/// The base build and data version of each version of the game (python-sc2's table).
const DATA_VERSIONS: &str = include_str!("../../assets/sc2_data_versions.json");

/// The data version of a base build, None for the builds the table does not have: the recent
/// ones start without it.
pub fn data_version(base_build: u32) -> Option<String> {
    let table: std::collections::HashMap<String, String> =
        serde_json::from_str(DATA_VERSIONS).ok()?;
    table.get(&base_build.to_string()).cloned()
}

/// The Wine prefix: the one of Config, or `~/Games/battlenet` (Lutris' default for Battle.net).
pub fn wine_prefix(settings: &AppSettings) -> std::path::PathBuf {
    if !settings.sc2_wine_prefix.trim().is_empty() {
        return std::path::PathBuf::from(settings.sc2_wine_prefix.trim());
    }
    dirs::home_dir()
        .unwrap_or_default()
        .join("Games")
        .join("battlenet")
}

/// The Proton umu runs the game with: the one of Config, or the first Proton of the system's
/// Steam compatibility tools; None lets umu use its own (downloaded the first time).
pub fn proton_path(settings: &AppSettings) -> Option<std::path::PathBuf> {
    if !settings.sc2_proton_path.trim().is_empty() {
        return Some(std::path::PathBuf::from(settings.sc2_proton_path.trim()));
    }
    let mut found: Vec<std::path::PathBuf> =
        std::fs::read_dir("/usr/share/steam/compatibilitytools.d")
            .ok()?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().to_lowercase().contains("proton"))
            })
            .collect();
    found.sort();
    found.into_iter().next()
}

/// The protobuf encoding of the few messages of the API used here.
pub mod proto {
    fn varint(mut value: u64, out: &mut Vec<u8>) {
        while value >= 0x80 {
            out.push((value as u8) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    fn field_varint(field: u32, value: u64, out: &mut Vec<u8>) {
        varint(u64::from(field) << 3, out);
        varint(value, out);
    }

    fn field_bytes(field: u32, bytes: &[u8], out: &mut Vec<u8>) {
        varint((u64::from(field) << 3) | 2, out);
        varint(bytes.len() as u64, out);
        out.extend_from_slice(bytes);
    }

    fn request(field: u32, message: &[u8]) -> Vec<u8> {
        let mut request = vec![];
        field_bytes(field, message, &mut request);
        request
    }

    /// `Request { start_replay: RequestStartReplay { replay_path, observed_player_id: 1,
    /// options: { score: true }, disable_fog: true, realtime: false, map_data } }`: the score
    /// keeps the observations small, the replay only moves when it is stepped.
    pub fn start_replay(replay_path: &str, map_data: Option<&[u8]>) -> Vec<u8> {
        let mut options = vec![];
        field_varint(2, 1, &mut options);
        let mut start = vec![];
        field_bytes(1, replay_path.as_bytes(), &mut start);
        field_varint(2, 1, &mut start);
        field_bytes(3, &options, &mut start);
        field_varint(4, 1, &mut start);
        if let Some(map_data) = map_data {
            field_bytes(6, map_data, &mut start);
        }
        field_varint(7, 0, &mut start);
        request(4, &start)
    }

    /// `Request { step: RequestStep { count } }`.
    pub fn step(count: u32) -> Vec<u8> {
        let mut step = vec![];
        field_varint(1, u64::from(count), &mut step);
        request(12, &step)
    }

    /// `Request { observation: RequestObservation {} }`.
    pub fn observation() -> Vec<u8> {
        request(10, &[])
    }

    /// The fields of a message: (number, varint value or bytes).
    fn fields(mut input: &[u8]) -> Vec<(u32, Result<u64, &[u8]>)> {
        let read_varint = |input: &mut &[u8]| -> Option<u64> {
            let mut value = 0u64;
            for shift in (0..64).step_by(7) {
                let (byte, rest) = input.split_first()?;
                *input = rest;
                value |= u64::from(byte & 0x7f) << shift;
                if byte & 0x80 == 0 {
                    return Some(value);
                }
            }
            None
        };
        let mut result = vec![];
        while let Some(key) = read_varint(&mut input) {
            let (field, wire) = ((key >> 3) as u32, key & 7);
            match wire {
                0 => match read_varint(&mut input) {
                    Some(value) => result.push((field, Ok(value))),
                    None => break,
                },
                2 => {
                    let Some(len) = read_varint(&mut input) else {
                        break;
                    };
                    let len = len as usize;
                    if len > input.len() {
                        break;
                    }
                    result.push((field, Err(&input[..len])));
                    input = &input[len..];
                }
                1 if input.len() >= 8 => input = &input[8..],
                5 if input.len() >= 4 => input = &input[4..],
                _ => break,
            }
        }
        result
    }

    /// The errors of a `Response`: its `error` strings, and the error of its `start_replay`.
    pub fn errors(response: &[u8]) -> Vec<String> {
        const START_REPLAY_ERRORS: [&str; 8] = [
            "",
            "MissingReplay",
            "InvalidReplayPath",
            "InvalidReplayData",
            "InvalidMapData",
            "InvalidObservedPlayerId",
            "MissingOptions",
            "LaunchError",
        ];
        let mut errors = vec![];
        for (field, value) in fields(response) {
            match (field, value) {
                (98, Err(text)) => errors.push(String::from_utf8_lossy(text).to_string()),
                (4, Err(start)) => {
                    for (field, value) in fields(start) {
                        match (field, value) {
                            (1, Ok(code)) => errors.push(
                                START_REPLAY_ERRORS
                                    .get(code as usize)
                                    .map(|name| name.to_string())
                                    .unwrap_or_else(|| format!("error {code}")),
                            ),
                            (2, Err(details)) => {
                                errors.push(String::from_utf8_lossy(details).to_string())
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        errors.retain(|error| !error.is_empty());
        errors
    }

    /// The status of the game in a `Response`: 4 in a replay, 5 when it ended.
    pub fn status(response: &[u8]) -> Option<u64> {
        fields(response)
            .into_iter()
            .find_map(|(field, value)| (field == 99).then_some(value.ok()).flatten())
    }

    /// The status of a replay that ended.
    pub const STATUS_ENDED: u64 = 5;

    /// The game loop of a `Response { observation: { observation: { game_loop } } }`.
    pub fn game_loop(response: &[u8]) -> Option<u32> {
        let message = |input: &[u8], number: u32| {
            fields(input)
                .into_iter()
                .find_map(|(field, value)| (field == number).then_some(value.err()).flatten())
                .map(|bytes| bytes.to_vec())
        };
        let response_observation = message(response, 10)?;
        let observation = message(&response_observation, 3)?;
        fields(&observation)
            .into_iter()
            .find_map(|(field, value)| (field == 9).then_some(value.ok()).flatten())
            .map(|game_loop| game_loop as u32)
    }
}

/// The map file of a game on a local map: a `.SC2Map` under `maps_dir` whose name, letters and
/// digits only, starts with the title of the map, i.e. `IncorporealAIE_v4.SC2Map` for
/// "Incorporeal AIE".
pub fn find_local_map(maps_dir: &std::path::Path, title: &str) -> Option<std::path::PathBuf> {
    let simple = |name: &str| -> String {
        name.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    };
    let title = simple(title);
    if title.is_empty() {
        return None;
    }
    let mut dirs = vec![maps_dir.to_path_buf()];
    let mut found = vec![];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("SC2Map"))
                && path
                    .file_stem()
                    .is_some_and(|stem| simple(&stem.to_string_lossy()).starts_with(&title))
            {
                found.push(path);
            }
        }
    }
    // The latest version of the map last, i.e. _v4 after _v3.
    found.sort();
    found.pop()
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
mod launch {
    use super::*;
    use crate::settings::load_stored_app_settings;
    use std::net::TcpStream;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{Receiver, Sender};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use tungstenite::WebSocket;
    use tungstenite::stream::MaybeTlsStream;

    /// How long the game has to open its API.
    const START_TIMEOUT: Duration = Duration::from_secs(180);
    /// How often the controller steps the replay while it plays.
    const TICK: Duration = Duration::from_millis(40);

    type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

    /// The replay being played: its state, its controls, and the flag that closes the game.
    struct Session {
        stop: Arc<AtomicBool>,
        controls: Sender<Sc2ReplayControl>,
        state: Arc<Mutex<Sc2ReplayState>>,
    }

    static SESSION: Mutex<Option<Session>> = Mutex::new(None);

    pub fn state() -> Sc2ReplayState {
        SESSION
            .lock()
            .ok()
            .and_then(|session| {
                session
                    .as_ref()
                    .and_then(|session| session.state.lock().ok().map(|state| state.clone()))
            })
            .unwrap_or_default()
    }

    pub fn control(control: Sc2ReplayControl) -> Result<String, String> {
        let session = SESSION.lock().map_err(|err| err.to_string())?;
        let session = session
            .as_ref()
            .filter(|session| session.state.lock().is_ok_and(|state| state.active))
            .ok_or("No replay is playing in StarCraft II")?;
        session
            .controls
            .send(control)
            .map(|_| String::new())
            .map_err(|_| "The game is closed".to_string())
    }

    /// Closes the game of the replay being played, if any.
    pub fn stop() {
        if let Ok(session) = SESSION.lock()
            && let Some(session) = session.as_ref()
        {
            session.stop.store(true, Ordering::Relaxed);
        }
    }

    pub async fn exec_sc2_replay(
        app_handle: AppHandle,
        replay_file_name: String,
        ext_fs_id: Option<u64>,
    ) -> Result<String, String> {
        let settings = load_stored_app_settings(&app_handle).map_err(|err| err.to_string())?;
        // With a remote backend the replay and its maps are copied here first, as for swarmy-bevy.
        let (replay, cache_path) = if settings.is_remote() {
            let dir = crate::remote::local_files_dir(&app_handle, &settings)
                .map_err(|err| err.to_string())?;
            let replay =
                crate::remote::download_replay(&settings, &replay_file_name, &dir.join("replays"))
                    .await
                    .map_err(|err| err.to_string())?;
            let caches = dir.join("caches");
            let cache_ids = read_replay(&replay)?.cache_ids;
            crate::remote::download_map_caches(&settings, &cache_ids, &caches)
                .await
                .map_err(|err| err.to_string())?;
            (replay, caches)
        } else {
            (
                PathBuf::from(&replay_file_name),
                PathBuf::from(&settings.cache_path),
            )
        };
        // One game at a time: the one playing closes first.
        stop();
        tauri::async_runtime::spawn_blocking(move || {
            start_session(&settings, &replay, &cache_path, ext_fs_id)
        })
        .await
        .map_err(|err| format!("StarCraft II: {err}"))?
    }

    /// What is needed of the replay to play it.
    struct ReplayInfo {
        base_build: u32,
        duration_loops: u32,
        title: String,
        /// The sha256 of the map caches, empty for a game on a local map.
        cache_ids: Vec<String>,
    }

    fn read_replay(replay: &Path) -> Result<ReplayInfo, String> {
        let name = replay.display().to_string();
        let unreadable = |err: &dyn std::fmt::Display| format!("Unable to read {name}: {err}");
        let (mpq, contents) = s2protocol::read_mpq(&name).map_err(|err| unreadable(&err))?;
        let (_, header) = s2protocol::read_protocol_header(&mpq).map_err(|err| unreadable(&err))?;
        let details =
            s2protocol::read_details(&name, &mpq, &contents).map_err(|err| unreadable(&err))?;
        let init_data =
            s2protocol::read_init_data(&name, &mpq, &contents).map_err(|err| unreadable(&err))?;
        Ok(ReplayInfo {
            base_build: header.m_version.m_base_build,
            duration_loops: header.m_elapsed_game_loops,
            title: details.title,
            cache_ids: init_data
                .sync_lobby_state
                .game_description
                .cache_handles
                .iter()
                .map(|handle| handle[handle.len().saturating_sub(64)..].to_string())
                .collect(),
        })
    }

    /// Where the game is and what it reads, on this system.
    struct Install {
        /// The folder of the game, with `Versions` and `Support64`.
        game_dir: PathBuf,
        /// The cache of Battle.net, where the game finds the maps.
        cache_dir: PathBuf,
        /// Where the replay is copied for the game, and that folder for the game; None when the
        /// game reads it where it is.
        replay_copy: Option<(PathBuf, String)>,
        /// The installation, for the messages.
        name: String,
    }

    #[cfg(target_os = "linux")]
    fn install(settings: &AppSettings) -> Install {
        let prefix = wine_prefix(settings);
        let drive_c = prefix.join("drive_c");
        Install {
            game_dir: drive_c.join("Program Files (x86)/StarCraft II"),
            cache_dir: drive_c.join("ProgramData/Blizzard Entertainment/Battle.net/Cache"),
            replay_copy: Some((
                drive_c.join("swarmy-replays"),
                "C:\\swarmy-replays".to_string(),
            )),
            name: format!("the Wine prefix {}", prefix.display()),
        }
    }

    #[cfg(target_os = "windows")]
    fn install(settings: &AppSettings) -> Install {
        let game_dir = Some(settings.sc2_game_dir.trim())
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("C:\\Program Files (x86)\\StarCraft II"));
        let program_data =
            std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
        Install {
            name: game_dir.display().to_string(),
            game_dir,
            cache_dir: PathBuf::from(program_data)
                .join("Blizzard Entertainment")
                .join("Battle.net")
                .join("Cache"),
            replay_copy: None,
        }
    }

    /// The command that starts the game: through umu on Linux, the game itself on Windows.
    #[cfg(target_os = "linux")]
    fn game_command(settings: &AppSettings, exe: &Path) -> Command {
        let mut command = Command::new("umu-run");
        command
            .arg(exe)
            .env("WINEPREFIX", wine_prefix(settings))
            .env("GAMEID", "umu-default")
            .env("PROTON_VERB", "run")
            .env("WINEDLLOVERRIDES", "locationapi=d")
            // Started by the API the game draws as fast as it can, its interface animations (the
            // portrait of the selection) ran far too fast (2026-11-21): DXVK caps the frames.
            .env("DXVK_FRAME_RATE", "60");
        if let Some(proton) = proton_path(settings) {
            command.env("PROTONPATH", proton);
        }
        command
    }

    #[cfg(target_os = "windows")]
    fn game_command(_settings: &AppSettings, exe: &Path) -> Command {
        Command::new(exe)
    }

    /// Stops the game of a port. umu runs it in a container on Linux: its processes are found by
    /// their command line.
    fn stop_game(child: &mut Child, port: u16) {
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("pkill")
                .args([
                    "-f",
                    &format!("^[A-Z]:.*SC2_x64.exe -listen 127.0.0.1 -port {port} "),
                ])
                .status();
            let _ = Command::new("pkill")
                .args(["-f", "^C:.*BlizzardError"])
                .status();
        }
        #[cfg(target_os = "windows")]
        let _ = port;
        let _ = child.kill();
        let _ = child.wait();
    }

    /// Copies the map caches missing from the cache of Battle.net, where the game looks for
    /// them: `Cache/<2 first>/<2 next>/<sha256>.s2ma`.
    fn copy_map_caches(
        cache_dir: &Path,
        cache_path: &Path,
        cache_ids: &[String],
    ) -> Result<(), String> {
        for id in cache_ids {
            if id.len() < 4 {
                continue;
            }
            let file = format!("{id}.s2ma");
            let target = cache_dir.join(&id[0..2]).join(&id[2..4]).join(&file);
            if target.exists() {
                continue;
            }
            let source = cache_path.join(&file);
            if !source.exists() {
                return Err(format!(
                    "The map cache {file} is not in the Map Caches Path {}: open the replay in swarmy-bevy once to download it",
                    cache_path.display()
                ));
            }
            std::fs::create_dir_all(target.parent().unwrap_or(cache_dir))
                .and_then(|_| std::fs::copy(&source, &target))
                .map_err(|err| {
                    format!("Unable to copy {file} to {}: {err}", cache_dir.display())
                })?;
        }
        Ok(())
    }

    fn free_port() -> Result<u16, String> {
        std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .map(|address| address.port())
            .map_err(|err| format!("No free port for the game's API: {err}"))
    }

    /// Sends a request and reads its answer, the errors of the answer are an error.
    fn call(socket: &mut Socket, request: Vec<u8>) -> Result<Vec<u8>, String> {
        socket
            .send(tungstenite::Message::binary(request))
            .map_err(|err| err.to_string())?;
        loop {
            match socket.read().map_err(|err| err.to_string())? {
                tungstenite::Message::Binary(bytes) => {
                    let errors = proto::errors(&bytes);
                    if !errors.is_empty() {
                        return Err(errors.join(", "));
                    }
                    return Ok(bytes.to_vec());
                }
                tungstenite::Message::Close(_) => return Err("the game closed".to_string()),
                _ => continue,
            }
        }
    }

    fn start_session(
        settings: &AppSettings,
        replay: &Path,
        cache_path: &Path,
        ext_fs_id: Option<u64>,
    ) -> Result<String, String> {
        let info = read_replay(replay)?;
        let install = install(settings);
        let base = format!("Base{}", info.base_build);
        let exe = install
            .game_dir
            .join("Versions")
            .join(&base)
            .join("SC2_x64.exe");
        if !exe.exists() {
            return Err(format!(
                "StarCraft II {base} is not installed in {}: open a replay of this version once in the game, Battle.net downloads it",
                install.name
            ));
        }
        let map_data = if info.cache_ids.is_empty() {
            let maps = install.game_dir.join("Maps");
            let map = find_local_map(&maps, &info.title).ok_or_else(|| {
                format!(
                    "The replay was played on a local map ({}), put its .SC2Map in {}",
                    info.title,
                    maps.display()
                )
            })?;
            Some(
                std::fs::read(&map)
                    .map_err(|err| format!("Unable to read {}: {err}", map.display()))?,
            )
        } else {
            copy_map_caches(&install.cache_dir, cache_path, &info.cache_ids)?;
            None
        };
        let file_name = replay
            .file_name()
            .ok_or("The replay has no file name")?
            .to_string_lossy()
            .to_string();
        let replay_path = match &install.replay_copy {
            Some((dir, game_dir)) => {
                std::fs::create_dir_all(dir)
                    .and_then(|_| std::fs::copy(replay, dir.join(&file_name)))
                    .map_err(|err| {
                        format!("Unable to copy the replay to {}: {err}", dir.display())
                    })?;
                format!("{game_dir}\\{file_name}")
            }
            None => replay.display().to_string(),
        };

        let port = free_port()?;
        let mut command = game_command(settings, &exe);
        command
            .args(["-listen", "127.0.0.1", "-port", &port.to_string()])
            .args([
                "-displayMode",
                "0",
                "-windowwidth",
                "1280",
                "-windowheight",
                "800",
            ])
            .current_dir(install.game_dir.join("Support64"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(version) = data_version(info.base_build) {
            command.args(["-dataVersion", &version]);
        }
        tracing::info!("Starting StarCraft II {base} on port {port}: {command:?}");
        let mut child = command.spawn().map_err(|err| {
            format!(
                "Unable to start StarCraft II ({:?}): {err}",
                command.get_program()
            )
        })?;

        let started = Instant::now();
        let mut socket = loop {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!(
                    "StarCraft II {base} stopped before opening its API: {status}"
                ));
            }
            match tungstenite::connect(format!("ws://127.0.0.1:{port}/sc2api")) {
                Ok((socket, _)) => break socket,
                Err(_) if started.elapsed() < START_TIMEOUT => {
                    std::thread::sleep(Duration::from_millis(500))
                }
                Err(err) => {
                    stop_game(&mut child, port);
                    return Err(format!(
                        "StarCraft II {base} did not open its API in {}s: {err}",
                        START_TIMEOUT.as_secs()
                    ));
                }
            }
        };
        let start = proto::start_replay(&replay_path, map_data.as_deref());
        if let Err(err) = call(&mut socket, start.clone()) {
            stop_game(&mut child, port);
            return Err(format!(
                "StarCraft II {base} did not start the replay: {err}"
            ));
        }
        let state = Arc::new(Mutex::new(Sc2ReplayState {
            active: true,
            replay_file_name: replay.display().to_string(),
            ext_fs_id,
            duration_loops: info.duration_loops,
            speed: 1.,
            ..Default::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (controls, received) = std::sync::mpsc::channel();
        if let Ok(mut session) = SESSION.lock() {
            *session = Some(Session {
                stop: stop.clone(),
                controls,
                state: state.clone(),
            });
        }
        let controller = Controller {
            socket,
            child,
            port,
            start,
            state,
            stop,
        };
        std::thread::spawn(move || controller.run(received));
        Ok(format!(
            "StarCraft II {base} is open: control via the overlay"
        ))
    }

    /// Steps the replay at the speed chosen and answers the controls, until the game closes.
    struct Controller {
        socket: Socket,
        child: Child,
        port: u16,
        /// The request that starts the replay, again for a jump back.
        start: Vec<u8>,
        state: Arc<Mutex<Sc2ReplayState>>,
        stop: Arc<AtomicBool>,
    }

    impl Controller {
        fn update(&self, change: impl FnOnce(&mut Sc2ReplayState)) {
            if let Ok(mut state) = self.state.lock() {
                change(&mut state);
            }
        }

        fn read(&self) -> Sc2ReplayState {
            self.state
                .lock()
                .map(|state| state.clone())
                .unwrap_or_default()
        }

        /// Steps `count` loops and reads where the replay is: the step is answered at once, the
        /// observation once the game computed the loops.
        fn step(&mut self, count: u32) -> Result<(), String> {
            let answer = call(&mut self.socket, proto::step(count))?;
            let observation = call(&mut self.socket, proto::observation())?;
            let game_loop = proto::game_loop(&observation);
            let ended = [&answer, &observation]
                .iter()
                .any(|response| proto::status(response) == Some(proto::STATUS_ENDED));
            self.update(|state| {
                if let Some(game_loop) = game_loop {
                    state.game_loop = game_loop;
                }
                if ended {
                    state.ended = true;
                    state.playing = false;
                }
            });
            Ok(())
        }

        /// Jumps to a second: forward the game computes the way there, back the replay starts
        /// again first.
        fn seek(&mut self, seconds: u32) -> Result<(), String> {
            let target = (seconds as f32 * LOOPS_PER_SECOND) as u32;
            self.update(|state| state.seeking = Some(seconds));
            let current = self.read();
            if target < current.game_loop || current.ended {
                call(&mut self.socket, self.start.clone())?;
                self.update(|state| {
                    state.game_loop = 0;
                    state.ended = false;
                });
            }
            let from = self.read().game_loop;
            let result = if target > from {
                self.step(target - from)
            } else {
                Ok(())
            };
            self.update(|state| state.seeking = None);
            result
        }

        fn answer(&mut self, control: Sc2ReplayControl) -> Result<(), String> {
            match control {
                Sc2ReplayControl::Play => {
                    if self.read().ended {
                        self.seek(0)?;
                    }
                    self.update(|state| state.playing = true);
                }
                Sc2ReplayControl::Pause => self.update(|state| state.playing = false),
                Sc2ReplayControl::Speed { speed } => {
                    self.update(|state| state.speed = speed.clamp(0.25, 32.))
                }
                Sc2ReplayControl::Seek { seconds } => self.seek(seconds)?,
            }
            Ok(())
        }

        fn run(mut self, controls: Receiver<Sc2ReplayControl>) {
            let mut last = Instant::now();
            // The loops owed at this speed, stepped when they make a whole one.
            let mut owed = 0f32;
            while !self.stop.load(Ordering::Relaxed) && matches!(self.child.try_wait(), Ok(None)) {
                while let Ok(control) = controls.try_recv() {
                    let result = self.answer(control);
                    self.update(|state| match result {
                        Ok(()) => state.message.clear(),
                        Err(err) => state.message = err,
                    });
                }
                let now = Instant::now();
                let elapsed = now.duration_since(last).as_secs_f32();
                last = now;
                let state = self.read();
                if state.playing && !state.ended {
                    owed += elapsed * LOOPS_PER_SECOND * state.speed;
                    let count = owed.floor() as u32;
                    if count > 0 {
                        owed -= count as f32;
                        if let Err(err) = self.step(count) {
                            self.update(|state| {
                                state.message = err;
                                state.playing = false;
                            });
                        }
                    }
                } else {
                    owed = 0.;
                }
                std::thread::sleep(TICK.saturating_sub(now.elapsed()));
            }
            stop_game(&mut self.child, self.port);
            self.update(|state| {
                state.active = false;
                state.playing = false;
            });
            tracing::info!("StarCraft II on port {} closed", self.port);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_encodes_the_start_of_a_replay() {
        let request = proto::start_replay("C:\\r.SC2Replay", None);
        // Request.start_replay (4, length-delimited) around the RequestStartReplay.
        assert_eq!(request[0], (4 << 3) | 2);
        assert_eq!(request[1] as usize, request.len() - 2);
        let start = &request[2..];
        // replay_path (1), then observed_player_id (2) = 1.
        assert_eq!(&start[..2], &[(1 << 3) | 2, 14]);
        assert_eq!(&start[2..16], b"C:\\r.SC2Replay");
        assert_eq!(&start[16..18], &[2 << 3, 1]);
        // options (3) = { score (2) = true }, disable_fog (4), realtime (7) = false.
        assert_eq!(&start[18..22], &[(3 << 3) | 2, 2, 2 << 3, 1]);
        assert_eq!(&start[22..], &[4 << 3, 1, 7 << 3, 0]);
        // step (12) { count (1) = 300 }.
        assert_eq!(proto::step(300), vec![(12 << 3) | 2, 3, 1 << 3, 0xac, 0x02]);
        assert_eq!(proto::observation(), vec![(10 << 3) | 2, 0]);
    }

    #[test]
    fn it_reads_the_answers() {
        // Response { start_replay (4): { error (1) = InvalidMapData (4), error_details (2) } }.
        let mut start = vec![1 << 3, 4, (2 << 3) | 2, 3];
        start.extend(b"bad");
        let mut response = vec![(4 << 3) | 2, start.len() as u8];
        response.extend(start);
        // status (99) = in_replay (4).
        response.extend([0x98, 0x06, 4]);
        assert_eq!(
            proto::errors(&response),
            vec!["InvalidMapData".to_string(), "bad".to_string()]
        );
        assert_eq!(proto::status(&response), Some(4));
        assert!(proto::errors(&[0x98, 0x06, 5]).is_empty());
        // observation (10) { observation (3) { game_loop (9) = 300 } }, status ended.
        let observation = vec![
            (10 << 3) | 2,
            5,
            (3 << 3) | 2,
            3,
            9 << 3,
            0xac,
            0x02,
            0x98,
            0x06,
            5,
        ];
        assert_eq!(proto::game_loop(&observation), Some(300));
        assert_eq!(proto::status(&observation), Some(proto::STATUS_ENDED));
    }

    #[test]
    fn it_knows_the_data_version_of_the_old_builds() {
        assert_eq!(
            data_version(75689).as_deref(),
            Some("B89B5D6FA7CBF6452E721311BFBC6CB2")
        );
        assert_eq!(data_version(1), None);
    }

    #[test]
    fn it_finds_the_map_of_a_local_game() {
        let dir = std::env::temp_dir().join(format!("swarmy-maps-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("Arena")).unwrap();
        for name in [
            "IncorporealAIE_v3.SC2Map",
            "IncorporealAIE_v4.SC2Map",
            "Pylon.SC2Map",
        ] {
            std::fs::write(dir.join("Arena").join(name), b"").unwrap();
        }
        let found = find_local_map(&dir, "Incorporeal AIE").unwrap();
        assert!(found.ends_with("Arena/IncorporealAIE_v4.SC2Map"));
        assert!(find_local_map(&dir, "Torches AIE").is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
