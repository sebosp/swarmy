//! Welcome to swarmy: what it is for, where its data comes from and the versions it is built with.

use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, GITHUB_LOGO};
use swarmy_common::*;

pub const ISSUES_URL: &str = "https://github.com/sebosp/swarmy/issues";

#[component]
pub fn Home() -> impl IntoView {
    let versions = RwSignal::new(AppVersions::default());
    spawn_local(async move {
        match invoke_json::<AppVersions>("get_app_versions", &()).await {
            Ok(backend_versions) => versions.set(backend_versions),
            Err(err) => leptos::logging::error!("Unable to read the versions: {err}"),
        }
    });
    let version_row = |name: &'static str, version: Signal<String>| {
        view! {
            <tr>
                <td class="pr-4 text-gray-400">{name}</td>
                <td class="text-white"><code>{move || version.get()}</code></td>
            </tr>
        }
    };
    view! {
        <div class="max-w-3xl p-4 space-y-6">
            <div>
                <h1 class="text-2xl font-semibold text-white">
                    "Welcome to Swarmy, a StarCraft II Replay Batch Analyser"
                </h1>
                <p class="mt-2 text-sm text-gray-400">
                    "Swarmy reads a whole directory of replays at once to help you identify trends: \
                    which units, openings and timings are played on each patch and how they change \
                    over time. Casters can collect statistics about players, maps, race balance \
                    (PvT, TvZ, ZvP...) and player vs player records before a match."
                </p>
                <p class="mt-2 text-sm text-gray-400">
                    "Start in Scan: choose the replay directory and Optimize it into a snapshot. The \
                    Changelog has what is new and Help describes every view."
                </p>
            </div>
            <div>
                <h2 class="text-base/7 font-semibold text-white">"Where the data comes from"</h2>
                <ul class="mt-1 list-disc list-inside text-sm text-gray-400 space-y-1">
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
            <div>
                <h2 class="text-base/7 font-semibold text-white">"Version Dependencies"</h2>
                <table class="mt-1 text-sm">
                    <tbody>
                        {version_row("Swarmy UI", Signal::derive(|| env!("CARGO_PKG_VERSION").to_string()))}
                        {version_row("Swarmy backend", Signal::derive(move || versions.get().swarmy))}
                        {version_row("s2protocol-rs", Signal::derive(move || versions.get().s2protocol))}
                        {version_row("Rerun", Signal::derive(move || versions.get().rerun))}
                        {version_row("Polars", Signal::derive(move || versions.get().polars))}
                        {version_row("Tauri", Signal::derive(move || versions.get().tauri))}
                    </tbody>
                </table>
            </div>
            <div class="text-sm text-gray-400">
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
