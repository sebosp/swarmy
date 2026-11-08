//! swarmy as the remote backend of other swarmy apps: `swarmy --serve 0.0.0.0:7878` runs the
//! commands of the snapshot, i.e. the polars queries, for a computer that cannot handle them. It
//! has no window, it uses the settings and the personal database of the swarmy app of the same
//! computer, the Replay Path of its Config is the snapshot served. See `remote` for the client.
//!
//! - `POST /api/<command>`, the arguments of the command as a JSON object: the response of the
//!   command, or its error with status 500.
//! - `GET /api/events`: the events of the commands, i.e. the progress of Optimize, as server-sent
//!   events.
//! - `GET /api/version`: the versions of swarmy and its libraries.
//!
//! - `GET /api/web/session` and `GET /api/web/settings`: what the browser version needs to start.
//! - Every other `GET`: the files of the browser version, see `web`.
//!
//! With `--token`, the requests need the header `Authorization: Bearer <token>`. With
//! `--admin-key-file` anyone can read the snapshot, and the token or the administrator key are
//! needed for the [`ADMIN_COMMANDS`] and the files of the server.

use crate::commands::{dispatch, BACKEND_COMMANDS};
use crate::host::{SwarmyHost, SETTINGS_STORE};
use crate::majordomo::{AsyncTask, MajordomoCoordinator};
use crate::settings::{app_versions, load_stored_app_settings};
use crate::web::{
    admin_settings, key_matches, public_settings, strip_server_paths, WebAssets, ADMIN_COMMANDS,
    ADMIN_KEY_HEADER, PUBLIC_COMMANDS, STATIC_COMMANDS,
};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode, Uri};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Map, Value};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use swarmy_common::{ApiResponse, EvolutionSeries, SwarmyError};
use tokio::sync::{broadcast, mpsc};

/// The port of the examples and of the placeholder of Config.
pub const DEFAULT_PORT: u16 = 7878;

/// The events kept for a client that reads them slower than they are sent.
const EVENTS_CAPACITY: usize = 1_024;

/// The host of the commands in the server.
#[derive(Debug)]
pub struct ServerHost {
    data_dir: PathBuf,
    events: broadcast::Sender<(String, Value)>,
    majordomo_tx: mpsc::Sender<AsyncTask>,
    /// The writes of the settings store, one at a time.
    store_lock: Mutex<()>,
    watcher: Arc<crate::replay_watcher::ReplayWatcherState>,
}

impl ServerHost {
    fn store_path(&self) -> PathBuf {
        self.data_dir.join(SETTINGS_STORE)
    }
}

impl SwarmyHost for ServerHost {
    fn data_dir(&self) -> Result<PathBuf, SwarmyError> {
        Ok(self.data_dir.clone())
    }

    fn read_store(&self) -> Result<Map<String, Value>, SwarmyError> {
        match std::fs::read_to_string(self.store_path()) {
            Ok(store) => Ok(serde_json::from_str(&store)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
            Err(err) => Err(err.into()),
        }
    }

    fn write_store(&self, key: &str, value: Value) -> Result<(), SwarmyError> {
        let _guard = self
            .store_lock
            .lock()
            .map_err(|_| SwarmyError::Other("The settings lock is poisoned".to_string()))?;
        let mut store = self.read_store()?;
        store.insert(key.to_string(), value);
        std::fs::create_dir_all(&self.data_dir)?;
        // Written aside and renamed, the swarmy app of the computer may be reading it.
        let path = self.store_path();
        let written = path.with_extension("json.tmp");
        std::fs::write(&written, serde_json::to_string_pretty(&store)?)?;
        std::fs::rename(&written, &path)?;
        Ok(())
    }

    fn emit(&self, event: &str, payload: Value) {
        // There are no receivers while no client listens to the events.
        let _ = self.events.send((event.to_string(), payload));
    }

    fn replays_changed(&self) {
        // The watcher counts the new replays again through its own handle of the host.
        if let Some(host) = SERVER_HOST.get() {
            crate::replay_watcher::refresh(host.clone());
        }
    }

    fn replay_watcher(&self) -> Arc<crate::replay_watcher::ReplayWatcherState> {
        self.watcher.clone()
    }

    fn majordomo(&self) -> mpsc::Sender<AsyncTask> {
        self.majordomo_tx.clone()
    }
}

#[derive(Clone)]
struct ServerState {
    host: Arc<ServerHost>,
    token: Option<String>,
    admin_key_file: Option<PathBuf>,
    web: Arc<WebAssets>,
}

impl ServerState {
    /// Whether the request has the token, when there is one.
    fn has_token(&self, headers: &HeaderMap) -> bool {
        let Some(token) = self.token.as_deref() else {
            return false;
        };
        headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            == Some(token)
    }

    /// Whether the request is of the administrator: it has the token or the key. A server with
    /// neither is private, as a remote backend of the same network, every request is.
    fn is_admin(&self, headers: &HeaderMap) -> bool {
        if self.token.is_none() && self.admin_key_file.is_none() {
            return true;
        }
        if self.has_token(headers) {
            return true;
        }
        let Some(key_file) = &self.admin_key_file else {
            return false;
        };
        // The browser sends the key in its header, the app as the token of its Remote Backend.
        let bearer = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        headers
            .get(ADMIN_KEY_HEADER)
            .and_then(|value| value.to_str().ok())
            .into_iter()
            .chain(bearer)
            .any(|key| key_matches(key_file, key))
    }

    /// The response to a request that is not of the administrator, None when it is.
    fn refuse_not_admin(&self, headers: &HeaderMap) -> Option<Response> {
        (!self.is_admin(headers)).then(|| {
            error_response(
                StatusCode::FORBIDDEN,
                "This needs the administrator key of the server",
            )
        })
    }

    /// The response to a request without the token, None when the request has it. With an
    /// administrator key the snapshot is public, the token is only one way to be the administrator.
    fn refuse_unauthorized(&self, headers: &HeaderMap) -> Option<Response> {
        if self.admin_key_file.is_some() {
            return None;
        }
        let token = self.token.as_deref()?;
        let bearer = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));
        if bearer == Some(token) {
            return None;
        }
        Some(error_response(
            StatusCode::UNAUTHORIZED,
            "The token of the remote backend is missing or wrong",
        ))
    }
}

/// An error with the JSON of SwarmyError, as the frontend gets it from a local command.
fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    let err = SwarmyError::Other(message.into());
    (status, Json(serde_json::to_value(&err).unwrap_or_default())).into_response()
}

/// The app data directory of the swarmy app, its settings and personal database are the ones of
/// the server unless `--data-dir` is given.
pub fn default_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join(crate::logging::APP_IDENTIFIER))
}

/// Serves the commands of the snapshot until the process is stopped.
pub async fn serve(addr: SocketAddr, options: ServeOptions) -> Result<(), SwarmyError> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    serve_on(listener, options).await
}

/// How the server runs, from the command line.
#[derive(Debug, Default)]
pub struct ServeOptions {
    pub data_dir: Option<PathBuf>,
    pub token: Option<String>,
    pub admin_key_file: Option<PathBuf>,
    pub web: WebAssets,
}

/// The host of the server, for the watcher that needs its own handle of it.
static SERVER_HOST: std::sync::OnceLock<Arc<dyn SwarmyHost>> = std::sync::OnceLock::new();

async fn serve_on(
    listener: tokio::net::TcpListener,
    options: ServeOptions,
) -> Result<(), SwarmyError> {
    let ServeOptions {
        data_dir,
        token,
        admin_key_file,
        web,
    } = options;
    let addr = listener.local_addr()?;
    let data_dir = data_dir
        .or_else(default_data_dir)
        .ok_or_else(|| SwarmyError::Other("No app data directory, use --data-dir".to_string()))?;
    let (majordomo_tx, majordomo_rx) = mpsc::channel(4_096); // TODO: Magic number removal
    let (events, _) = broadcast::channel(EVENTS_CAPACITY);
    let host = Arc::new(ServerHost {
        data_dir,
        events,
        majordomo_tx,
        store_lock: Mutex::new(()),
        watcher: Default::default(),
    });
    // The server always watches its Replay Path, the apps that use it are told of the new replays.
    let _ = SERVER_HOST.set(host.clone());
    crate::replay_watcher::configure(host.clone(), true);
    MajordomoCoordinator::init_coordinator_thread(majordomo_rx, host.clone())?;
    let settings = load_stored_app_settings(host.as_ref())?;
    tracing::info!(
        "Serving the snapshot of {} with the settings of {} on http://{addr}{}",
        settings.replay_path,
        host.data_dir.display(),
        if token.is_some() {
            ", with a token"
        } else {
            ""
        },
    );
    if web.root.is_none() && web.embedded.is_empty() {
        tracing::warn!("This binary has no browser version, it was built without the frontend");
    }
    let state = ServerState {
        host,
        token,
        admin_key_file,
        web: Arc::new(web),
    };
    let app = Router::new()
        .route("/api/version", get(version))
        .route("/api/events", get(stream_events))
        .route("/api/files/caches/{cache_id}", get(map_cache_file))
        .route("/api/files/replay", get(replay_file))
        .route("/api/web/session", get(web_session))
        .route("/api/web/settings", get(web_settings))
        .route("/api/{command}", post(run_command))
        .fallback(get(web_file))
        .with_state(state);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn version(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Some(refused) = state.refuse_unauthorized(&headers) {
        return refused;
    }
    Json(app_versions()).into_response()
}

/// A map cache of the Map Caches Path of the server, for the swarmy-bevy of a client: only a
/// cache id, the sha256 of the cache in hexadecimal, is accepted.
// The files of the server are of its administrator, the browser version does not use them.
async fn map_cache_file(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(cache_id): Path<String>,
) -> Response {
    if let Some(refused) = state.refuse_unauthorized(&headers) {
        return refused;
    }
    if let Some(refused) = state.refuse_not_admin(&headers) {
        return refused;
    }
    if !is_cache_id(&cache_id) {
        return error_response(
            StatusCode::BAD_REQUEST,
            format!("{cache_id} is not a cache id"),
        );
    }
    let settings = match load_stored_app_settings(state.host.as_ref()) {
        Ok(settings) => settings,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    };
    let file = PathBuf::from(&settings.cache_path).join(format!(
        "{cache_id}.{}",
        s2protocol::cache_handles::CACHE_MPQ_ARCHIVE_EXTENSION
    ));
    send_file(&file).await
}

/// The query of [`replay_file`].
#[derive(serde::Deserialize)]
struct ReplayFileQuery {
    /// The path of the replay on the server, as in the snapshot.
    path: String,
}

/// A replay of the Replay Path of the server, for the swarmy-bevy of a client: only the
/// .SC2Replay files inside the Replay Path are sent.
async fn replay_file(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Query(query): Query<ReplayFileQuery>,
) -> Response {
    if let Some(refused) = state.refuse_unauthorized(&headers) {
        return refused;
    }
    if let Some(refused) = state.refuse_not_admin(&headers) {
        return refused;
    }
    let settings = match load_stored_app_settings(state.host.as_ref()) {
        Ok(settings) => settings,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    };
    match replay_inside(&settings.replay_path, &query.path) {
        Some(file) => send_file(&file).await,
        None => error_response(
            StatusCode::FORBIDDEN,
            format!(
                "{} is not a replay of the Replay Path of the server",
                query.path
            ),
        ),
    }
}

/// Whether the name is a cache id: the sha256 of the cache, 64 hexadecimal digits.
fn is_cache_id(name: &str) -> bool {
    name.len() == 64 && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// The replay, once its links and `..` are resolved, when it is a .SC2Replay inside the replay
/// path.
fn replay_inside(replay_path: &str, replay: &str) -> Option<PathBuf> {
    let root = std::fs::canonicalize(replay_path).ok()?;
    let file = std::fs::canonicalize(replay).ok()?;
    let is_replay = file
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("SC2Replay"));
    (is_replay && file.starts_with(&root) && file.is_file()).then_some(file)
}

async fn send_file(file: &std::path::Path) -> Response {
    match tokio::fs::read(file).await {
        Ok(contents) => (
            [(header::CONTENT_TYPE, "application/octet-stream")],
            contents,
        )
            .into_response(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => error_response(
            StatusCode::NOT_FOUND,
            format!("{} is not on the server", file.display()),
        ),
        Err(err) => error_response(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

/// Whether the browser is of the administrator, the frontend shows Scan and Optimize then.
async fn web_session(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    // Whether the server has a key is not told: the browser always asks for one.
    Json(serde_json::json!({ "admin": state.is_admin(&headers) })).into_response()
}

/// The settings for the browser, without anything private of the server.
async fn web_settings(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Some(refused) = state.refuse_unauthorized(&headers) {
        return refused;
    }
    let settings = match load_stored_app_settings(state.host.as_ref()) {
        Ok(settings) => settings,
        Err(err) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    };
    let (replay_path, cache_path) = (settings.replay_path.clone(), settings.cache_path.clone());
    // No snapshot yet is not an error, the view says what to do.
    let stats = tokio::task::spawn_blocking(move || {
        crate::snapshot_stats::try_get_snapshot_metadata(replay_path, cache_path)
            .map_err(|err| err.to_string())
    })
    .await;
    let stats = match stats {
        Ok(Ok(stats)) => stats,
        Ok(Err(err)) => {
            tracing::warn!("No snapshot statistics: {err}");
            Default::default()
        }
        Err(err) => {
            tracing::error!("The snapshot statistics failed: {err}");
            Default::default()
        }
    };
    let settings = if state.is_admin(&headers) {
        admin_settings(settings, stats)
    } else {
        public_settings(stats)
    };
    Json(settings).into_response()
}

/// A file of the browser version.
async fn web_file(State(state): State<ServerState>, uri: Uri) -> Response {
    match state.web.read(uri.path()) {
        Some(contents) => (
            [(header::CONTENT_TYPE, crate::web::content_type(uri.path()))],
            contents,
        )
            .into_response(),
        None => error_response(StatusCode::NOT_FOUND, format!("{} not found", uri.path())),
    }
}

/// The commands of the app that use no snapshot nor window.
async fn dispatch_static(command: &str, args: Value) -> Result<ApiResponse, SwarmyError> {
    use crate::patch_overview as patch;
    #[derive(serde::Deserialize)]
    struct UnitName {
        unit_name: String,
    }
    #[derive(serde::Deserialize)]
    struct Version {
        version: u32,
    }
    #[derive(serde::Deserialize)]
    struct Series {
        series: Vec<EvolutionSeries>,
    }
    match command {
        "list_unit_icons" => crate::unit_icons::list_unit_icons().await,
        "list_patch_units" => patch::list_patch_units().await,
        "query_unit_overview" => {
            let args: UnitName = serde_json::from_value(args)?;
            patch::query_unit_overview(args.unit_name).await
        }
        "query_patch_changes" => {
            let args: Version = serde_json::from_value(args)?;
            patch::query_patch_changes(args.version).await
        }
        "list_evolution_items" => {
            let args: UnitName = serde_json::from_value(args)?;
            patch::list_evolution_items(args.unit_name).await
        }
        "query_patch_evolution" => {
            let args: Series = serde_json::from_value(args)?;
            patch::query_patch_evolution(args.series).await
        }
        _ => Err(SwarmyError::Other(format!("Unknown command {command}"))),
    }
}

async fn run_command(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(command): Path<String>,
    body: Bytes,
) -> Response {
    if let Some(refused) = state.refuse_unauthorized(&headers) {
        return refused;
    }
    let is_static = STATIC_COMMANDS.contains(&command.as_str());
    if !is_static && !BACKEND_COMMANDS.contains(&command.as_str()) {
        return error_response(
            StatusCode::NOT_FOUND,
            format!("{command} does not run in the remote backend"),
        );
    }
    let args = if body.is_empty() {
        Value::Object(Map::new())
    } else {
        match serde_json::from_slice(&body) {
            Ok(args) => args,
            Err(err) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    format!("The arguments of {command}: {err}"),
                )
            }
        }
    };
    let is_admin = state.is_admin(&headers);
    // Unknown to both lists means it is not public, the lists are checked by a test.
    if !is_static && !PUBLIC_COMMANDS.contains(&command.as_str()) && !is_admin {
        debug_assert!(ADMIN_COMMANDS.contains(&command.as_str()));
        if let Some(refused) = state.refuse_not_admin(&headers) {
            return refused;
        }
    }
    tracing::info!("Running {command}");
    let result = if is_static {
        dispatch_static(&command, args).await
    } else {
        dispatch(state.host.as_ref(), &command, args).await
    };
    match result {
        Ok(mut response) => {
            if !is_admin {
                if let Ok(settings) = load_stored_app_settings(state.host.as_ref()) {
                    response.message = strip_server_paths(response.message, &settings);
                }
            }
            Json(response).into_response()
        }
        Err(err) => {
            tracing::error!("{command} failed: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::to_value(&err).unwrap_or_default()),
            )
                .into_response()
        }
    }
}

async fn stream_events(State(state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Some(refused) = state.refuse_unauthorized(&headers) {
        return refused;
    }
    let stream = futures_util::stream::unfold(state.host.events.subscribe(), |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok((event, payload)) => {
                    let event = Event::default().event(event).data(payload.to_string());
                    return Some((Ok::<_, Infallible>(event), rx));
                }
                // The client missed some progress, the next one replaces it.
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use swarmy_common::{AppSettings, ReplayNotes};

    #[test]
    fn it_accepts_only_cache_ids() {
        assert!(is_cache_id(&"ab12".repeat(16)));
        assert!(!is_cache_id("../settings.json"));
        assert!(!is_cache_id(&"g".repeat(64)));
        assert!(!is_cache_id(&"a".repeat(63)));
    }

    #[test]
    fn it_sends_only_the_replays_of_the_replay_path() {
        let root = std::env::temp_dir().join(format!("swarmy-replays-{}", std::process::id()));
        let replays = root.join("replays");
        std::fs::create_dir_all(replays.join("2026")).unwrap();
        let replay = replays.join("2026").join("game.SC2Replay");
        std::fs::write(&replay, b"replay").unwrap();
        std::fs::write(root.join("outside.SC2Replay"), b"replay").unwrap();
        std::fs::write(replays.join("settings.json"), b"{}").unwrap();
        let replay_path = replays.display().to_string();
        assert_eq!(
            replay_inside(&replay_path, &replay.display().to_string()),
            Some(std::fs::canonicalize(&replay).unwrap())
        );
        // Outside of the replay path, also through `..`.
        let outside = replays.join("..").join("outside.SC2Replay");
        assert_eq!(
            replay_inside(&replay_path, &outside.display().to_string()),
            None
        );
        // Not a replay.
        let settings = replays.join("settings.json");
        assert_eq!(
            replay_inside(&replay_path, &settings.display().to_string()),
            None
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn it_runs_the_commands_of_the_remote_backend() {
        let data_dir = std::env::temp_dir().join(format!("swarmy-server-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data_dir);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let served = serve_on(
            listener,
            ServeOptions {
                data_dir: Some(data_dir.clone()),
                token: Some("token".to_string()),
                ..ServeOptions::default()
            },
        );
        // SwarmyError is not Send.
        tokio::spawn(async move { served.await.map_err(String::from) });
        let settings = AppSettings {
            remote_backend_url: url,
            remote_backend_token: "token".to_string(),
            ..Default::default()
        };
        let notes = ReplayNotes {
            sha256: "abc".to_string(),
            rating: Some(4),
            note: "Remote".to_string(),
            tags: vec!["macro".to_string()],
        };
        let saved = crate::remote::call(
            &settings,
            "set_replay_notes",
            serde_json::json!({ "notes": notes }),
        )
        .await
        .unwrap();
        assert!(saved.meta.success, "{}", saved.message);
        let read = crate::remote::call(
            &settings,
            "get_replay_notes",
            serde_json::json!({ "sha256": "abc" }),
        )
        .await
        .unwrap();
        let read: ReplayNotes = serde_json::from_str(&read.message).unwrap();
        assert_eq!(read, notes);
        assert!(data_dir.join("swarmy.duckdb").exists());
        // The commands of the app itself do not run in the backend.
        let refused = crate::remote::call(&settings, "open_folder", serde_json::json!({})).await;
        assert!(refused.is_err());
        let wrong_token = AppSettings {
            remote_backend_token: "other".to_string(),
            ..settings
        };
        let refused = crate::remote::call(&wrong_token, "list_replay_tags", Value::Null).await;
        assert!(format!("{refused:?}").contains("token"), "{refused:?}");
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    /// The browser version: anyone reads, the administrator key is needed for the rest and a new
    /// key works without a restart.
    #[tokio::test]
    async fn it_serves_the_browser_version_with_an_administrator_key() {
        let dir = std::env::temp_dir().join(format!("swarmy-web-server-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let web_root = dir.join("web");
        std::fs::create_dir_all(&web_root).unwrap();
        std::fs::write(web_root.join("index.html"), "<html>swarmy</html>").unwrap();
        let key_file = dir.join("admin-key.sha512");
        std::fs::write(&key_file, crate::web::sha512_hex("first")).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let served = serve_on(
            listener,
            ServeOptions {
                data_dir: Some(dir.join("data")),
                token: None,
                admin_key_file: Some(key_file.clone()),
                web: WebAssets {
                    root: Some(web_root),
                    ..WebAssets::default()
                },
            },
        );
        tokio::spawn(async move { served.await.map_err(String::from) });
        let client = reqwest::Client::new();
        let post = |command: &str, key: Option<&str>| {
            let request = client
                .post(format!("{url}/api/{command}"))
                .json(&serde_json::json!({}));
            match key {
                Some(key) => request.header(ADMIN_KEY_HEADER, key),
                None => request,
            }
            .send()
        };

        let index = client.get(format!("{url}/")).send().await.unwrap();
        assert_eq!(index.status(), 200);
        assert_eq!(index.text().await.unwrap(), "<html>swarmy</html>");
        let missing = client.get(format!("{url}/nope.js")).send().await.unwrap();
        assert_eq!(missing.status(), 404);

        // Every command of the administrator is refused without the key, before it runs.
        for command in ADMIN_COMMANDS {
            let refused = post(command, None).await.unwrap();
            assert_eq!(refused.status(), 403, "{command}");
            let refused = post(command, Some("wrong")).await.unwrap();
            assert_eq!(refused.status(), 403, "{command}");
        }
        let replay = client
            .get(format!("{url}/api/files/replay?path=a"))
            .send()
            .await
            .unwrap();
        assert_eq!(replay.status(), 403);
        // The readers do not need it, the commands without a snapshot answer here.
        let public = post("list_patch_units", None).await.unwrap();
        assert_eq!(public.status(), 200);
        // A command the browser cannot run is not found.
        assert_eq!(post("open_folder", None).await.unwrap().status(), 404);

        // The key opens them, a new one replaces it in the next request.
        let session = |key: &'static str| {
            client
                .get(format!("{url}/api/web/session"))
                .header(ADMIN_KEY_HEADER, key)
                .send()
        };
        let body: Value = session("first").await.unwrap().json().await.unwrap();
        assert_eq!(body["admin"], true);
        // The app sends the key as the token of its Remote Backend.
        let bearer = client
            .get(format!("{url}/api/web/session"))
            .bearer_auth("first")
            .send()
            .await
            .unwrap();
        let body: Value = bearer.json().await.unwrap();
        assert_eq!(body["admin"], true);
        // Whether the server has a key is not told.
        assert!(body.get("key_required").is_none());
        assert_eq!(
            post("list_replay_tags", Some("first"))
                .await
                .unwrap()
                .status(),
            200
        );
        std::fs::write(&key_file, crate::web::sha512_hex("second")).unwrap();
        let body: Value = session("first").await.unwrap().json().await.unwrap();
        assert_eq!(body["admin"], false);
        let body: Value = session("second").await.unwrap().json().await.unwrap();
        assert_eq!(body["admin"], true);

        // The settings tell nothing of the server.
        let settings: AppSettings = client
            .get(format!("{url}/api/web/settings"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(settings.replay_path.is_empty());
        assert!(settings.remote_backend_token.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
