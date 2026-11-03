//! Runs the commands of the snapshot in a remote backend, a swarmy started with `--serve` on a
//! computer that can handle the queries, when the Remote Backend of Config is set. See `server`.

use crate::commands::BACKEND_COMMANDS;
use crate::settings::load_stored_app_settings;
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;
use swarmy_common::*;
use tauri::ipc::{Invoke, InvokeBody, InvokeError};
use tauri::{AppHandle, Emitter, Manager};

/// How often the events of the backend check whether the Remote Backend of Config changed.
const EVENTS_CHECK_INTERVAL: Duration = Duration::from_secs(5);

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            // Optimize takes as long as the replays to add, there is no timeout for the response.
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("the HTTP client of the remote backend")
    })
}

fn request(settings: &AppSettings, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
    let url = format!("{}/{path}", settings.remote_backend_url);
    let request = client().request(method, url);
    if settings.remote_backend_token.is_empty() {
        request
    } else {
        request.bearer_auth(&settings.remote_backend_token)
    }
}

/// Runs a command in the backend. The error is the JSON of the error of the command, the same one
/// it would have when it runs here.
async fn forward(settings: &AppSettings, command: &str, args: Value) -> Result<Value, Value> {
    let unreachable = |err: reqwest::Error| {
        serde_json::json!({
            "kind": "other",
            "message": format!("The remote backend {} failed: {err}", settings.remote_backend_url),
        })
    };
    let response = request(settings, reqwest::Method::POST, &format!("api/{command}"))
        .json(&args)
        .send()
        .await
        .map_err(unreachable)?;
    let status = response.status();
    let body = response.text().await.map_err(unreachable)?;
    let body = serde_json::from_str(&body).unwrap_or(Value::String(body));
    if status.is_success() {
        Ok(body)
    } else {
        Err(body)
    }
}

/// Runs a command in the backend, i.e. the parts of a command that run here.
pub async fn call(
    settings: &AppSettings,
    command: &str,
    args: Value,
) -> Result<ApiResponse, SwarmyError> {
    match forward(settings, command, args).await {
        Ok(response) => Ok(serde_json::from_value(response)?),
        Err(err) => Err(SwarmyError::Other(format!(
            "{command} in the remote backend: {err}"
        ))),
    }
}

/// The stats of the snapshot of the backend, for the Config and Scan views.
pub async fn snapshot_stats(settings: &AppSettings) -> Result<SnapshotStats, SwarmyError> {
    let response = call(
        settings,
        "get_snapshot_metadata",
        Value::Object(Default::default()),
    )
    .await?;
    // The message is the response of the majordomo, its message the stats.
    let response: ApiResponse = serde_json::from_str(&response.message)?;
    if !response.meta.success {
        return Err(SwarmyError::Other(response.message));
    }
    Ok(serde_json::from_str(&response.message)?)
}

/// The handler of the commands of the frontend: the commands of the snapshot run in the remote
/// backend when one is set in Config, `backend` runs them here otherwise.
pub fn invoke_handler(
    backend: impl Fn(Invoke) -> bool + Send + Sync + 'static,
    local: impl Fn(Invoke) -> bool + Send + Sync + 'static,
) -> impl Fn(Invoke) -> bool + Send + Sync + 'static {
    move |invoke| {
        let command = invoke.message.command().to_string();
        if !BACKEND_COMMANDS.contains(&command.as_str()) {
            return local(invoke);
        }
        let app_handle = invoke.message.webview().app_handle().clone();
        match load_stored_app_settings(&app_handle) {
            Ok(settings) if settings.is_remote() => {
                let args = match invoke.message.payload() {
                    InvokeBody::Json(args) => args.clone(),
                    InvokeBody::Raw(_) => Value::Object(Default::default()),
                };
                invoke.resolver.respond_async(async move {
                    tracing::debug!("Running {command} in {}", settings.remote_backend_url);
                    forward(&settings, &command, args)
                        .await
                        .map_err(InvokeError)
                });
                true
            }
            _ => backend(invoke),
        }
    }
}

/// The swarmy of the remote backend, for Config to check that it is reachable. The message is
/// the versions of the backend.
#[tauri::command(rename_all = "snake_case")]
pub async fn check_remote_backend(
    remote_backend_url: String,
    remote_backend_token: String,
) -> Result<ApiResponse, SwarmyError> {
    let settings = AppSettings {
        remote_backend_url: remote_backend_url.trim().trim_end_matches('/').to_string(),
        remote_backend_token: remote_backend_token.trim().to_string(),
        ..Default::default()
    };
    let result = async {
        let response = request(&settings, reqwest::Method::GET, "api/version")
            .send()
            .await
            .and_then(|response| response.error_for_status())
            .map_err(|err| SwarmyError::Other(format!("{err}")))?;
        let versions: AppVersions = response
            .json()
            .await
            .map_err(|err| SwarmyError::Other(format!("{err}")))?;
        Ok(versions)
    };
    Ok(ApiResponseBuilder::new().process_result(result.await))
}

/// Sends the events of the remote backend, i.e. the progress of Optimize, to the frontend, while
/// a remote backend is set.
pub fn relay_events(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let remote = load_stored_app_settings(&app_handle)
                .ok()
                .filter(AppSettings::is_remote);
            if let Some(settings) = remote {
                if let Err(err) = relay_events_of(&app_handle, &settings).await {
                    tracing::debug!(
                        "The events of {} stopped: {err}",
                        settings.remote_backend_url
                    );
                }
            }
            tokio::time::sleep(EVENTS_CHECK_INTERVAL).await;
        }
    });
}

/// Relays the server-sent events of the backend until it closes them or Config changes.
async fn relay_events_of(app_handle: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let mut response = request(settings, reqwest::Method::GET, "api/events")
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|err| err.to_string())?;
    tracing::info!("Relaying the events of {}", settings.remote_backend_url);
    let mut buffer: Vec<u8> = vec![];
    let mut check = tokio::time::interval(EVENTS_CHECK_INTERVAL);
    loop {
        tokio::select! {
            chunk = response.chunk() => {
                let Some(chunk) = chunk.map_err(|err| err.to_string())? else {
                    return Ok(());
                };
                buffer.extend_from_slice(&chunk);
                while let Some(end) = buffer.windows(2).position(|window| window == b"\n\n") {
                    let message: Vec<u8> = buffer.drain(..end + 2).collect();
                    if let Some((event, payload)) = parse_event(&String::from_utf8_lossy(&message)) {
                        if let Err(err) = app_handle.emit(&event, payload) {
                            tracing::error!("Error sending the {event} event: {err}");
                        }
                    }
                }
            }
            _ = check.tick() => {
                let current = load_stored_app_settings(app_handle).map_err(String::from)?;
                if current.remote_backend_url != settings.remote_backend_url
                    || current.remote_backend_token != settings.remote_backend_token
                {
                    return Ok(());
                }
            }
        }
    }
}

/// The name and the JSON payload of a server-sent event, None for the keep alive comments.
fn parse_event(message: &str) -> Option<(String, Value)> {
    let mut event = None;
    let mut data = String::new();
    for line in message.lines() {
        if let Some(name) = line.strip_prefix("event:") {
            event = Some(name.trim().to_string());
        } else if let Some(line) = line.strip_prefix("data:") {
            data.push_str(line.strip_prefix(' ').unwrap_or(line));
        }
    }
    Some((event?, serde_json::from_str(&data).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_server_sent_events() {
        assert_eq!(
            parse_event("event: optimize-progress\ndata: {\"added\":3}\n\n"),
            Some((
                "optimize-progress".to_string(),
                serde_json::json!({ "added": 3 })
            ))
        );
        assert_eq!(parse_event(":\n\n"), None);
    }
}

/// The directory of the files of the remote backend copied to this computer, per backend: the
/// replays and the map caches that swarmy-bevy reads.
pub fn local_files_dir(
    app_handle: &AppHandle,
    settings: &AppSettings,
) -> Result<std::path::PathBuf, SwarmyError> {
    let backend: String = settings
        .remote_backend_url
        .split_once("://")
        .map_or(settings.remote_backend_url.as_str(), |(_, rest)| rest)
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|err| SwarmyError::Other(format!("No app data directory: {err}")))?;
    Ok(data_dir.join("remote").join(backend))
}

/// Downloads a file of the backend to `destination`, unless it was downloaded already: the
/// replays and the map caches do not change.
async fn download(
    settings: &AppSettings,
    request: reqwest::RequestBuilder,
    destination: &std::path::Path,
) -> Result<(), SwarmyError> {
    if destination.is_file() {
        return Ok(());
    }
    let failed = |err: reqwest::Error| {
        SwarmyError::Other(format!(
            "Downloading {} from the remote backend {} failed: {err}",
            destination.display(),
            settings.remote_backend_url
        ))
    };
    let response = request.send().await.map_err(failed)?;
    let status = response.status();
    let contents = response.bytes().await.map_err(failed)?;
    if !status.is_success() {
        // The errors of the server are the JSON of a SwarmyError, its kind and message.
        let message = serde_json::from_slice::<Value>(&contents)
            .ok()
            .and_then(|err| err.get("message")?.as_str().map(str::to_string))
            .unwrap_or_else(|| String::from_utf8_lossy(&contents).to_string());
        return Err(SwarmyError::Other(format!(
            "The remote backend {} did not send {}: {message}",
            settings.remote_backend_url,
            destination.display()
        )));
    }
    if let Some(dir) = destination.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    // Written aside then renamed, an interrupted download is not taken for a complete one.
    let partial = destination.with_extension("partial");
    tokio::fs::write(&partial, &contents).await?;
    tokio::fs::rename(&partial, destination).await?;
    Ok(())
}

/// Downloads the map caches of the backend to `dir`, as `<cache id>.s2ma`.
pub async fn download_map_caches(
    settings: &AppSettings,
    cache_ids: &[String],
    dir: &std::path::Path,
) -> Result<(), SwarmyError> {
    for cache_id in cache_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
    {
        let file = dir.join(format!(
            "{cache_id}.{}",
            s2protocol::cache_handles::CACHE_MPQ_ARCHIVE_EXTENSION
        ));
        let request = request(
            settings,
            reqwest::Method::GET,
            &format!("api/files/caches/{cache_id}"),
        );
        download(settings, request, &file).await?;
    }
    Ok(())
}

/// Downloads a replay of the backend, `replay` is its path on the backend, to `dir`. The file
/// keeps its name, after a hash of the path on the backend so that two replays with the same name
/// do not collide.
pub async fn download_replay(
    settings: &AppSettings,
    replay: &str,
    dir: &std::path::Path,
) -> Result<std::path::PathBuf, SwarmyError> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    replay.hash(&mut hasher);
    let name = std::path::Path::new(replay)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "replay.SC2Replay".to_string());
    let file = dir.join(format!("{:016x}-{name}", hasher.finish()));
    let request =
        request(settings, reqwest::Method::GET, "api/files/replay").query(&[("path", replay)]);
    download(settings, request, &file).await?;
    Ok(file)
}
