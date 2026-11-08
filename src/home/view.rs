//! Welcome to swarmy: what it is for, where its data comes from and the versions it is built with.

use crate::components::loading::Loading;
use crate::navigation::{use_navigation, TOUR_PAGE};
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, FADERS, GITHUB_LOGO, SIGNPOST};
use swarmy_common::*;

pub const ISSUES_URL: &str = "https://github.com/sebosp/swarmy/issues";

/// When the snapshot was last optimized: how long ago, and the date and time.
fn optimized_at(stats: &SnapshotStats) -> Option<(String, String)> {
    let seconds = stats
        .date_modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let date = chrono::DateTime::from_timestamp(seconds as i64, 0)?;
    Some((
        crate::scan::arrow_ipc_stats::time_ago(date.naive_utc()),
        date.with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M")
            .to_string(),
    ))
}

/// The arguments of the check_remote_backend command.
#[derive(serde::Serialize)]
struct RemoteBackendArgs {
    remote_backend_url: String,
    remote_backend_token: String,
}

/// Where the snapshot is, on this computer or on a remote backend (which one, its version), its
/// replay directory and when it was optimized; or where to set it up when it is not set yet.
#[component]
fn SetupStatus() -> impl IntoView {
    let navigation = use_navigation();
    let (settings, set_settings) = signal(None::<AppSettings>);
    let (_, set_app_settings) = signal(AppSettings::default());
    // The swarmy of the remote backend, None until it answers.
    let backend_version = RwSignal::new(None::<Result<String, String>>);
    crate::settings::with_fetch_current_app_config(set_app_settings, move |loaded: AppSettings| {
        if loaded.is_remote() {
            let args = RemoteBackendArgs {
                remote_backend_url: loaded.remote_backend_url.clone(),
                remote_backend_token: loaded.remote_backend_token.clone(),
            };
            spawn_local(async move {
                let checked = invoke_json::<AppVersions>("check_remote_backend", &args)
                    .await
                    .map(|versions| {
                        format!(
                            "swarmy {}, s2protocol {}",
                            versions.swarmy, versions.s2protocol
                        )
                    });
                backend_version.set(Some(checked));
            });
        }
        set_settings.set(Some(loaded))
    });
    let open_config = move |_ev: MouseEvent| navigation.active_page.set("Config".to_string());
    let open_tour = move |_ev: MouseEvent| navigation.active_page.set(TOUR_PAGE.to_string());
    move || {
        let Some(settings) = settings.get() else {
            return view! { <Loading message="Reading the configuration..." /> }.into_any();
        };
        let remote = settings.is_remote();
        let host = settings.remote_host().to_string();
        let replay_path = settings.snapshot_replay_path();
        let not_configured = !settings.can_scan();
        let box_class = if not_configured {
            "rounded-md border border-warning/60 bg-warning/10 p-3 space-y-2"
        } else {
            "rounded-md border border-base-content/15 bg-base-200/40 p-3 space-y-2"
        };
        let stats = settings.snapshot_stats.clone();
        let snapshot = if stats.num_games > 0 {
            let games = crate::components::abilities::group_digits(stats.num_games);
            match optimized_at(&stats) {
                Some((ago, at)) => format!("{games} games, optimized {ago} ({at})"),
                None => format!("{games} games"),
            }
        } else if remote {
            "Not optimized yet, run Optimize in Scan".to_string()
        } else {
            "Not optimized yet, see Scan".to_string()
        };
        let row = |label: String, value: AnyView| {
            view! {
                <tr>
                    <td class="pr-4 py-0.5 align-top text-muted whitespace-nowrap">{label}</td>
                    <td class="py-0.5 text-base-content">{value}</td>
                </tr>
            }
        };
        let path_view = |path: String| {
            let text = if path.is_empty() {
                "Not set".to_string()
            } else {
                path
            };
            view! { <span class="font-mono break-all">{text}</span> }.into_any()
        };
        view! {
            <div class=box_class>
                {not_configured.then(|| view! {
                    <p class="text-sm text-warning">
                        "The replays directory is not set yet: open Config, set the Replay Path to \
                        the directory with your replays and Save. Then Optimize them in Scan."
                    </p>
                })}
                <table class="text-sm">
                    <tbody>
                        {row(
                            "Snapshot".to_string(),
                            if remote {
                                view! {
                                    <span>
                                        "On the remote backend "
                                        <span class="font-mono">{host.clone()}</span>
                                    </span>
                                }
                                .into_any()
                            } else {
                                view! { <span>"On this computer"</span> }.into_any()
                            },
                        )}
                        {remote.then(|| row(
                            "Backend version".to_string(),
                            view! {
                                <span>
                                    {move || match backend_version.get() {
                                        None => "Asking the backend...".to_string(),
                                        Some(Ok(version)) => version,
                                        Some(Err(err)) => format!("Unreachable: {err}"),
                                    }}
                                </span>
                            }
                            .into_any(),
                        ))}
                        {row(
                            if remote {
                                format!("Replays directory on {host}")
                            } else {
                                "Replays directory".to_string()
                            },
                            if crate::components::web_session::is_web() && replay_path.is_empty() {
                                view! { <span class="text-muted">"Kept by the server"</span> }
                                    .into_any()
                            } else if remote && replay_path.is_empty() {
                                view! { <span class="text-muted">"Not sent by the backend, update it to swarmy 0.2.3 or later"</span> }.into_any()
                            } else {
                                path_view(replay_path)
                            },
                        )}
                        {(!remote).then(|| row(
                            "Map caches directory".to_string(),
                            path_view(settings.cache_path.clone()),
                        ))}
                        {row("Games".to_string(), view! { <span>{snapshot}</span> }.into_any())}
                        {(remote && !settings.replay_path.is_empty()).then(|| row(
                            "Replays on this computer".to_string(),
                            view! {
                                <span>
                                    <span class="font-mono break-all">{settings.replay_path.clone()}</span>
                                    <span class="text-muted">" (watched for new replays)"</span>
                                </span>
                            }
                            .into_any(),
                        ))}
                    </tbody>
                </table>
                <div class="flex gap-2">
                    <button
                        class=if not_configured { "btn btn-accent btn-sm" } else { "btn btn-ghost btn-sm" }
                        on:click=open_config
                    >
                        <Icon icon=FADERS weight=IconWeight::Light />
                        "Open Config"
                    </button>
                    <button class="btn btn-primary btn-sm" on:click=open_tour title="A guided tour of the views and the configuration">
                        <Icon icon=SIGNPOST weight=IconWeight::Light />
                        "Take the tour"
                    </button>
                </div>
            </div>
        }
        .into_any()
    }
}

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="max-w-3xl p-4 space-y-6">
            <div>
                <h1 class="text-2xl font-semibold text-base-content">
                    "Welcome to Swarmy, a StarCraft II Replay Batch Analyser"
                </h1>
                <p class="mt-2 text-sm text-muted">
                    "Swarmy reads a whole directory of replays at once to help you identify trends: \
                    which units, openings and timings are played on each patch and how they change \
                    over time. Casters can collect statistics about players, maps, race balance \
                    (PvT, TvZ, ZvP...) and player vs player records before a match."
                </p>
                <p class="mt-2 text-sm text-muted">
                    {(!crate::components::web_session::is_web())
                        .then_some("Set the replay directory in Config, then Optimize it into a snapshot in Scan. ")}
                    "The Tour walks through every view, the Changelog has what is new and Help \
                    describes how the statistics are calculated."
                </p>
            </div>
            <SetupStatus />
            {(!crate::components::web_session::is_web()).then(|| view! { <OpenLink /> })}
            <div>
                <h2 class="text-base/7 font-semibold text-base-content">"Where the data comes from"</h2>
                <ul class="mt-1 list-disc list-inside text-sm text-muted space-y-1">
                    <li>
                        "The replays (.SC2Replay) and the map caches of Blizzard's StarCraft II, the \
                        unit costs and button icons from the balance data exported with the \
                        StarCraft II Editor."
                    </li>
                    <li>
                        "They are decoded by "
                        <ExternalLink url="https://github.com/sebosp/s2protocol-rs" label="s2protocol-rs" />
                        ", a rustified version of Blizzard's "
                        <ExternalLink url="https://github.com/Blizzard/s2protocol/" label="s2protocol" />
                        ", reading the MPQ archives with "
                        <ExternalLink url="https://github.com/sebosp/nom-mpq" label="nom-mpq" />
                        ", a rustified version of "
                        <ExternalLink url="https://github.com/arkx/mpyq/" label="mpyq" />
                        "."
                    </li>
                    <li>
                        "The map ramps of swarmy-bevy use the ramp algorithm of "
                        <ExternalLink
                            url="https://github.com/RFEphemeration/sc2-map-analyzer/"
                            label="sc2-map-analyzer"
                        />
                        "."
                    </li>
                </ul>
            </div>
            <p class="text-sm text-muted">
                "The versions of swarmy and of the libraries it is built with are in Help."
            </p>
            <div class="text-sm text-muted">
                <p>"Comments and suggestions are welcome, as well as bug reports."</p>
                <button
                    class="btn btn-primary btn-sm mt-2"
                    on:click=move |_ev: MouseEvent| open_external_url(ISSUES_URL)
                >
                    <Icon icon=GITHUB_LOGO weight=IconWeight::Bold />
                    "Open an issue on GitHub"
                </button>
            </div>
        </div>
    }
}

/// Opens in the app a link of the browser version of swarmy, i.e. one sent by someone: the view and
/// what it shows. The data is the one of the backend of the app, it should be the server of the
/// link.
#[component]
fn OpenLink() -> impl IntoView {
    let navigation = crate::navigation::use_navigation();
    let link = RwSignal::new(String::new());
    let error = RwSignal::new(None::<&'static str>);
    let open = move || {
        let text = link.get_untracked();
        match text.split_once('#') {
            Some((_, route)) if route.starts_with('/') => {
                error.set(None);
                crate::navigation::open_route(&navigation, route);
            }
            _ => error.set(Some("This is not a link of a swarmy view, they look like http://server:7878/#/replay-detail?replay=1234")),
        }
    };
    view! {
        <div>
            <h2 class="text-base/7 font-semibold text-base-content">"Open a link"</h2>
            <p class="text-sm text-muted">
                "A link of the browser version of swarmy, the address of a view in the browser, opens the same view here."
            </p>
            <form
                class="mt-2 flex items-center gap-2"
                on:submit=move |ev| {
                    ev.prevent_default();
                    open();
                }
            >
                <input
                    type="url"
                    class="input input-sm w-96 bg-base-200 text-base-content border border-base-content/20"
                    placeholder="http://server:7878/#/player?name=Serral"
                    prop:value=move || link.get()
                    on:input=move |ev| link.set(event_target_value(&ev))
                />
                <button class="btn btn-sm" type="submit">"Open"</button>
            </form>
            <p class="mt-1 text-sm text-error" role="alert">{move || error.get()}</p>
        </div>
    }
}

/// Opens the link in the default browser, only the project links are allowed by the backend.
fn open_external_url(url: &'static str) {
    spawn_local(async move {
        #[derive(serde::Serialize)]
        struct Url {
            url: &'static str,
        }
        if let Err(err) = invoke_response("open_external_url", &Url { url }).await {
            leptos::logging::error!("Unable to open {url}: {err}");
        }
    });
}

/// A link that opens in the default browser.
#[component]
fn ExternalLink(url: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <a
            href="#"
            class="nav-link"
            title=url
            on:click=move |ev: MouseEvent| {
                ev.prevent_default();
                open_external_url(url);
            }
        >
            {label}
        </a>
    }
}
