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
//! With `--token`, the requests need the header `Authorization: Bearer <token>`.

use crate::commands::{dispatch, BACKEND_COMMANDS};
use crate::host::{SwarmyHost, SETTINGS_STORE};
use crate::majordomo::{AsyncTask, MajordomoCoordinator};
use crate::settings::{app_versions, load_stored_app_settings};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Map, Value};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use swarmy_common::SwarmyError;
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

    fn replays_changed(&self) {}

    fn majordomo(&self) -> mpsc::Sender<AsyncTask> {
        self.majordomo_tx.clone()
    }
}

#[derive(Clone)]
struct ServerState {
    host: Arc<ServerHost>,
    token: Option<String>,
}

impl ServerState {
    /// The response to a request without the token, None when the request has it.
    fn refuse_unauthorized(&self, headers: &HeaderMap) -> Option<Response> {
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
pub async fn serve(
    addr: SocketAddr,
    data_dir: Option<PathBuf>,
    token: Option<String>,
) -> Result<(), SwarmyError> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    serve_on(listener, data_dir, token).await
}

async fn serve_on(
    listener: tokio::net::TcpListener,
    data_dir: Option<PathBuf>,
    token: Option<String>,
) -> Result<(), SwarmyError> {
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
    });
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
    let state = ServerState { host, token };
    let app = Router::new()
        .route("/api/version", get(version))
        .route("/api/events", get(stream_events))
        .route("/api/{command}", post(run_command))
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

async fn run_command(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(command): Path<String>,
    body: Bytes,
) -> Response {
    if let Some(refused) = state.refuse_unauthorized(&headers) {
        return refused;
    }
    if !BACKEND_COMMANDS.contains(&command.as_str()) {
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
    tracing::info!("Running {command}");
    match dispatch(state.host.as_ref(), &command, args).await {
        Ok(response) => Json(response).into_response(),
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

    #[tokio::test]
    async fn it_runs_the_commands_of_the_remote_backend() {
        let data_dir = std::env::temp_dir().join(format!("swarmy-server-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data_dir);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let served = serve_on(listener, Some(data_dir.clone()), Some("token".to_string()));
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
}
