//! Build orders, expansions, resources, engagements, macro, unit choices and chat of a replay.

use crate::components::charts::{BarChart, ChartData};
use crate::components::links::{
    MapLink, PatchLink, PlayerLink, RaceLink, ReplayLink, ResultMarker, UnitCountLinks, UnitLink,
};
use crate::components::loading::Loading;
use crate::components::units::UnitsTable;
use crate::navigation::use_navigation;
use crate::replay_list::actions::{trigger_swarmy_bevy_on_replay, trigger_swarmy_rerun_on_replay};
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use phosphor_leptos::{Icon, IconWeight, CUBE, STRATEGY};
use swarmy_common::*;

/// Build order entries after this game second are hidden unless the whole game is shown.
const BUILD_ORDER_DEFAULT_SECONDS: u32 = 300;

/// In a single game, units produced less than this are not the most effective unit.
const MIN_PRODUCED_FOR_EFFECTIVENESS: usize = 3;

/// A side that lost this many times what the other lost made a bad trade.
const BAD_TRADE_RATIO: f32 = 1.5;

const TABLE_CLASS: &str = "min-w-full text-xs text-gray-400";

#[component]
pub fn ReplayDetailView() -> impl IntoView {
    let navigation = use_navigation();
    let detail = RwSignal::new(None::<ReplayDetail>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let hide_workers = RwSignal::new(false);
    let whole_game = RwSignal::new(false);
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
            <Show when=move || navigation.replay_id.get().is_none()>
                <p class="text-sm">"Choose a replay by its ID in the Replay List."</p>
            </Show>
            <Show when=move || loading.get()>
                <Loading message="Analysing the replay..." />
            </Show>
            <ErrorMessage error />
            {move || detail.get().map(|detail| view! {
                <ReplayHeader detail=detail.clone() />
                <div class="flex gap-4 text-xs text-gray-400">
                    <label class="flex items-center gap-1">
                        <input type="checkbox" class="checkbox checkbox-xs" bind:checked=hide_workers />
                        "Hide workers in the build order"
                    </label>
                    <label class="flex items-center gap-1">
                        <input type="checkbox" class="checkbox checkbox-xs" bind:checked=whole_game />
                        {format!(
                            "Show the whole build order, not only the first {} minutes",
                            BUILD_ORDER_DEFAULT_SECONDS / 60,
                        )}
                    </label>
                </div>
                <PlayerSections players=detail.players.clone() snapshot_seconds=detail.snapshot_seconds hide_workers whole_game />
                <Engagements engagements=detail.engagements />
                <Messages messages=detail.messages />
            })}
        </div>
    }
}

#[component]
fn ReplayHeader(detail: ReplayDetail) -> impl IntoView {
    let location = detail.replay_location.clone();
    let bevy_location = detail.replay_location.clone();
    view! {
        <div class="flex items-start justify-between rounded-md border border-white/15 bg-gray-800/40 p-2">
            <div>
                <h1 class="text-xl font-semibold text-white">
                    <MapLink map_title=detail.map_title.clone() />
                </h1>
                <p class="text-sm text-gray-400">
                    "Replay "
                    <ReplayLink ext_fs_id=detail.ext_fs_id />
                    {format!(
                        " · {} · {} · patch version ",
                        detail.replay_date,
                        format_game_time(detail.duration_seconds),
                    )}
                    <PatchLink version=detail.protocol_version />
                </p>
                <p class="text-sm text-gray-300">
                    "Winner: "
                    {detail
                        .winners
                        .into_iter()
                        .map(|player_name| view! {
                            <span class="mr-2">
                                <ResultMarker result="Win".to_string() />
                                <PlayerLink player_name />
                            </span>
                        })
                        .collect_view()}
                </p>
            </div>
            <div class="flex gap-2">
                <button
                    class="btn btn-primary btn-sm"
                    title="Load Rerun Preview"
                    on:click=move |_ev: MouseEvent| trigger_swarmy_rerun_on_replay(&location)
                >
                    <Icon icon=STRATEGY weight=IconWeight::Light />
                    "Load Rerun Preview"
                </button>
                <button
                    class="btn btn-primary btn-sm"
                    title="The units born and died over time on the map, with the players, upgrades and most effective units"
                    on:click=move |_ev: MouseEvent| trigger_swarmy_bevy_on_replay(&bevy_location)
                >
                    <Icon icon=CUBE weight=IconWeight::Light />
                    "Load in swarmy-bevy"
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
            std::cmp::Ordering::Less => "text-green-400",
            std::cmp::Ordering::Equal => "text-gray-400",
            std::cmp::Ordering::Greater => "text-red-400",
        };
        let title = format!(
            "The average is {} in {} games of the same patch in the snapshot",
            format_game_time(average.seconds),
            average.samples,
        );
        view! { <span class=format!("ml-2 {color}") title=title>{average.compare(seconds)}</span> }
    })
}

/// The grid columns of a band, one per player.
fn player_columns(players: usize) -> &'static str {
    match players {
        0 | 1 => "grid-cols-1",
        2 => "grid-cols-2",
        3 => "grid-cols-3",
        _ => "grid-cols-4",
    }
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
            view! { <div class=format!("scroll-section px-2 py-1 {height}")>{content(summary)}</div> }
        })
        .collect_view();
    view! {
        <div class="rounded-md border border-white/15 bg-gray-800/40">
            <h3 class="px-2 py-1 text-sm font-semibold text-white border-b border-white/15 bg-gray-800/80 rounded-t-md">
                {title}
            </h3>
            <div class=format!("grid {} divide-x divide-white/15", player_columns(players.len()))>
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
    let all_players = players.clone();
    let player_headers = players
        .iter()
        .map(|summary| {
            let [r, g, b, _] = summary.player.color;
            view! {
                <div class="px-2 py-1">
                    <div class="flex items-center gap-2 text-base text-white">
                        <span class="inline-block w-3 h-3 rounded-full" style=format!("background: rgb({r},{g},{b})")></span>
                        <ResultMarker result=summary.player.result.clone() />
                        <PlayerLink player_name=summary.player.name.clone() />
                        <span class="text-sm text-gray-400"><RaceLink race=summary.player.race.clone() /></span>
                        {summary.rating.map(|rating| view! { <span class="text-xs text-gray-400">{format!("MMR {rating}")}</span> })}
                    </div>
                    <PlayerBadges summary=summary.clone() />
                </div>
            }
        })
        .collect_view();
    view! {
        <div class="space-y-2">
            <div class=format!(
                "sticky top-0 z-10 grid {} divide-x divide-white/15 rounded-md border border-white/20 bg-gray-900",
                player_columns(players.len()),
            )>
                {player_headers}
            </div>

            {section_band("Expansions", "h-24", &players, move |summary| view! {
                <ul class="text-xs text-gray-400 font-mono">
                    {base_events(summary)
                        .into_iter()
                        .map(|(seconds, new_base, text, average)| view! {
                            <li>
                                <ResultMarker result=if new_base { "Win" } else { "Loss" }.to_string() />
                                {format!("{} {text}", format_game_time(seconds))}
                                <PatchComparison seconds average />
                            </li>
                        })
                        .collect_view()}
                </ul>
            })}

            {section_band("Resources", "h-24", &players, move |summary| {
                let resources = summary.resources.clone();
                view! {
                    <table class=TABLE_CLASS>
                        <tbody>
                            <tr>
                                <td class="pr-2">"Lost"</td>
                                <td class="pr-2 text-red-400">{format!("{} minerals, {} vespene", resources.lost.minerals, resources.lost.vespene)}</td>
                            </tr>
                            <tr>
                                <td class="pr-2">"Killed"</td>
                                <td class="pr-2 text-green-400">{format!("{} minerals, {} vespene", resources.killed.minerals, resources.killed.vespene)}</td>
                            </tr>
                            <tr title="The time needed to mine the lost resources back, at the income when they were lost">
                                <td class="pr-2">"Mining time lost"</td>
                                <td class="pr-2">{format_game_time(resources.mining_seconds_lost)}</td>
                            </tr>
                            <tr title="The build time of the units and structures lost">
                                <td class="pr-2">"Build time lost"</td>
                                <td class="pr-2">{format_game_time(resources.build_seconds_lost)}</td>
                            </tr>
                        </tbody>
                    </table>
                }
            })}

            {section_band("Macro", "h-56", &players, move |summary| view! {
                <MacroDetails macro_summary=summary.macro_summary.clone() chart_scale />
            })}

            {section_band("Focus", "h-40", &players, move |summary| view! {
                <FocusDetails focus=summary.focus.clone() chart_scale />
            })}

            {section_band("Economy", "h-28", &players, move |summary| {
                let enemy_benchmarks = enemy_benchmarks(&all_players, summary);
                view! {
                    <table class=TABLE_CLASS>
                        <thead>
                            <tr class="text-left text-gray-300">
                                <th class="pr-2">"Time"</th>
                                <th class="pr-2">"Workers"</th>
                                <th class="pr-2">"Supply"</th>
                                <th class="pr-2">"Income/min"</th>
                                <th>"Army value"</th>
                            </tr>
                        </thead>
                        <tbody class="font-mono">
                            {summary
                                .benchmarks
                                .iter()
                                .map(|benchmark| {
                                    let enemy = enemy_benchmarks
                                        .iter()
                                        .find(|enemy| enemy.minute == benchmark.minute);
                                    let ahead = |value: fn(&Benchmark) -> i32| {
                                        let value_of = value(benchmark);
                                        view! {
                                            {value_of}
                                            {enemy
                                                .is_some_and(|enemy| value_of > value(enemy))
                                                .then(|| view! {
                                                    <span class="text-green-500" title="Ahead of the enemy">" ▲"</span>
                                                })}
                                        }
                                    };
                                    view! {
                                        <tr>
                                            <td class="pr-2">{format_game_time(benchmark.minute * 60)}</td>
                                            <td class="pr-2">{ahead(|benchmark| benchmark.workers)}</td>
                                            <td class="pr-2">{ahead(|benchmark| benchmark.supply)}</td>
                                            <td class="pr-2">{ahead(|benchmark| benchmark.minerals_rate + benchmark.vespene_rate)}</td>
                                            <td>{ahead(|benchmark| benchmark.army_value)}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
            })}

            {section_band(
                if snapshot_seconds < 600 {
                    format!("Army at the end of the game ({})", format_game_time(snapshot_seconds))
                } else {
                    "Army at 10:00".to_string()
                },
                "h-28",
                &players, move |summary| view! {
                <p class="text-xs text-gray-300">{format!("{} bases", summary.bases_at_10)}</p>
                <UnitCountsTable counts=summary.army_at_10.clone() />
            })}

            {section_band("Unit choices", "h-36", &players, move |summary| {
                let total_decisions =
                    summary.good_decisions + summary.even_decisions + summary.bad_decisions.len();
                view! {
                    <p class="text-xs text-gray-400" title="Units produced scored against the enemy army at the time, from the balance data strengths and weaknesses">
                        <span class="text-green-500">{format!("{} good", summary.good_decisions)}</span>
                        {format!(" · {} even · ", summary.even_decisions)}
                        <span class="text-red-500">{format!("{} bad", summary.bad_decisions.len())}</span>
                        {format!(" out of {total_decisions}")}
                    </p>
                    <table class=TABLE_CLASS>
                        <tbody>
                            {summary
                                .bad_decisions
                                .iter()
                                .map(|decision| view! {
                                    <tr>
                                        <td class="pr-2 font-mono">{format_game_time(decision.seconds)}</td>
                                        <td class="pr-2 text-white"><UnitLink unit_name=decision.unit.clone() /></td>
                                        <td class="pr-2 text-red-400">{format!("{:+.2}", decision.score)}</td>
                                        <td>"weak vs " <UnitCountLinks counts=decision.weak_vs.clone() /></td>
                                    </tr>
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
            })}

            {section_band("Most effective units", "h-44", &players, move |summary| view! {
                <UnitsTable units=summary.units.clone() min_produced=MIN_PRODUCED_FOR_EFFECTIVENESS />
            })}

            {section_band("Units killed · Units lost · Killed by", "h-48", &players, move |summary| view! {
                <div class="grid grid-cols-3 gap-2">
                    <div>
                        <p class="text-xs text-gray-300">"Killed"</p>
                        <UnitCountsTable counts=summary.units_killed.clone() />
                    </div>
                    <div>
                        <p class="text-xs text-gray-300">"Lost"</p>
                        <UnitCountsTable counts=summary.units_lost.clone() />
                    </div>
                    <div>
                        <p class="text-xs text-gray-300">"Killed by"</p>
                        <UnitCountsTable counts=summary.killed_by.clone() />
                    </div>
                </div>
            })}

            {section_band("Build order", "h-96", &players, move |summary| {
                let build_order = summary.build_order.clone();
                view! {
                    <table class=TABLE_CLASS>
                        <thead class="sticky top-0 bg-gray-800">
                            <tr class="text-left text-gray-300">
                                <th class="pr-2">"Time"</th>
                                <th class="pr-2">"Supply"</th>
                                <th>"Unit, structure or upgrade"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || build_order
                                .iter()
                                .filter(|entry| !(hide_workers.get() && entry.is_worker))
                                .filter(|entry| whole_game.get() || entry.seconds <= BUILD_ORDER_DEFAULT_SECONDS)
                                .cloned()
                                .map(|entry| view! {
                                    <tr>
                                        <td class="pr-2 font-mono">{format_game_time(entry.seconds)}</td>
                                        <td class="pr-2 font-mono">{entry.supply}</td>
                                        <td class="text-white">
                                            {if entry.is_upgrade {
                                                entry.name.into_any()
                                            } else {
                                                view! { <UnitLink unit_name=entry.name /> }.into_any()
                                            }}
                                            <PatchComparison seconds=entry.seconds average=entry.patch_average />
                                        </td>
                                    </tr>
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
            })}
        </div>
    }
}

/// The opening, scouting, proxy and first kill of the player.
#[component]
fn PlayerBadges(summary: ReplayPlayerSummary) -> impl IntoView {
    let badge = "inline-block rounded bg-gray-700 px-1.5 py-0.5 text-xs text-gray-300";
    let opening = (!summary.opening.is_empty()).then(|| summary.opening.join(" → "));
    view! {
        <div class="mt-1 flex flex-wrap gap-1">
            {opening.map(|opening| view! { <span class=badge title="The first tech structures">{format!("Opening: {opening}")}</span> })}
            // The first Overlord almost always scouts, the badge only tells something for the
            // other races.
            {summary.first_scout.filter(|_| summary.player.race != "Zerg").map(|scout| view! {
                <span class=badge title="The first command targeting the enemy start location">
                    {format!("Scouted at {} with {}", format_game_time(scout.seconds), scout.text)}
                </span>
            })}
            {summary.first_kill_seconds.map(|seconds| view! {
                <span class=badge>{format!("First kill at {}", format_game_time(seconds))}</span>
            })}
            {summary.proxy.map(|proxy| view! {
                <span class="inline-block rounded bg-orange-900 px-1.5 py-0.5 text-xs text-orange-200">
                    {format!("Proxy: {} at {}", proxy.text, format_game_time(proxy.seconds))}
                </span>
            })}
            {summary.was_behind.then(|| view! {
                <span class=badge title="The army value was below 70% of the enemy one at some point after 05:00">
                    "Was behind in army"
                </span>
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
        FocusKind::Army => ("bg-red-500", "#ef4444"),
        FocusKind::Macro => ("bg-indigo-500", "#6366f1"),
        FocusKind::Economy => ("bg-yellow-500", "#eab308"),
        FocusKind::Expand => ("bg-green-500", "#22c55e"),
        FocusKind::Upgrades => ("bg-purple-500", "#a855f7"),
    }
}

/// The share of the selections and commands of each minute by what they were about.
#[component]
fn FocusDetails(focus: PlayerFocus, chart_scale: CommandsScale) -> impl IntoView {
    let total: u32 = focus.totals.iter().sum();
    let legend = FocusKind::ALL
        .iter()
        .map(|kind| {
            let actions = focus.totals[kind.index()];
            let percent = actions as f32 * 100. / total.max(1) as f32;
            view! {
                <span class="mr-2 inline-flex items-center gap-1">
                    <span class=format!("inline-block w-2 h-2 rounded-sm {}", focus_color(*kind).0)></span>
                    {format!("{} {percent:.0}%", kind.label())}
                </span>
            }
        })
        .collect_view();
    let minutes = chart_scale.minutes;
    let mut chart = ChartData::new(
        (0..minutes)
            .map(|minute| format!("{minute:02}:00"))
            .collect(),
    )
    .with_colors(&FocusKind::ALL.map(|kind| focus_color(kind).1))
    .stacked("100%");
    for kind in FocusKind::ALL {
        chart = chart.with_series(
            kind.label(),
            (0..minutes)
                .map(|minute| {
                    focus
                        .per_minute
                        .get(minute)
                        .map(|actions| actions[kind.index()] as f64)
                        .unwrap_or_default()
                })
                .collect(),
        );
    }
    view! {
        <p class="text-xs text-gray-400" title="The units selected and commanded: army units, townhalls and production, workers, townhalls built, research">
            {legend}
        </p>
        <p class="text-xs text-gray-300" title="The longest time with army selections or commands and no macro, expand or upgrade action, after 02:00">
            "Longest army only: "
            {match focus.longest_army_only {
                Some(range) => format!(
                    "{} from {} to {}",
                    format_game_time(range.end - range.start),
                    format_game_time(range.start),
                    format_game_time(range.end),
                ),
                None => "-".to_string(),
            }}
        </p>
        <BarChart data=chart class="h-32" />
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
                    <td class="font-mono">{format_game_time(macro_summary.supply_blocked_seconds)}</td>
                </tr>
                <tr title="More than 1000 minerals and vespene unspent">
                    <td class="pr-2">"Floating resources"</td>
                    <td class="font-mono">{format!("{} · {} unspent on average", format_game_time(macro_summary.floating_seconds), macro_summary.avg_unspent)}</td>
                </tr>
                <tr>
                    <td class="pr-2">"Injects · Chrono boosts · MULEs"</td>
                    <td>{format!("{} · {} · {}", macro_summary.injects, macro_summary.chrono_boosts, macro_summary.mules)}</td>
                </tr>
                <tr>
                    <td class="pr-2">"Commands per minute"</td>
                    <td>{format!("{avg_commands:.0} on average")}</td>
                </tr>
                {macro_summary
                    .production_idle
                    .iter()
                    .map(|idle| view! {
                        <tr title="The time between the end of a unit and the start of the next one">
                            <td class="pr-2">{format!("{} idle ({})", idle.structure, idle.structures)}</td>
                            <td class="font-mono">{format_game_time(idle.idle_seconds)}</td>
                        </tr>
                    })
                    .collect_view()}
            </tbody>
        </table>
        <div class="mt-1 text-[10px] text-gray-500">
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
                    .map(|count| view! {
                        <tr>
                            <td class="pr-2 text-white"><UnitLink unit_name=count.unit /></td>
                            <td>{count.count}</td>
                        </tr>
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

/// The units dying close in time, with what each side lost and who traded badly.
#[component]
fn Engagements(engagements: Vec<Engagement>) -> impl IntoView {
    let empty = engagements.is_empty();
    view! {
        <div class="rounded-md border border-white/15 bg-gray-800/40">
            <h2 class="px-2 py-1 text-base/7 font-semibold text-white border-b border-white/15 bg-gray-800/80 rounded-t-md">
                "Engagements"
            </h2>
            <div class="scroll-section max-h-96 px-2 py-1">
                <Show when=move || empty>
                    <p class="text-xs text-gray-500">"None"</p>
                </Show>
                <table class=TABLE_CLASS>
                    <thead class="sticky top-0 bg-gray-800">
                        <tr class="text-left text-gray-300">
                            <th class="pr-2">"Time"</th>
                            <th class="pr-2">"Player"</th>
                            <th class="pr-2">"Lost"</th>
                            <th class="pr-2" title="The time needed to mine the lost resources back">"Mining time"</th>
                            <th>"Units"</th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-white/10">
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
                                        // A side that lost much more than the other traded badly.
                                        let bad_trade = engagement_bad_trade(lost, most_lost, least_lost);
                                        let good_trade = lost == least_lost
                                            && engagement_bad_trade(most_lost, most_lost, least_lost);
                                        view! {
                                            <tr>
                                                <td class="pr-2 font-mono">{(idx == 0).then(|| time.clone())}</td>
                                                <td class="pr-2">
                                                    {bad_trade.then(|| view! { <span class="text-red-500" title="Bad trade">"▼ "</span> })}
                                                    {good_trade.then(|| view! { <span class="text-green-500" title="Good trade">"▲ "</span> })}
                                                    <PlayerLink player_name=loss.player />
                                                </td>
                                                <td class="pr-2">{format!("{}m {}g · {} supply", loss.resources.minerals, loss.resources.vespene, loss.supply)}</td>
                                                <td class="pr-2 font-mono">{format_game_time(loss.mining_seconds)}</td>
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
        <div class="rounded-md border border-white/15 bg-gray-800/40">
            <h2 class="px-2 py-1 text-base/7 font-semibold text-white border-b border-white/15 bg-gray-800/80 rounded-t-md">
                "Messages"
            </h2>
            <div class="px-2 py-1">
                <Show when=move || empty>
                    <p class="text-xs text-gray-500">"None"</p>
                </Show>
                <table class=TABLE_CLASS>
                    <tbody>
                        {messages
                            .into_iter()
                            .map(|event| view! {
                                <tr>
                                    <td class="pr-2 font-mono">{format_game_time(event.seconds)}</td>
                                    <td class="pr-2"><PlayerLink player_name=event.player /></td>
                                    <td class="text-white">{event.text}</td>
                                </tr>
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
