//! The access log and the metrics of `swarmy --serve`: a line per request, in JSON, in the
//! `access.<date>.log` files of the logs directory (see `logging`), and the Prometheus metrics of
//! `GET /metrics`. They tell which views are used and which queries are slow. The view of the
//! browser version that asks comes in the header `X-Swarmy-View`, written by
//! `public/web_bridge.js` from the address of the view (`#/replay-detail?...` is `replay-detail`).
//!
//! The log is about the views and the queries, not about the visitors: no addresses (a visitor id
//! that changes at each start of the server), the arguments of the public commands only (the ones
//! of the administrator can hold the Battle.net secret or the personal notes), the user agent cut
//! short. Decided with the author on 2026-11-09.

use crate::commands::BACKEND_COMMANDS;
use crate::web::{DESKTOP_COMMANDS, PUBLIC_COMMANDS, STATIC_COMMANDS};
use axum::body::{Body, HttpBody};
use axum::extract::{ConnectInfo, Request};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
    TextEncoder,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// The target of the access events, the `access.<date>.log` files take only this one.
pub const ACCESS_TARGET: &str = "swarmy_access";

/// The header of the browser version with the view that asks.
pub const VIEW_HEADER: &str = "x-swarmy-view";

/// The arguments of a command in the log, at most this many characters.
const MAX_ARGS_LEN: usize = 1_000;
/// The user agent in the log, at most this many characters.
const MAX_USER_AGENT_LEN: usize = 200;
/// The path in the log, at most this many characters: the scanners of the internet try long ones.
const MAX_PATH_LEN: usize = 200;
/// A body bigger than this is not read for the log, the command still gets it.
const MAX_BODY_LEN: u64 = 1 << 20;
/// The characters of a view name, and its length at most: the header is whatever the client sends.
const MAX_VIEW_LEN: usize = 32;
/// The distinct views counted in the metrics: the ones after are `other`, a client cannot grow the
/// metrics with made-up names.
const MAX_VIEWS: usize = 64;
/// The view when the header is missing (the swarmy apps, curl) or unusable.
const NO_VIEW: &str = "-";

/// Whether the request ran as the administrator, the handlers put it in the response.
#[derive(Clone, Copy, Debug)]
pub struct AdminRequest(pub bool);

/// The salt of the visitor ids, new at each start: an id cannot be turned into an address, and
/// the ids of two days cannot be joined.
static SALT: LazyLock<String> = LazyLock::new(|| uuid::Uuid::new_v4().to_string());

struct Metrics {
    registry: Registry,
    requests: IntCounterVec,
    duration: HistogramVec,
    /// The views seen so far, see `MAX_VIEWS`.
    views: Mutex<HashSet<String>>,
}

static METRICS: LazyLock<Metrics> = LazyLock::new(|| {
    let registry = Registry::new();
    let requests = IntCounterVec::new(
        Opts::new(
            "swarmy_requests_total",
            "The requests answered, by route (the command of POST /api/<command>), the view of the browser version that asked and the HTTP status",
        ),
        &["route", "view", "status"],
    )
    .expect("the requests counter");
    let duration = HistogramVec::new(
        HistogramOpts::new(
            "swarmy_request_duration_seconds",
            "The time to answer a request, by route",
        )
        .buckets(vec![
            0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0,
        ]),
        &["route"],
    )
    .expect("the duration histogram");
    let build_info = IntGaugeVec::new(
        Opts::new("swarmy_build_info", "The version of swarmy, always 1"),
        &["version"],
    )
    .expect("the build info gauge");
    build_info
        .with_label_values(&[env!("CARGO_PKG_VERSION")])
        .set(1);
    let start_time = IntGauge::new(
        "swarmy_start_time_seconds",
        "When the server started, seconds since the Unix epoch",
    )
    .expect("the start time gauge");
    start_time.set(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_secs() as i64)
            .unwrap_or_default(),
    );
    for metric in [
        Box::new(requests.clone()) as Box<dyn prometheus::core::Collector>,
        Box::new(duration.clone()),
        Box::new(build_info),
        Box::new(start_time),
    ] {
        registry.register(metric).expect("a metric registered once");
    }
    Metrics {
        registry,
        requests,
        duration,
        views: Mutex::new(HashSet::new()),
    }
});

/// The route of a request as the log and the metrics name it: the command of `POST /api/<command>`
/// (`unknown_command` for one the server does not have, so that a scanner cannot invent routes),
/// the API routes without their `/api/`, `static` for the files of the browser version.
pub fn route_name(method: &Method, path: &str) -> String {
    match path.strip_prefix("/api/") {
        Some(route @ ("version" | "events" | "web/session" | "web/settings")) => route.to_string(),
        Some("files/replay") => "files/replay".to_string(),
        Some(route) if route.starts_with("files/caches/") => "files/caches".to_string(),
        Some(command) if *method == Method::POST => {
            if (BACKEND_COMMANDS.contains(&command) && !DESKTOP_COMMANDS.contains(&command))
                || STATIC_COMMANDS.contains(&command)
            {
                command.to_string()
            } else {
                "unknown_command".to_string()
            }
        }
        Some(_) => "unknown_route".to_string(),
        None if path == "/metrics" => "metrics".to_string(),
        None => "static".to_string(),
    }
}

/// The view of the header, in the characters of the routes of the frontend, or `-`.
pub fn view_name(headers: &HeaderMap) -> String {
    let view: String = headers
        .get(VIEW_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(MAX_VIEW_LEN)
        .collect::<String>()
        .to_ascii_lowercase();
    if view.is_empty() {
        NO_VIEW.to_string()
    } else {
        view
    }
}

/// The view as a label of the metrics: the first `MAX_VIEWS` distinct ones, then `other`.
fn view_label(seen: &Mutex<HashSet<String>>, view: &str) -> String {
    if view == NO_VIEW {
        return view.to_string();
    }
    let mut views = seen.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if views.contains(view) {
        view.to_string()
    } else if views.len() < MAX_VIEWS {
        views.insert(view.to_string());
        view.to_string()
    } else {
        "other".to_string()
    }
}

/// A short id of the visitor: the address Cloudflare saw, or the one that connected, hashed with
/// the salt of this run. The same visitor has the same id until the server restarts.
pub fn visitor_id(headers: &HeaderMap, peer: Option<SocketAddr>) -> String {
    let address = headers
        .get("cf-connecting-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .or_else(|| peer.map(|peer| peer.ip().to_string()))
        .unwrap_or_default();
    if address.is_empty() {
        return NO_VIEW.to_string();
    }
    let digest = Sha256::digest(format!("{}{address}", *SALT));
    digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn header_value(headers: &HeaderMap, name: &str, max_len: usize) -> String {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(|value| truncate(value, max_len))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| NO_VIEW.to_string())
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        text.to_string()
    } else {
        let cut: String = text.chars().take(max_len).collect();
        format!("{cut}…")
    }
}

/// Whether the arguments of a command can be logged: the public queries. The administrator's
/// commands write the notes and the settings of the server.
fn args_logged(route: &str) -> bool {
    PUBLIC_COMMANDS.contains(&route) || STATIC_COMMANDS.contains(&route)
}

/// Writes a line of the access log and counts the request in the metrics, around every request.
pub async fn log_request(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let query = request.uri().query().unwrap_or_default().to_string();
    let route = route_name(&method, &path);
    let view = view_name(request.headers());
    let visitor = visitor_id(request.headers(), Some(peer));
    let country = header_value(request.headers(), "cf-ipcountry", 2);
    let user_agent = header_value(request.headers(), "user-agent", MAX_USER_AGENT_LEN);
    // The arguments of the public commands, read here and given back to the command.
    let (request, args) = if args_logged(&route) && body_fits(request.headers()) {
        let (parts, body) = request.into_parts();
        match axum::body::to_bytes(body, MAX_BODY_LEN as usize).await {
            Ok(bytes) => {
                let args = truncate(String::from_utf8_lossy(&bytes).trim(), MAX_ARGS_LEN);
                (Request::from_parts(parts, Body::from(bytes)), args)
            }
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("The arguments of {route} could not be read: {err}"),
                )
                    .into_response();
            }
        }
    } else if route == "files/replay" || route == "files/caches" {
        (request, truncate(&query, MAX_ARGS_LEN))
    } else {
        (request, String::new())
    };
    let response = next.run(request).await;
    let elapsed = started.elapsed();
    let status = response.status().as_u16();
    let admin = response
        .extensions()
        .get::<AdminRequest>()
        .is_some_and(|admin| admin.0);
    // The size of the body when it is known (the JSON answers), 0 for a stream (the events).
    let bytes = HttpBody::size_hint(response.body())
        .exact()
        .unwrap_or_default();
    if route != "metrics" {
        let metrics = &*METRICS;
        metrics
            .requests
            .with_label_values(&[
                &route,
                &view_label(&metrics.views, &view),
                &status.to_string(),
            ])
            .inc();
        metrics
            .duration
            .with_label_values(&[&route])
            .observe(elapsed.as_secs_f64());
    }
    tracing::info!(
        target: ACCESS_TARGET,
        %route,
        %method,
        path = %truncate(&path, MAX_PATH_LEN),
        status,
        ms = elapsed.as_millis() as u64,
        bytes,
        %view,
        admin,
        %visitor,
        %country,
        %args,
        %user_agent,
        "access"
    );
    response
}

/// Whether the body is small enough to be read for the log: it says its length, chunked bodies
/// are not logged.
fn body_fits(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length <= MAX_BODY_LEN)
}

/// The metrics in the text format of Prometheus, for `GET /metrics`.
pub fn render_metrics() -> Response {
    let encoder = TextEncoder::new();
    let mut buffer = vec![];
    if let Err(err) = encoder.encode(&METRICS.registry.gather(), &mut buffer) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("The metrics could not be written: {err}"),
        )
            .into_response();
    }
    ([(header::CONTENT_TYPE, encoder.format_type())], buffer).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn it_names_the_routes() {
        assert_eq!(route_name(&Method::GET, "/api/version"), "version");
        assert_eq!(route_name(&Method::GET, "/api/web/session"), "web/session");
        assert_eq!(
            route_name(
                &Method::GET,
                &format!("/api/files/caches/{}", "ab".repeat(32))
            ),
            "files/caches"
        );
        assert_eq!(
            route_name(&Method::GET, "/api/files/replay"),
            "files/replay"
        );
        assert_eq!(
            route_name(&Method::POST, "/api/query_replay_list"),
            "query_replay_list"
        );
        assert_eq!(
            route_name(&Method::POST, "/api/list_patch_units"),
            "list_patch_units"
        );
        assert_eq!(
            route_name(&Method::POST, "/api/drop_tables"),
            "unknown_command"
        );
        assert_eq!(
            route_name(&Method::GET, "/api/query_replay_list"),
            "unknown_route"
        );
        assert_eq!(route_name(&Method::GET, "/metrics"), "metrics");
        assert_eq!(route_name(&Method::GET, "/"), "static");
        assert_eq!(route_name(&Method::GET, "/wp-admin/setup.php"), "static");
    }

    #[test]
    fn it_reads_the_view_of_the_header() {
        let mut headers = HeaderMap::new();
        assert_eq!(view_name(&headers), "-");
        headers.insert(VIEW_HEADER, HeaderValue::from_static("replay-detail"));
        assert_eq!(view_name(&headers), "replay-detail");
        headers.insert(
            VIEW_HEADER,
            HeaderValue::from_static("Player <script>alert(1)</script>"),
        );
        assert_eq!(view_name(&headers), "playerscriptalert1script");
        headers.insert(VIEW_HEADER, HeaderValue::from_static("?!"));
        assert_eq!(view_name(&headers), "-");
        let long = "a".repeat(100);
        headers.insert(VIEW_HEADER, HeaderValue::from_str(&long).unwrap());
        assert_eq!(view_name(&headers).len(), MAX_VIEW_LEN);
    }

    #[test]
    fn it_hides_the_addresses_of_the_visitors() {
        let peer: SocketAddr = "192.168.1.20:4000".parse().unwrap();
        let mut headers = HeaderMap::new();
        let lan = visitor_id(&headers, Some(peer));
        assert_eq!(lan.len(), 12);
        assert!(lan.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(lan, visitor_id(&headers, Some(peer)));
        // The address Cloudflare saw counts, not Cloudflare's own.
        headers.insert("cf-connecting-ip", HeaderValue::from_static("203.0.113.9"));
        let visitor = visitor_id(&headers, Some(peer));
        assert_ne!(visitor, lan);
        assert_eq!(visitor, visitor_id(&headers, None));
        assert_eq!(visitor_id(&HeaderMap::new(), None), "-");
    }

    #[test]
    fn it_logs_the_arguments_of_the_public_commands_only() {
        assert!(args_logged("query_replay_list"));
        assert!(args_logged("query_patch_changes"));
        assert!(!args_logged("set_battlenet_credentials"));
        assert!(!args_logged("set_replay_notes"));
        assert!(!args_logged("unknown_command"));
        assert_eq!(truncate("abc", 5), "abc");
        assert_eq!(truncate("abcdef", 3), "abc…");
    }

    #[test]
    fn it_caps_the_views_of_the_metrics() {
        let seen = Mutex::new(HashSet::new());
        for i in 0..(MAX_VIEWS + 10) {
            view_label(&seen, &format!("view-{i}"));
        }
        assert_eq!(view_label(&seen, "view-0"), "view-0");
        assert_eq!(view_label(&seen, "made-up"), "other");
        assert_eq!(view_label(&seen, "-"), "-");
        let metrics = render_metrics();
        assert_eq!(metrics.status(), StatusCode::OK);
    }
}
