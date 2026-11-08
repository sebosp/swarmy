//! Build orders, expansions, resources, engagements, macro, unit choices and chat of a replay.

use super::notes::ReplayNotesPanel;
use crate::components::abilities::AbilityUses;
use crate::components::brand_icon::{native_title, BrandIcon};
use crate::components::charts::{AdvantageChart, BarChart, ChartData, LineChart, CHART_COLORS};
use crate::components::links::{
    MapLink, PatchLink, PlayerLink, RaceLink, ReplayLink, ResultMarker, UnitCountLinks, UnitLink,
};
use crate::components::loading::Loading;
use crate::components::units::UnitsTable;
use crate::navigation::use_navigation;
use crate::replay_list::actions::{
    trigger_swarmy_bevy_on_replay, trigger_swarmy_bevy_on_replay_at, trigger_swarmy_rerun_on_replay,
};
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{
    Icon, IconWeight, CARET_DOUBLE_RIGHT, CARET_DOWN, CARET_LEFT, CARET_RIGHT, EXPORT, LOCK_SIMPLE,
    LOCK_SIMPLE_OPEN,
};
use swarmy_common::*;

/// Build order entries after this game second are hidden unless the whole game is shown.
const BUILD_ORDER_DEFAULT_SECONDS: u32 = 300;

/// In a single game, units produced less than this are not the most effective unit.
const MIN_PRODUCED_FOR_EFFECTIVENESS: usize = 3;

/// A side that lost this many times what the other lost made a bad trade.
const BAD_TRADE_RATIO: f32 = 1.5;

const TABLE_CLASS: &str = "min-w-full text-xs text-muted";

#[component]
pub fn ReplayDetailView() -> impl IntoView {
    let navigation = use_navigation();
    let detail = RwSignal::new(None::<ReplayDetail>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let hide_workers = RwSignal::new(false);
    let whole_game = RwSignal::new(false);
    // Without a replay chosen, the newest one of the snapshot is shown.
    if navigation.replay_id.get_untracked().is_none() {
        go_to_replay(None, ReplayStep::Latest, error);
    }
    Effect::new(move |_| {
        let Some(ext_fs_id) = navigation.replay_id.get() else {
            return;
        };
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke_json::<ReplayDetail>(
                "query_replay_detail",
                &ReplayDetailQuery { ext_fs_id },
            )
            .await
            {
                Ok(response) => detail.set(Some(response)),
                Err(err) => {
                    detail.set(None);
                    error.set(Some(err));
                }
            }
            loading.set(false);
        });
    });
    view! {
        <div class="p-2 space-y-3">
            <Show when=move || navigation.replay_id.get().is_none() && error.get().is_none()>
                <Loading message="Analysing the latest replay of the snapshot..." />
            </Show>
            <Show when=move || loading.get()>
                {move || {
                    let id = navigation.replay_id.get().unwrap_or_default();
                    let message = match navigation.replay_preview.get() {
                        Some(preview) if preview.ext_fs_id == id => format!(
                            "Analysing replay {id} on {}: {}...",
                            preview.map_title,
                            preview
                                .players
                                .iter()
                                .map(|name| clean_player_name_clan_escapes(name))
                                .collect::<Vec<_>>()
                                .join(" vs "),
                        ),
                        _ => format!("Analysing replay {id}..."),
                    };
                    view! { <Loading message /> }
                }}
            </Show>
            <ErrorMessage error />
            // The replay shown until the next one is analysed is grayed out and cannot be hovered,
            // so that its charts are not read as the ones of the next replay.
            <div
                class=move || {
                    if loading.get() {
                        "space-y-3 opacity-40 pointer-events-none select-none"
                    } else {
                        "space-y-3"
                    }
                }
                aria-busy=move || loading.get().to_string()
            >
            {move || {
                detail
                    .get()
                    .map(|detail| {
                        view! {
                            <ReplayNavigation detail=detail.clone() error />
                            {crate::components::web_session::use_web_session().admin.get().then(|| view! {
                                <ReplayNotesPanel sha256=detail.sha256.clone() />
                            })}
                            <div class="flex gap-4 text-xs text-muted">
                                <label class="flex items-center gap-1">
                                    <input
                                        type="checkbox"
                                        class="checkbox checkbox-xs"
                                        bind:checked=hide_workers
                                    />
                                    "Hide workers in the build order"
                                </label>
                                <label class="flex items-center gap-1">
                                    <input
                                        type="checkbox"
                                        class="checkbox checkbox-xs"
                                        bind:checked=whole_game
                                    />
                                    {format!(
                                        "Show the whole build order, not only the first {} minutes",
                                        BUILD_ORDER_DEFAULT_SECONDS / 60,
                                    )}
                                </label>
                            </div>
                            {detail
                                .win_chances
                                .clone()
                                .map(|chances| {
                                    view! {
                                        <WinChanceChart
                                            chances
                                            players=detail.players.clone()
                                            replay_location=detail.replay_location.clone()
                                        />
                                    }
                                })}
                            {detail.creep.clone().map(|creep| view! { <CreepPanel creep /> })}
                            <PlayerSections
                                players=detail.players.clone()
                                snapshot_seconds=detail.snapshot_seconds
                                hide_workers
                                whole_game
                            />
                            <Engagements engagements=detail.engagements />
                            <Messages messages=detail.messages />
                        }
                    })
            }}
            </div>
        </div>
    }
}

/// Opens the replay before or after the current one, of the locked map and players, or the newest
/// one.
fn go_to_replay(ext_fs_id: Option<u64>, step: ReplayStep, error: RwSignal<Option<String>>) {
    let navigation = use_navigation();
    let locks = navigation.replay_locks.get_untracked();
    let query = AdjacentReplayQuery {
        ext_fs_id,
        step,
        map_title: locks.map_title,
        player_names: locks.player_names,
    };
    spawn_local(async move {
        #[derive(serde::Serialize)]
        struct Args {
            query: AdjacentReplayQuery,
        }
        match invoke_json::<Option<u64>>("query_adjacent_replay", &Args { query }).await {
            Ok(Some(ext_fs_id)) => {
                navigation.open_replay(ext_fs_id);
                navigation.replay_is_latest.set(step == ReplayStep::Latest);
            }
            Ok(None) if step == ReplayStep::Next => navigation.replay_is_latest.set(true),
            Ok(None) if step == ReplayStep::Latest => error.set(Some(
                "There are no replays in the snapshot, Optimize them in Scan.".to_string(),
            )),
            Ok(None) => error.set(Some(
                match step {
                    ReplayStep::Previous => {
                        "This is the first replay of the locked map and players."
                    }
                    _ => "This is the last replay of the locked map and players.",
                }
                .to_string(),
            )),
            Err(err) => error.set(Some(err)),
        }
    });
}

/// A lock toggle, a locked map or player is kept when going to the previous or next replay.
#[component]
fn LockToggle(locked: Signal<bool>, on_toggle: Callback<()>, title: &'static str) -> impl IntoView {
    view! {
        <button
            class=move || {
                if locked.get() { "btn btn-xs btn-accent" } else { "btn btn-xs btn-ghost" }
            }
            title=title
            on:click=move |_| on_toggle.run(())
        >
            {move || {
                if locked.get() {
                    view! { <Icon icon=LOCK_SIMPLE weight=IconWeight::Fill /> }.into_any()
                } else {
                    view! { <Icon icon=LOCK_SIMPLE_OPEN weight=IconWeight::Light /> }.into_any()
                }
            }}
        </button>
    }
}

/// The previous and next replays, of the map and players that are locked.
#[component]
fn ReplayNavigation(detail: ReplayDetail, error: RwSignal<Option<String>>) -> impl IntoView {
    let navigation = use_navigation();
    let locks = navigation.replay_locks;
    let ext_fs_id = detail.ext_fs_id;
    let map_title = detail.map_title.clone();
    let map_locked = {
        let map_title = map_title.clone();
        Signal::derive(move || locks.read().map_title.as_deref() == Some(map_title.as_str()))
    };
    // The winners first, then the losers.
    let mut ordered: Vec<&ReplayPlayerSummary> = detail.players.iter().collect();
    ordered.sort_by_key(|summary| result_order(&summary.player.result));
    let players = ordered
        .into_iter()
        .map(|summary| {
            let player_name = summary.player.name.clone();
            let locked = {
                let player_name = player_name.clone();
                Signal::derive(move || locks.read().player_names.contains(&player_name))
            };
            let toggle = {
                let player_name = player_name.clone();
                Callback::new(move |_| {
                    locks.update(|locks| {
                        if let Some(idx) = locks
                            .player_names
                            .iter()
                            .position(|name| *name == player_name)
                        {
                            locks.player_names.remove(idx);
                        } else {
                            locks.player_names.push(player_name.clone());
                        }
                    })
                })
            };
            view! {
                <span class="flex items-center gap-1">
                    <LockToggle
                        locked
                        on_toggle=toggle
                        title="Lock the player: the previous and next replays are of this player"
                    />
                    <ResultMarker result=summary.player.result.clone() />
                    <PlayerLink player_name />
                </span>
            }
        })
        .collect_view();
    let toggle_map = Callback::new(move |_| {
        let map_title = map_title.clone();
        locks.update(|locks| {
            locks.map_title = if locks.map_title.as_deref() == Some(map_title.as_str()) {
                None
            } else {
                Some(map_title)
            };
        })
    });
    let is_latest = navigation.replay_is_latest;
    let location = detail.replay_location.clone();
    let bevy_location = detail.replay_location.clone();
    let exported = RwSignal::new(String::new());
    let export = move |_ev: MouseEvent| {
        #[derive(serde::Serialize)]
        struct Args {
            ext_fs_id: u64,
        }
        exported.set("Writing the report...".to_string());
        spawn_local(async move {
            match invoke_response("export_replay_report", &Args { ext_fs_id }).await {
                Ok(response) if response.message.is_empty() => exported.set(String::new()),
                Ok(response) => exported.set(format!("Saved {}", response.message)),
                Err(err) => exported.set(err),
            }
        });
    };
    // One bar: the navigation, the replay with its map and players (each once, lockable), and the
    // actions as icons, their title says what they do.
    view! {
        <div class="flex flex-wrap items-center gap-x-4 gap-y-2 rounded-md border border-base-content/15 bg-base-200/40 p-2 text-sm">
            <div class="join">
                <button
                    class="btn btn-sm btn-primary join-item"
                    title="The previous replay by date, of the locked map and players"
                    on:click=move |_| {
                        error.set(None);
                        go_to_replay(Some(ext_fs_id), ReplayStep::Previous, error)
                    }
                >
                    <Icon icon=CARET_LEFT weight=IconWeight::Bold />
                    "Previous"
                </button>
                <button
                    class="btn btn-sm btn-primary join-item"
                    title=move || {
                        if is_latest.get() {
                            "This is the newest replay of the locked map and players"
                        } else {
                            "The next replay by date, of the locked map and players"
                        }
                    }
                    disabled=move || is_latest.get()
                    on:click=move |_| {
                        error.set(None);
                        go_to_replay(Some(ext_fs_id), ReplayStep::Next, error)
                    }
                >
                    "Next"
                    <Icon icon=CARET_RIGHT weight=IconWeight::Bold />
                </button>
                <button
                    class="btn btn-sm btn-ghost join-item"
                    title="The newest replay of the snapshot, of the locked map and players"
                    disabled=move || is_latest.get()
                    on:click=move |_| {
                        error.set(None);
                        go_to_replay(None, ReplayStep::Latest, error)
                    }
                >
                    <Icon icon=CARET_DOUBLE_RIGHT weight=IconWeight::Bold />
                </button>
            </div>
            <div>
                <span class="text-base font-semibold text-base-content">
                    "Replay " <ReplayLink ext_fs_id=detail.ext_fs_id />
                </span>
                <span class="text-muted">
                    {format!(
                        " · {} · {} · patch ",
                        detail.replay_date,
                        format_game_time(detail.duration_seconds),
                    )}
                    <PatchLink version=detail.protocol_version />
                </span>
            </div>
            <span class="flex items-center gap-1">
                <LockToggle
                    locked=map_locked
                    on_toggle=toggle_map
                    title="Lock the map: the previous and next replays are on this map"
                />
                <MapLink map_title=detail.map_title.clone() />
            </span>
            {players}
            <div class="ml-auto flex items-center gap-1">
                <span class="text-xs text-muted">{move || exported.get()}</span>
                <button
                    class="btn btn-ghost btn-sm btn-square native-only"
                    title=native_title("Open the replay in the Rerun preview: the units, the camera and the events over time")
                    on:click=move |_ev: MouseEvent| trigger_swarmy_rerun_on_replay(&location)
                >
                    <BrandIcon name="rerun" />
                </button>
                <button
                    class="btn btn-ghost btn-sm btn-square native-only"
                    title=native_title("Open the replay in swarmy-bevy: the units born and died over time on the map, with the players, upgrades and most effective units")
                    on:click=move |_ev: MouseEvent| trigger_swarmy_bevy_on_replay(&bevy_location)
                >
                    <BrandIcon name="bevy" />
                </button>
                <button
                    class="btn btn-ghost btn-sm btn-square"
                    title="Export a report: saves the game as a single HTML page to share with a coach or a friend, the win chance, the three things to fix, the economy, the build orders and the biggest fights"
                    on:click=export
                >
                    <Icon icon=EXPORT weight=IconWeight::Light size="18px" />
                </button>
            </div>
        </div>
    }
}

/// The expansions started and the bases lost, by time, with the expansions compared to the
/// average of the patch.
fn base_events(summary: &ReplayPlayerSummary) -> Vec<(u32, bool, String, Option<PatchAverage>)> {
    let mut events: Vec<(u32, bool, String, Option<PatchAverage>)> = summary
        .expansions
        .iter()
        .map(|entry| {
            let text = format!("{} at {} supply", entry.name, entry.supply);
            (entry.seconds, true, text, entry.patch_average.clone())
        })
        .chain(
            summary
                .bases_lost
                .iter()
                .map(|event| (event.seconds, false, event.text.clone(), None)),
        )
        .collect();
    events.sort_by_key(|(seconds, _, _, _)| *seconds);
    events
}

/// How the time compares to the average of the patch, green when faster, red when slower.
#[component]
fn PatchComparison(seconds: u32, average: Option<PatchAverage>) -> impl IntoView {
    average.map(|average| {
        let color = match seconds.cmp(&average.seconds) {
            std::cmp::Ordering::Less => "text-success",
            std::cmp::Ordering::Equal => "text-muted",
            std::cmp::Ordering::Greater => "text-error",
        };
        let title = format!(
            "The average is {} in {} games of the same patch in the snapshot",
            format_game_time(average.seconds),
            average.samples,
        );
        view! {
            <span class=format!("ml-2 {color}") title=title>
                {average.compare(seconds)}
            </span>
        }
    })
}

/// The grid columns of a band, one per player.
/// The columns of the players, with a gutter between two players: the page scrolls with the
/// wheel over a gutter, where the sections of the players would scroll themselves.
fn player_grid_style(players: usize) -> String {
    let columns = vec!["minmax(0,1fr)"; players.clamp(1, 4)].join(" 1.25rem ");
    format!("grid-template-columns: {columns}")
}

/// The cells of the players with a gutter between them, see [`player_grid_style`].
fn with_gutters(cells: Vec<AnyView>, header: bool) -> Vec<AnyView> {
    let mut out = Vec::with_capacity(cells.len() * 2);
    for (idx, cell) in cells.into_iter().enumerate() {
        if idx > 0 {
            out.push(
                view! {
                    <div
                        class="scroll-gutter flex items-center justify-center text-faint text-xs select-none"
                        title="Scroll the page here, without scrolling a section"
                    >
                        {header.then_some("⇕")}
                    </div>
                }
                .into_any(),
            );
        }
        out.push(cell);
    }
    out
}

/// A band with the same section of every player side by side, the cells of a band share the grid
/// row so the bands line up whatever the content of each player.
fn section_band<V: IntoView + 'static>(
    title: impl IntoView + 'static,
    height: &'static str,
    players: &[ReplayPlayerSummary],
    content: impl Fn(&ReplayPlayerSummary) -> V,
) -> impl IntoView {
    let cells = players
        .iter()
        .map(|summary| {
            view! { <div class=format!("scroll-section min-w-0 px-2 py-1 {height}")>{content(summary)}</div> }
                .into_any()
        })
        .collect::<Vec<_>>();
    let cells = with_gutters(cells, false);
    // The arrow collapses or expands the band, as the sections of the other views.
    let collapsed = RwSignal::new(false);
    view! {
        <div class="rounded-md border border-base-content/15 bg-base-200/40">
            <h3
                class="flex items-center gap-1 px-2 py-1 text-sm font-semibold text-base-content bg-base-200/80 rounded-t-md"
                class:border-b=move || !collapsed.get()
                style="border-color: color-mix(in oklab, var(--color-base-content) 15%, transparent)"
            >
                <button
                    type="button"
                    class="btn btn-ghost btn-xs btn-square"
                    aria-expanded=move || (!collapsed.get()).to_string()
                    title=move || if collapsed.get() { "Expand this section" } else { "Collapse this section" }
                    on:click=move |_| collapsed.update(|collapsed| *collapsed = !*collapsed)
                >
                    {move || {
                        let icon = if collapsed.get() { CARET_RIGHT } else { CARET_DOWN };
                        view! { <Icon icon=icon weight=IconWeight::Bold size="14px" /> }
                    }}
                </button>
                {title}
            </h3>
            <div class="grid" class:hidden=move || collapsed.get() style=player_grid_style(players.len())>
                {cells}
            </div>
        </div>
    }
}

/// The sections of the players as bands, each band compares the players side by side.
#[component]
fn PlayerSections(
    players: Vec<ReplayPlayerSummary>,
    /// The game second of the army and bases, 10:00 or the end of shorter games.
    snapshot_seconds: u32,
    hide_workers: RwSignal<bool>,
    whole_game: RwSignal<bool>,
) -> impl IntoView {
    let chart_scale = CommandsScale::of_players(&players);
    // The kinds of the Focus shown, the same for every player so that they compare.
    let focus_shown = RwSignal::new(Vec::<FocusKind>::new());
    let all_players = players.clone();
    let player_headers = players
        .iter()
        .map(|summary| {
            let [r, g, b, _] = summary.player.color;
            view! {
                <div class="px-2 py-1">
                    <div class="flex items-center gap-2 text-base text-base-content">
                        <span
                            class="inline-block w-3 h-3 rounded-full"
                            style=format!("background: rgb({r},{g},{b})")
                        ></span>
                        <ResultMarker result=summary.player.result.clone() />
                        <PlayerLink player_name=summary.player.name.clone() />
                        <span class="text-sm text-muted">
                            <RaceLink race=summary.player.race.clone() />
                        </span>
                        {summary
                            .rating
                            .map(|rating| {
                                view! {
                                    <span class="text-xs text-muted">
                                        {format!("MMR {rating}")}
                                    </span>
                                }
                            })}
                    </div>
                    <PlayerBadges summary=summary.clone() />
                </div>
            }
            .into_any()
        })
        .collect::<Vec<_>>();
    view! {
        <div class="space-y-2">
            <div
                class="sticky top-0 z-10 grid rounded-md border border-base-content/20 bg-base-100"
                style=player_grid_style(players.len())
            >
                {with_gutters(player_headers, true)}
            </div>

            {section_band(
                view! {
                    <span title="What cost the most minerals and vespene in this game, estimated so that they compare: a supply block costs the income of its seconds, idle production 2 per second, the missed injects, chrono boosts and MULEs what they are worth, a lost fight what was lost more than the enemy">
                        "Three things to fix"
                    </span>
                },
                "h-28",
                &players,
                move |summary| {
                    if summary.player.is_computer {
                        return view! { <p class="text-xs text-faint">"A computer player"</p> }
                            .into_any();
                    }
                    if summary.lessons.is_empty() {
                        return view! { <p class="text-xs text-faint">"Nothing costly"</p> }
                            .into_any();
                    }
                    view! {
                        <ol class="text-xs text-muted list-decimal list-inside space-y-1">
                            {summary
                                .lessons
                                .iter()
                                .map(|lesson| {
                                    let (title_help, cost_help) = lesson_help(&lesson.title, lesson.cost);
                                    view! {
                                        <li>
                                            <span class="font-semibold text-warning cursor-help" title=title_help>
                                                {lesson.title.clone()}
                                            </span>
                                            <span class="text-faint cursor-help" title=cost_help>
                                                {format!(" ~{}", format_resources(lesson.cost))}
                                            </span>
                                            ": "
                                            {lesson.detail.clone()}
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ol>
                    }
                        .into_any()
                },
            )}

            {section_band(
                view! {
                    <span title="In the 1v1 games of the snapshot, the openings of the race (its first three tech structures) that won the most against the opening of the opponent in this game, from at least 5 games. The win rates are pulled towards 50% by 5 wins and 5 losses, so that few games do not rank first">
                        "Against the opponent's opening"
                    </span>
                },
                "h-28",
                &players,
                move |summary| {
                    let answers = summary.opening_answers.clone();
                    if answers.answers.is_empty() {
                        return view! {
                            <p class="text-xs text-faint">
                                "Not a 1v1, or the opening of the opponent is played too little"
                            </p>
                        }
                            .into_any();
                    }
                    let own = summary.opening.join(" → ");
                    view! {
                        <p class="text-xs text-muted" title="The opening of the opponent in this game, the openings below answered it best">
                            "vs "
                            <span class="text-error font-medium">{answers.enemy_opening.clone()}</span>
                        </p>
                        <table class=TABLE_CLASS>
                            <tbody>
                                {answers
                                    .answers
                                    .iter()
                                    .map(|answer| {
                                        let chosen = answer.opening == own;
                                        let class = if chosen { "pr-2 text-warning" } else { "pr-2" };
                                        view! {
                                            <tr title=chosen.then_some("Chosen build order: the opening of the player in this game")>
                                                <td class=class>{answer.opening.clone()}</td>
                                                <td class="pr-2 num">
                                                    {format!(
                                                        "{:.0}% of {}",
                                                        100. * answer.win_rate(),
                                                        answer.games,
                                                    )}
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                                {answers
                                    .own
                                    .filter(|own| !answers.answers.contains(own))
                                    .map(|own| {
                                        view! {
                                            <tr title="Chosen build order: the opening of the player in this game, not among the best answers">
                                                <td class="pr-2 text-warning">{own.opening.clone()}</td>
                                                <td class="pr-2 num">
                                                    {format!("{:.0}% of {}", 100. * own.win_rate(), own.games)}
                                                </td>
                                            </tr>
                                        }
                                    })}
                            </tbody>
                        </table>
                    }
                        .into_any()
                },
            )}

            {section_band(
                "Expansions",
                "h-24",
                &players,
                move |summary| {
                    view! {
                        <ul class="text-xs text-muted font-mono">
                            {base_events(summary)
                                .into_iter()
                                .map(|(seconds, new_base, text, average)| {
                                    view! {
                                        <li>
                                            <ResultMarker result=if new_base { "Win" } else { "Loss" }
                                                .to_string() />
                                            {format!("{} {text}", format_game_time(seconds))}
                                            <PatchComparison seconds average />
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                },
            )}

            {section_band(
                "Resources",
                "h-24",
                &players,
                move |summary| {
                    let resources = summary.resources.clone();
                    view! {
                        <table class=TABLE_CLASS>
                            <tbody>
                                <tr>
                                    <td class="pr-2">"Lost"</td>
                                    <td class="pr-2 text-error">
                                        {format!(
                                            "{} minerals, {} vespene",
                                            resources.lost.minerals,
                                            resources.lost.vespene,
                                        )}
                                    </td>
                                </tr>
                                <tr>
                                    <td class="pr-2">"Killed"</td>
                                    <td class="pr-2 text-success">
                                        {format!(
                                            "{} minerals, {} vespene",
                                            resources.killed.minerals,
                                            resources.killed.vespene,
                                        )}
                                    </td>
                                </tr>
                                <tr title="The time needed to mine the lost resources back, at the income when they were lost">
                                    <td class="pr-2">"Mining time lost"</td>
                                    <td class="pr-2">
                                        {format_game_time(resources.mining_seconds_lost)}
                                    </td>
                                </tr>
                                <tr title="The build time of the units and structures lost">
                                    <td class="pr-2">"Build time lost"</td>
                                    <td class="pr-2">
                                        {format_game_time(resources.build_seconds_lost)}
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                    }
                },
            )}

            {section_band(
                "Macro",
                "h-56",
                &players,
                move |summary| {
                    view! {
                        <MacroDetails macro_summary=summary.macro_summary.clone() chart_scale />
                    }
                },
            )}

            {section_band(
                view! {
                    <span title="The injects, chrono boosts and MULEs until 10:00 compared to what the energy of the casters allowed (0.7875 energy per second, an inject per hatchery every 29 seconds). The casters also spend energy on creep tumors and transfuse, 100% is not expected">
                        "Energy used until 10:00"
                    </span>
                },
                "h-20",
                &players,
                move |summary| {
                    if summary.ability_efficiency.is_empty() {
                        return view! {
                            <p class="text-xs text-faint">
                                "No casters, or the commands of this patch have no names yet"
                            </p>
                        }
                            .into_any();
                    }
                    view! {
                        <div class="space-y-2">
                            {summary
                                .ability_efficiency
                                .iter()
                                .map(|efficiency| view! { <EnergyBar efficiency=efficiency.clone() /> })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                },
            )}

            {section_band(
                view! {
                    <span title="Control groups give quick access to armies and structures, the camera moves (minimap clicks, screen edge, camera hotkeys) show how the attention moves over the map">
                        "Control groups and camera"
                    </span>
                },
                "h-16",
                &players,
                move |summary| {
                    let control = summary.control.clone();
                    let groups = control
                        .groups
                        .iter()
                        .map(|group| group.to_string())
                        .collect::<Vec<_>>()
                        .join(" ");
                    view! {
                        <table class=TABLE_CLASS>
                            <tbody>
                                <tr>
                                    <td class="pr-2">"Groups"</td>
                                    <td class="pr-2 font-mono">
                                        {if groups.is_empty() {
                                            "none".to_string()
                                        } else {
                                            groups
                                        }}
                                    </td>
                                    <td class="pr-2">{format!("{} assigns", control.assigns)}</td>
                                </tr>
                                <tr>
                                    <td class="pr-2">"Per minute"</td>
                                    <td class="pr-2">
                                        {format!("{:.0} recalls", control.recalls_per_minute)}
                                    </td>
                                    <td class="pr-2">
                                        {format!(
                                            "{:.0} camera moves",
                                            control.camera_moves_per_minute,
                                        )}
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                    }
                },
            )}

            {section_band(
                "Abilities",
                "h-48",
                &players,
                move |summary| view! { <AbilityUses abilities=summary.abilities.clone() race=summary.player.race.clone() /> },
            )}

            {section_band(
                "Focus",
                "h-40",
                &players,
                move |summary| view! { <FocusDetails focus=summary.focus.clone() chart_scale shown=focus_shown /> },
            )}

            {section_band(
                "Economy",
                "h-28",
                &players,
                move |summary| {
                    let enemy_benchmarks = enemy_benchmarks(&all_players, summary);
                    view! {
                        <table class=TABLE_CLASS>
                            <thead>
                                <tr class="text-left text-muted">
                                    <th class="pr-2">"Time"</th>
                                    <th class="pr-2 num">"Workers"</th>
                                    <th class="pr-2 num">"Supply"</th>
                                    <th class="pr-2 num">"Income/min"</th>
                                    <th class="num">"Army value"</th>
                                </tr>
                                {(!summary.winner_benchmarks.is_empty())
                                    .then(|| {
                                        view! {
                                            <tr class="text-left text-faint">
                                                <th colspan="5" class="font-normal">
                                                    {format!(
                                                        "/ the median of the {} winners of the matchup in the patch",
                                                        summary.winner_games,
                                                    )}
                                                </th>
                                            </tr>
                                        }
                                    })}
                            </thead>
                            <tbody class="tabular-nums">
                                {summary
                                    .benchmarks
                                    .iter()
                                    .map(|benchmark| {
                                        let enemy = enemy_benchmarks
                                            .iter()
                                            .find(|enemy| enemy.minute == benchmark.minute);
                                        let winner = summary
                                            .winner_benchmarks
                                            .iter()
                                            .find(|winner| winner.minute == benchmark.minute)
                                            .cloned();
                                        let winner_games = summary.winner_games;
                                        let ahead = move |value: fn(&Benchmark) -> i32| {
                                            let value_of = value(benchmark);
                                            // Each part has its width, so that the values of
                                            // the rows line up whatever their digits.
                                            view! {
                                                <span class="inline-block min-w-[5ch] text-right">
                                                    {value_of}
                                                </span>
                                                {winner
                                                    .as_ref()
                                                    .map(|winner| {
                                                        view! {
                                                            <span
                                                                class="inline-block min-w-[7ch] pl-0.5 text-left text-faint"
                                                                title=format!(
                                                                    "The median of the {winner_games} players who won this matchup in the patch",
                                                                )
                                                            >
                                                                {format!("/{}", value(winner))}
                                                            </span>
                                                        }
                                                    })}
                                                <span class="inline-block w-[2ch] text-center text-success">
                                                    {if enemy.is_some_and(|enemy| value_of > value(enemy)) {
                                                        view! { <span title="Ahead of the enemy">"▲"</span> }
                                                            .into_any()
                                                    } else {
                                                        // A no-break space keeps the width of the arrow.
                                                        "\u{a0}".into_any()
                                                    }}
                                                </span>
                                            }
                                        };
                                        view! {
                                            <tr>
                                                <td class="pr-2 tabular-nums">
                                                    {format_game_time(benchmark.minute * 60)}
                                                </td>
                                                <td class="pr-2 num">{ahead(|benchmark| benchmark.workers)}</td>
                                                <td class="pr-2 num">{ahead(|benchmark| benchmark.supply)}</td>
                                                <td class="pr-2 num">
                                                    {ahead(|benchmark| {
                                                        benchmark.minerals_rate + benchmark.vespene_rate
                                                    })}
                                                </td>
                                                <td class="num">{ahead(|benchmark| benchmark.army_value)}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
                    }
                },
            )}

            {section_band(
                if snapshot_seconds < 600 {
                    format!("Army at the end of the game ({})", format_game_time(snapshot_seconds))
                } else {
                    "Army at 10:00".to_string()
                },
                "h-28",
                &players,
                move |summary| {
                    view! {
                        <p class="text-xs text-muted">
                            {format!("{} bases", summary.bases_at_10)}
                        </p>
                        <UnitCountsTable counts=summary.army_at_10.clone() />
                    }
                },
            )}

            {section_band(
                "Unit choices",
                "h-36",
                &players,
                move |summary| {
                    let total_decisions = summary.good_decisions + summary.even_decisions
                        + summary.bad_decisions.len();
                    view! {
                        <p
                            class="text-xs text-muted"
                            title="Units produced scored against the enemy army at the time, from the balance data strengths and weaknesses"
                        >
                            <span class="text-success">
                                {format!("{} good", summary.good_decisions)}
                            </span>
                            {format!(" · {} even · ", summary.even_decisions)}
                            <span class="text-error">
                                {format!("{} bad", summary.bad_decisions.len())}
                            </span>
                            {format!(" out of {total_decisions}")}
                        </p>
                        <table class=TABLE_CLASS>
                            <tbody>
                                {summary
                                    .bad_decisions
                                    .iter()
                                    .map(|decision| {
                                        view! {
                                            <tr>
                                                <td class="pr-2 num">
                                                    {format_game_time(decision.seconds)}
                                                </td>
                                                <td class="pr-2 text-base-content">
                                                    <UnitLink unit_name=decision.unit.clone() />
                                                </td>
                                                <td class="pr-2 num text-error">
                                                    {format!("{:+.2}", decision.score)}
                                                </td>
                                                <td>
                                                    "weak vs "
                                                    <UnitCountLinks counts=decision.weak_vs.clone() />
                                                </td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
                    }
                },
            )}

            {section_band(
                "Most effective units",
                "h-44",
                &players,
                move |summary| {
                    view! {
                        <UnitsTable
                            units=summary.units.clone()
                            min_produced=MIN_PRODUCED_FOR_EFFECTIVENESS
                        />
                    }
                },
            )}

            {section_band(
                "Units killed · Units lost · Killed by",
                "h-48",
                &players,
                move |summary| {
                    view! {
                        <div class="grid grid-cols-3 gap-2">
                            <div>
                                <p class="text-xs text-muted">"Killed"</p>
                                <UnitCountsTable counts=summary.units_killed.clone() />
                            </div>
                            <div>
                                <p class="text-xs text-muted">"Lost"</p>
                                <UnitCountsTable counts=summary.units_lost.clone() />
                            </div>
                            <div>
                                <p class="text-xs text-muted">"Killed by"</p>
                                <UnitCountsTable counts=summary.killed_by.clone() />
                            </div>
                        </div>
                    }
                },
            )}

            {section_band(
                "Build order",
                "h-96",
                &players,
                move |summary| {
                    let build_order = summary.build_order.clone();
                    view! {
                        <table class=TABLE_CLASS>
                            <thead class="sticky top-0 bg-base-200">
                                <tr class="text-left text-muted">
                                    <th class="pr-2">"Time"</th>
                                    <th class="pr-2 num">"Supply"</th>
                                    <th>"Unit, structure or upgrade"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {move || {
                                    build_order
                                        .iter()
                                        .filter(|entry| !(hide_workers.get() && entry.is_worker))
                                        .filter(|entry| {
                                            whole_game.get()
                                                || entry.seconds <= BUILD_ORDER_DEFAULT_SECONDS
                                        })
                                        .cloned()
                                        .map(|entry| {
                                            view! {
                                                <tr>
                                                    <td class="pr-2 font-mono">
                                                        {format_game_time(entry.seconds)}
                                                    </td>
                                                    <td class="pr-2 num">{entry.supply}</td>
                                                    <td class="text-base-content">
                                                        {if entry.is_upgrade {
                                                            entry.name.into_any()
                                                        } else {
                                                            view! { <UnitLink unit_name=entry.name /> }.into_any()
                                                        }}
                                                        <PatchComparison
                                                            seconds=entry.seconds
                                                            average=entry.patch_average
                                                        />
                                                    </td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </tbody>
                        </table>
                    }
                },
            )}
        </div>
    }
}

/// The opening, scouting, proxy and first kill of the player.
#[component]
fn PlayerBadges(summary: ReplayPlayerSummary) -> impl IntoView {
    let badge = "inline-block rounded bg-base-300 px-1.5 py-0.5 text-xs text-muted";
    let opening = (!summary.opening.is_empty()).then(|| summary.opening.join(" → "));
    view! {
        <div class="mt-1 flex flex-wrap gap-1">
            {opening
                .map(|opening| {
                    view! {
                        <span class=badge title="The first tech structures">
                            {format!("Opening: {opening}")}
                        </span>
                    }
                })}
            // The first Overlord almost always scouts, the badge only tells something for the
            // other races.
            {summary
                .first_scout
                .filter(|_| summary.player.race != "Zerg")
                .map(|scout| {
                    view! {
                        <span
                            class=badge
                            title="The first command targeting the enemy start location"
                        >
                            {format!(
                                "Scouted at {} with {}",
                                format_game_time(scout.seconds),
                                scout.text,
                            )}
                        </span>
                    }
                })}
            {summary
                .first_kill_seconds
                .map(|seconds| {
                    view! {
                        <span class=badge>
                            {format!("First kill at {}", format_game_time(seconds))}
                        </span>
                    }
                })}
            {summary
                .proxy
                .map(|proxy| {
                    view! {
                        <span class="inline-block rounded bg-warning/20 px-1.5 py-0.5 text-xs text-warning">
                            {format!(
                                "Proxy: {} at {}",
                                proxy.text,
                                format_game_time(proxy.seconds),
                            )}
                        </span>
                    }
                })}
            {summary
                .was_behind
                .then(|| {
                    view! {
                        <span
                            class=badge
                            title="The army value was below 70% of the enemy one at some point after 05:00"
                        >
                            "Was behind in army"
                        </span>
                    }
                })}
        </div>
    }
}

/// The scale of the commands per minute charts, shared by the players so that the bars compare.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CommandsScale {
    max_commands: u32,
    minutes: usize,
}

impl CommandsScale {
    fn of_players(players: &[ReplayPlayerSummary]) -> Self {
        let commands = players
            .iter()
            .map(|player| &player.macro_summary.commands_per_minute);
        Self {
            max_commands: commands
                .clone()
                .flat_map(|commands| commands.iter().copied())
                .max()
                .unwrap_or(0)
                .max(1),
            minutes: commands.map(Vec::len).max().unwrap_or(0).max(1),
        }
    }
}

/// The best benchmark of the enemies of the player at each minute, the income is kept as a whole
/// in `minerals_rate`.
fn enemy_benchmarks(
    players: &[ReplayPlayerSummary],
    summary: &ReplayPlayerSummary,
) -> Vec<Benchmark> {
    let mut best: Vec<Benchmark> = vec![];
    for benchmark in players
        .iter()
        .filter(|enemy| enemy.player.team_id != summary.player.team_id)
        .flat_map(|enemy| enemy.benchmarks.iter())
    {
        let income = benchmark.minerals_rate + benchmark.vespene_rate;
        match best.iter_mut().find(|best| best.minute == benchmark.minute) {
            Some(best) => {
                best.workers = best.workers.max(benchmark.workers);
                best.supply = best.supply.max(benchmark.supply);
                best.minerals_rate = best.minerals_rate.max(income);
                best.army_value = best.army_value.max(benchmark.army_value);
            }
            None => best.push(Benchmark {
                minerals_rate: income,
                vespene_rate: 0,
                ..benchmark.clone()
            }),
        }
    }
    best
}

/// The legend class of each [`FocusKind`] and its color in the focus chart.
fn focus_color(kind: FocusKind) -> (&'static str, &'static str) {
    match kind {
        FocusKind::Army => ("bg-error", "var(--color-error)"),
        FocusKind::Macro => ("bg-primary", "var(--color-primary)"),
        FocusKind::Economy => ("bg-warning", "var(--color-warning)"),
        FocusKind::Expand => ("bg-success", "var(--color-success)"),
        FocusKind::Upgrades => ("bg-secondary", "var(--color-secondary)"),
    }
}

/// The share of the selections and commands of each minute by what they were about. The legend
/// filters the chart as in Grafana: a click shows only that kind, a click on another adds it, a
/// click on the only one shown shows them all again. With them all the bars are the share of each
/// minute, with some of them the actions.
#[component]
fn FocusDetails(
    focus: PlayerFocus,
    chart_scale: CommandsScale,
    /// The kinds shown, all when empty, shared by the players.
    shown: RwSignal<Vec<FocusKind>>,
) -> impl IntoView {
    let total: u32 = focus.totals.iter().sum();
    let toggle = move |kind: FocusKind| {
        shown.update(|shown| {
            if shown.is_empty() {
                shown.push(kind);
            } else if let Some(idx) = shown.iter().position(|other| *other == kind) {
                shown.remove(idx);
            } else {
                shown.push(kind);
            }
            if shown.len() == FocusKind::ALL.len() {
                shown.clear();
            }
        })
    };
    let is_shown =
        move |kind: FocusKind| shown.with(|shown| shown.is_empty() || shown.contains(&kind));
    let legend = FocusKind::ALL
        .iter()
        .map(|kind| {
            let kind = *kind;
            let actions = focus.totals[kind.index()];
            let percent = actions as f32 * 100. / total.max(1) as f32;
            view! {
                <button
                    type="button"
                    class="inline-flex items-center gap-1 rounded px-1 hover:bg-base-300"
                    class:opacity-40=move || !is_shown(kind)
                    aria-pressed=move || is_shown(kind).to_string()
                    title=format!(
                        "{}: {actions} actions, {percent:.0}% of the game. Click to show only it, click others to add them",
                        kind.label(),
                    )
                    on:click=move |_| toggle(kind)
                >
                    <span class="inline-block w-2 h-2 rounded-sm" style=format!("background: {}", focus_color(kind).1)></span>
                    <span class="text-base-content">{kind.label()}</span>
                    <span class="tabular-nums">{format!("{percent:.0}%")}</span>
                </button>
            }
        })
        .collect_view();
    let minutes = chart_scale.minutes;
    let per_minute = focus.per_minute.clone();
    let chart = move || {
        let kinds: Vec<FocusKind> = FocusKind::ALL
            .into_iter()
            .filter(|kind| is_shown(*kind))
            .collect();
        let all = kinds.len() == FocusKind::ALL.len();
        let actions = |minute: usize, kind: FocusKind| {
            per_minute
                .get(minute)
                .map(|actions| actions[kind.index()])
                .unwrap_or_default()
        };
        // The tooltip of each minute in the colors of the bars, with the share and the actions.
        let tooltips = (0..minutes)
            .map(|minute| {
                let minute_total: u32 = kinds.iter().map(|kind| actions(minute, *kind)).sum();
                let rows = kinds
                    .iter()
                    .rev()
                    .map(|kind| {
                        let count = actions(minute, *kind);
                        format!(
                            "<div><span style=\"display:inline-block;width:8px;height:8px;border-radius:2px;margin-right:4px;background:{}\"></span>{} {:.0}% ({count})</div>",
                            focus_color(*kind).1,
                            kind.label(),
                            100. * count as f32 / minute_total.max(1) as f32,
                        )
                    })
                    .collect::<String>();
                format!("<div>{minute:02}:00, {minute_total} actions</div>{rows}")
            })
            .collect();
        let mut data = ChartData::new(
            (0..minutes)
                .map(|minute| format!("{minute:02}:00"))
                .collect(),
        )
        .with_owned_colors(
            kinds
                .iter()
                .map(|kind| focus_color(*kind).1.to_string())
                .collect(),
        )
        .with_tooltips(tooltips)
        .without_legend()
        .stacked(if all { "100%" } else { "normal" });
        for kind in &kinds {
            data = data.with_series(
                kind.label(),
                (0..minutes)
                    .map(|minute| actions(minute, *kind) as f64)
                    .collect(),
            );
        }
        view! { <BarChart data class="h-32" /> }
    };
    view! {
        <p
            class="text-xs text-muted flex flex-wrap items-center gap-x-1"
            title="The units selected and commanded: army units, townhalls and production, workers, townhalls built, research"
        >
            {legend}
        </p>
        <p
            class="text-xs text-muted"
            title="The longest time with army selections or commands and no macro, expand or upgrade action, after 02:00"
        >
            "Longest army only: "
            {match focus.longest_army_only {
                Some(range) => {
                    format!(
                        "{} from {} to {}",
                        format_game_time(range.end - range.start),
                        format_game_time(range.start),
                        format_game_time(range.end),
                    )
                }
                None => "-".to_string(),
            }}
        </p>
        {chart}
    }
}

#[component]
fn MacroDetails(macro_summary: MacroSummary, chart_scale: CommandsScale) -> impl IntoView {
    let commands = &macro_summary.commands_per_minute;
    let avg_commands = commands.iter().sum::<u32>() as f32 / commands.len().max(1) as f32;
    // The same scale for all the players, so that the bars compare.
    let chart = ChartData::new(
        (0..chart_scale.minutes)
            .map(|minute| format!("{minute:02}:00"))
            .collect(),
    )
    .with_series(
        "Commands",
        (0..chart_scale.minutes)
            .map(|minute| commands.get(minute).copied().unwrap_or_default() as f64)
            .collect(),
    )
    .with_y_max(chart_scale.max_commands as f64);
    view! {
        <table class=TABLE_CLASS>
            <tbody>
                <tr title="All the supply used, below 200">
                    <td class="pr-2">"Supply blocked"</td>
                    <td class="font-mono">
                        {format_game_time(macro_summary.supply_blocked_seconds)}
                    </td>
                </tr>
                <tr title="More than 1000 minerals and vespene unspent">
                    <td class="pr-2">"Floating resources"</td>
                    <td class="font-mono">
                        {format!(
                            "{} · {} unspent on average",
                            format_game_time(macro_summary.floating_seconds),
                            macro_summary.avg_unspent,
                        )}
                    </td>
                </tr>
                <tr>
                    <td class="pr-2">"Injects · Chrono boosts · MULEs"</td>
                    <td>
                        {format!(
                            "{} · {} · {}",
                            macro_summary.injects,
                            macro_summary.chrono_boosts,
                            macro_summary.mules,
                        )}
                    </td>
                </tr>
                <tr>
                    <td class="pr-2">"Commands per minute"</td>
                    <td>{format!("{avg_commands:.0} on average")}</td>
                </tr>
                {macro_summary
                    .production_idle
                    .iter()
                    .map(|idle| {
                        view! {
                            <tr title="The time between the end of a unit and the start of the next one">
                                <td class="pr-2">
                                    {format!("{} idle ({})", idle.structure, idle.structures)}
                                </td>
                                <td class="font-mono">{format_game_time(idle.idle_seconds)}</td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
        <div class="mt-1 text-[10px] text-faint">
            <div class="mb-0.5">"Commands per minute"</div>
            <BarChart data=chart class="h-32" />
        </div>
    }
}

#[component]
fn UnitCountsTable(counts: Vec<UnitCount>) -> impl IntoView {
    view! {
        <table class=TABLE_CLASS>
            <tbody>
                {counts
                    .into_iter()
                    .map(|count| {
                        view! {
                            <tr>
                                <td class="pr-2 text-base-content">
                                    <UnitLink unit_name=count.unit />
                                </td>
                                <td class="num">{count.count}</td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

/// "16% more than", "45% less than" or "as much as" the average.
fn compared_to_average(value: f32, average: f32) -> String {
    match CreepSummary::compared(value, average) {
        Some(percent) if percent >= 5. => format!("{percent:.0}% more than"),
        Some(percent) if percent <= -5. => format!("{:.0}% less than", -percent),
        Some(_) => "about as much as".to_string(),
        None => "while the games of the matchup average none,".to_string(),
    }
}

/// The creep tumors the Zerg placed and the ones the opponent cleaned, against the games of the
/// matchup in the snapshot.
#[component]
fn CreepPanel(creep: CreepSummary) -> impl IntoView {
    let zerg = clean_player_name_clan_escapes(&creep.zerg);
    let opponent = clean_player_name_clan_escapes(&creep.opponent);
    let labels: Vec<String> = creep
        .timeline
        .iter()
        .map(|minute| format_game_time(minute.minute * 60))
        .collect();
    let chart = |game: Vec<f64>, average: Vec<f64>, what: &'static str| {
        let tooltips = labels
            .iter()
            .zip(game.iter().zip(average.iter()))
            .map(|(time, (game, average))| {
                format!("By {time}: {game:.0} {what} in this game, {average:.1} on average")
            })
            .collect();
        ChartData::new(labels.clone())
            .with_series("This game", game)
            .with_series(&format!("Average {}", creep.matchup), average)
            .with_colors(&[CHART_COLORS[0], "var(--chart-muted)"])
            .with_tooltips(tooltips)
            .with_curve("straight")
    };
    let placed = chart(
        creep
            .timeline
            .iter()
            .map(|minute| minute.placed as f64)
            .collect(),
        creep
            .timeline
            .iter()
            .map(|minute| (minute.avg_placed as f64 * 10.).round() / 10.)
            .collect(),
        "tumors placed",
    );
    let cleaned = chart(
        creep
            .timeline
            .iter()
            .map(|minute| minute.cleaned as f64)
            .collect(),
        creep
            .timeline
            .iter()
            .map(|minute| (minute.avg_cleaned as f64 * 10.).round() / 10.)
            .collect(),
        "tumors cleaned",
    );
    let figure = |value: String| view! { <span class="font-medium text-base-content tabular-nums">{value}</span> };
    view! {
        <div class="rounded-md border border-base-content/15 bg-base-200/40">
            <h2 class="px-2 py-1 text-base/7 font-semibold text-base-content border-b border-base-content/15 bg-base-200/80 rounded-t-md">
                "Creep"
            </h2>
            <div class="px-2 py-1 space-y-1">
                <p class="text-xs text-faint">
                    {format!(
                        "The creep tumors {zerg} placed, by the Queens and by the tumors spreading, and the ones {opponent} killed (with scans, detectors, Oracle revelations or Observers, then the army), against the {} {} games of the snapshot.",
                        creep.games,
                        creep.matchup,
                    )}
                </p>
                <p class="text-sm text-muted">
                    "Spread: "
                    {figure(creep.placed.to_string())}
                    " tumors, "
                    {figure(format!("{:.2}", creep.placed_per_minute()))}
                    " per minute, "
                    {compared_to_average(creep.placed_per_minute(), creep.avg_placed_per_minute)}
                    " the average of "
                    {figure(format!("{:.2}", creep.avg_placed_per_minute))}
                    "."
                </p>
                <p class="text-sm text-muted">
                    "Cleanup: "
                    {figure(creep.cleaned.to_string())}
                    " tumors killed, "
                    {figure(format!("{:.2}", creep.cleaned_per_minute()))}
                    " per minute, "
                    {compared_to_average(creep.cleaned_per_minute(), creep.avg_cleaned_per_minute)}
                    " the average of "
                    {figure(format!("{:.2}", creep.avg_cleaned_per_minute))}
                    "."
                </p>
                {(!creep.timeline.is_empty())
                    .then(|| {
                        view! {
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <h3 class="text-xs font-semibold text-base-content">"Tumors placed by the end of each minute"</h3>
                                    <LineChart data=placed.clone() class="h-40" />
                                </div>
                                <div>
                                    <h3 class="text-xs font-semibold text-base-content">"Tumors cleaned by the end of each minute"</h3>
                                    <LineChart data=cleaned.clone() class="h-40" />
                                </div>
                            </div>
                        }
                    })}
            </div>
        </div>
    }
}

/// What favors a side less than this, in log-odds, is not named in the tooltip.
const WIN_CHANCE_FACTOR_THRESHOLD: f32 = 0.1;

/// The chance of each team to win over the game, from its army, economy and trades.
#[component]
fn WinChanceChart(
    chances: WinChances,
    players: Vec<ReplayPlayerSummary>,
    replay_location: String,
) -> impl IntoView {
    let team_name = |team: &[String]| {
        team.iter()
            .map(|name| clean_player_name_clan_escapes(name))
            .collect::<Vec<_>>()
            .join(" & ")
    };
    // A team has the color of its first player.
    let team_color = |team: &[String]| {
        players
            .iter()
            .find(|summary| team.first() == Some(&summary.player.name))
            .map(|summary| {
                let [r, g, b, _] = summary.player.color;
                format!("rgb({r},{g},{b})")
            })
            .unwrap_or_else(|| "var(--chart-muted)".to_string())
    };
    let first = team_name(&chances.first_team);
    let second = team_name(&chances.second_team);
    let points = &chances.points;
    let tooltips = points
        .iter()
        .map(|point| {
            let factors = [
                ("army", point.army),
                ("income", point.income),
                ("workers", point.workers),
                ("trades", point.trades),
            ];
            let favoring = |sign: f32| {
                factors
                    .iter()
                    .filter(|(_, value)| value * sign > WIN_CHANCE_FACTOR_THRESHOLD)
                    .map(|(name, _)| *name)
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            // The chance of the side ahead only, the other one is the rest.
            let (ahead, chance) = if point.chance >= 0.5 {
                (&first, point.chance)
            } else {
                (&second, 1. - point.chance)
            };
            let mut tooltip = format!(
                "{} · {ahead} {:.0}%",
                format_game_time(point.seconds),
                chance * 100.,
            );
            for (team, sign) in [(&first, 1.), (&second, -1.)] {
                let factors = favoring(sign);
                if !factors.is_empty() {
                    tooltip.push_str(&format!("<br>{team} ahead in {factors}"));
                }
            }
            tooltip
        })
        .collect();
    // One line: the chance of the second team is the rest of the first one's, a line for each
    // would mirror the other.
    let data = ChartData::new(
        points
            .iter()
            .map(|point| format_game_time(point.seconds))
            .collect(),
    )
    .with_series(
        &first,
        points
            .iter()
            .map(|point| ((point.chance as f64 - 0.5) * 1000.).round() / 10.)
            .collect(),
    )
    .with_owned_colors(vec![
        team_color(&chances.first_team),
        team_color(&chances.second_team),
    ])
    .with_tooltips(tooltips);
    let empty = points.is_empty();
    // The team names say which side of the middle is whose, the color is not the only cue.
    let side_label = |arrow: &'static str, name: String, color: String| {
        view! {
            <p class="flex items-center gap-1 text-xs text-muted">
                <span class="inline-block size-2.5 rounded-sm" style=format!("background:{color}")></span>
                {format!("{arrow} {name} ahead")}
            </p>
        }
    };
    let above = side_label("▲", first.clone(), team_color(&chances.first_team));
    let below = side_label("▼", second.clone(), team_color(&chances.second_team));
    let turning_points = chances.turning_points.clone();
    view! {
        <div class="rounded-md border border-base-content/15 bg-base-200/40">
            <h2 class="px-2 py-1 text-base/7 font-semibold text-base-content border-b border-base-content/15 bg-base-200/80 rounded-t-md">
                "Win chance"
            </h2>
            <div class="px-2 py-1">
                <p class="text-xs text-muted">
                    "Who the game favors every 30 seconds, from the army value, income, workers and resources killed of each side, \
                    as they turned out in thousands of games of the snapshot. It does not see the mistakes: \
                    a side ahead can still lose a fight, take a bad engagement or blunder the game away."
                </p>
                <Show when=move || empty>
                    <p class="text-xs text-faint">"The game ended before 02:00."</p>
                </Show>
                <Show when=move || !empty>
                    {above.clone()}
                    <AdvantageChart data=data.clone() />
                    {below.clone()}
                </Show>
                {(!turning_points.is_empty())
                    .then(|| {
                        view! {
                            <h3 class="mt-1 text-sm font-semibold text-base-content">"Turning points"</h3>
                            <ul class="text-xs text-muted space-y-1">
                                {turning_points
                                    .into_iter()
                                    .map(|point| {
                                        let favored = if point.change > 0. {
                                            &first
                                        } else {
                                            &second
                                        };
                                        let location = replay_location.clone();
                                        let start = point.start_seconds;
                                        view! {
                                            <li class="flex items-center gap-2">
                                                <span class="font-mono">
                                                    {format!(
                                                        "{}-{}",
                                                        format_game_time(point.start_seconds),
                                                        format_game_time(point.end_seconds),
                                                    )}
                                                </span>
                                                <span>
                                                    {format!("{favored} +{:.0}%", point.change.abs() * 100.)}
                                                </span>
                                                <span class="text-muted min-w-0 flex-1">
                                                    // A space after each comma of the unit lists, so
                                                    // that the line wraps.
                                                    {point
                                                        .engagement
                                                        .as_deref()
                                                        .map(|text| text.replace(',', ", ").replace(",  ", ", "))
                                                        .unwrap_or_else(|| {
                                                            "no fight: the economy or the army changed".to_string()
                                                        })}
                                                </span>
                                                <button
                                                    class="btn btn-ghost btn-xs native-only"
                                                    title=native_title("Open the replay in swarmy-bevy at this moment")
                                                    on:click=move |_ev: MouseEvent| {
                                                        trigger_swarmy_bevy_on_replay_at(&location, Some(start))
                                                    }
                                                >
                                                    <BrandIcon name="bevy" />
                                                </button>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                    })}
            </div>
        </div>
    }
}

/// The units dying close in time, with what each side lost and who traded badly.
#[component]
fn Engagements(engagements: Vec<Engagement>) -> impl IntoView {
    let empty = engagements.is_empty();
    view! {
        <div class="rounded-md border border-base-content/15 bg-base-200/40">
            <h2 class="px-2 py-1 text-base/7 font-semibold text-base-content border-b border-base-content/15 bg-base-200/80 rounded-t-md">
                "Engagements"
            </h2>
            <div class="scroll-section max-h-96 px-2 py-1">
                <Show when=move || empty>
                    <p class="text-xs text-faint">"None"</p>
                </Show>
                <table class=TABLE_CLASS>
                    <thead class="sticky top-0 bg-base-200">
                        <tr class="text-left text-muted">
                            <th class="pr-2">"Time"</th>
                            <th class="pr-2">"Player"</th>
                            <th class="pr-2">"Lost"</th>
                            <th
                                class="pr-2"
                                title="The time needed to mine the lost resources back"
                            >
                                "Mining time"
                            </th>
                            <th>"Units"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-base-content/10">
                        {engagements
                            .into_iter()
                            .flat_map(|engagement| {
                                let most_lost = engagement
                                    .losses
                                    .iter()
                                    .map(|loss| loss.resources.total())
                                    .max()
                                    .unwrap_or_default();
                                let least_lost = engagement
                                    .losses
                                    .iter()
                                    .map(|loss| loss.resources.total())
                                    .min()
                                    .unwrap_or_default();
                                let time = format!(
                                    "{} - {}",
                                    format_game_time(engagement.start_seconds),
                                    format_game_time(engagement.end_seconds),
                                );
                                engagement
                                    .losses
                                    .into_iter()
                                    .enumerate()
                                    .map(|(idx, loss)| {
                                        let lost = loss.resources.total();
                                        let bad_trade = engagement_bad_trade(
                                            lost,
                                            most_lost,
                                            least_lost,
                                        );
                                        let good_trade = lost == least_lost
                                            && engagement_bad_trade(most_lost, most_lost, least_lost);
                                        // A side that lost much more than the other traded badly.
                                        view! {
                                            <tr>
                                                <td class="pr-2 font-mono">
                                                    {(idx == 0).then(|| time.clone())}
                                                </td>
                                                <td class="pr-2">
                                                    {bad_trade
                                                        .then(|| {
                                                            view! {
                                                                <span class="text-error" title="Bad trade">
                                                                    "▼ "
                                                                </span>
                                                            }
                                                        })}
                                                    {good_trade
                                                        .then(|| {
                                                            view! {
                                                                <span class="text-success" title="Good trade">
                                                                    "▲ "
                                                                </span>
                                                            }
                                                        })} <PlayerLink player_name=loss.player />
                                                </td>
                                                <td class="pr-2">
                                                    {format!(
                                                        "{}m {}g · {} supply",
                                                        loss.resources.minerals,
                                                        loss.resources.vespene,
                                                        loss.supply,
                                                    )}
                                                </td>
                                                <td class="pr-2 font-mono">
                                                    {format_game_time(loss.mining_seconds)}
                                                </td>
                                                <td>{loss.units}</td>
                                            </tr>
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

/// Losing `BAD_TRADE_RATIO` times what the other side lost is a bad trade.
fn engagement_bad_trade(lost: u32, most_lost: u32, least_lost: u32) -> bool {
    lost == most_lost && lost as f32 > least_lost as f32 * BAD_TRADE_RATIO && lost > least_lost
}

#[component]
fn Messages(messages: Vec<TimedEvent>) -> impl IntoView {
    let empty = messages.is_empty();
    view! {
        <div class="rounded-md border border-base-content/15 bg-base-200/40">
            <h2 class="px-2 py-1 text-base/7 font-semibold text-base-content border-b border-base-content/15 bg-base-200/80 rounded-t-md">
                "Messages"
            </h2>
            <div class="px-2 py-1">
                <Show when=move || empty>
                    <p class="text-xs text-faint">"None"</p>
                </Show>
                <table class=TABLE_CLASS>
                    <tbody>
                        {messages
                            .into_iter()
                            .map(|event| {
                                view! {
                                    <tr>
                                        <td class="pr-2 font-mono">
                                            {format_game_time(event.seconds)}
                                        </td>
                                        <td class="pr-2">
                                            <PlayerLink player_name=event.player />
                                        </td>
                                        <td class="text-base-content">{event.text}</td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

/// What a lesson of "Three things to fix" means and how its cost is estimated, for their titles.
/// The costs follow `src-tauri/src/replay_detail/lessons.rs`.
fn lesson_help(title: &str, cost: u32) -> (String, String) {
    let cost = crate::components::abilities::group_digits(cost as u64);
    let (what, estimate) = match title {
        "Supply blocked" => (
            "The supply used reached the supply available below 200: no unit could start until a depot, pylon or overlord finished. Build supply before you need it.",
            "the income of the seconds blocked: what the player mined in that time could not become units",
        ),
        "Idle production" => (
            "The production structures waited between the end of a unit and the start of the next one. The time the supply was full, at 200 or blocked, is not counted: nothing could be built then.",
            "2 per second of each structure idle: the value of the units it could have made",
        ),
        "Floating resources" => (
            "The minerals and vespene unspent on average after the first minute, above 1,000: they do nothing in the bank. Add production or expand.",
            "the average unspent above 1,000",
        ),
        "Lost fight" => (
            "The fight that cost the most: the units lost minus the units killed. Fight where your units counter theirs, or wait for more.",
            "the minerals and vespene lost more than the enemy in that fight",
        ),
        missed if missed.starts_with("Missed") => (
            "The casters had the energy for more uses until 10:00: each missed inject, chrono boost or MULE is larvae, production time or minerals lost.",
            "what each missed use is worth (an inject 50, a chrono boost 20, a MULE 225) times the missed uses",
        ),
        _ => ("", "the resources it cost"),
    };
    (
        what.to_string(),
        format!("About {cost} minerals and vespene lost, estimated as {estimate}. The estimates compare the mistakes: the most costly is first."),
    )
}

/// The injects, chrono boosts or MULEs of a game until 10:00 as a bar: the track is what the energy
/// of the casters allowed, the fill what the player used, the tick the average of the games of the
/// patch. A sentence compares them, the numbers are in the title of each part.
#[component]
fn EnergyBar(efficiency: AbilityEfficiency) -> impl IntoView {
    let possible = efficiency.possible.max(1.);
    let share = efficiency.uses as f32 / possible;
    let width = |share: f32| format!("{:.1}%", 100. * share.clamp(0., 1.));
    let game_title = format!(
        "This game: {} of about {:.0} possible ({:.0}%), with {:.0} minutes of {} alive and finished",
        efficiency.uses,
        efficiency.possible,
        100. * share,
        efficiency.caster_minutes,
        efficiency.caster,
    );
    let average = efficiency.patch_share;
    let comparison = average.map(|average| {
        let difference = 100. * (share - average);
        let (text, class) = if difference >= 5. {
            ("more than the patch average", "text-success")
        } else if difference <= -5. {
            ("less than the patch average", "text-error")
        } else {
            ("about the patch average", "text-info")
        };
        (text, class, average)
    });
    view! {
        <div class="text-xs">
            <div class="flex items-baseline justify-between gap-2">
                <span class="text-base-content">{efficiency.ability.clone()}</span>
                <span class="tabular-nums" title=game_title.clone()>
                    <span class="font-medium text-base-content">{efficiency.uses}</span>
                    {format!(" of ~{:.0}, ", efficiency.possible)}
                    <span class="font-medium text-base-content">{format!("{:.0}%", 100. * share)}</span>
                </span>
            </div>
            <div
                class="relative mt-0.5 h-2 rounded-full bg-base-300"
                role="meter"
                aria-label=format!("{} until 10:00", efficiency.ability)
                aria-valuemin="0"
                aria-valuemax=format!("{:.0}", efficiency.possible)
                aria-valuenow=efficiency.uses.to_string()
                title=format!("The track: about {:.0} possible with the energy of the casters", efficiency.possible)
            >
                <div
                    class="absolute inset-y-0 left-0 rounded-full"
                    style=format!("width: {}; background: var(--chart-1)", width(share))
                    title=game_title.clone()
                ></div>
                // This game: the end of the fill, a marker that shows on any fill.
                <div
                    class="absolute top-1/2 size-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-base-100"
                    style=format!("left: {}; background: var(--chart-1)", width(share))
                    title=game_title
                ></div>
                {average.map(|average| view! {
                    <div
                        class="absolute -top-1 -bottom-1 w-0.5 -translate-x-1/2 bg-base-content"
                        style=format!("left: {}", width(average))
                        title=format!(
                            "Patch average: {:.0}% of the possible uses, in {} games of the patch",
                            100. * average,
                            efficiency.patch_games,
                        )
                    ></div>
                })}
            </div>
            {comparison.map(|(text, class, average)| view! {
                <p class="mt-0.5 text-muted">
                    "This game: "
                    <span class=format!("font-medium {class}")>{text}</span>
                    {format!(" ({:.0}%)", 100. * average)}
                </p>
            })}
        </div>
    }
}
