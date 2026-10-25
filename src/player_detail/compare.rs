//! Two players side by side: their preferred strategies in all their games and what each one
//! played in the games between them, with links to the replays.

use crate::components::links::{MapLink, PlayerLink, RaceLink, ReplayLink, ResultMarker, UnitLink};
use crate::components::loading::Loading;
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::BTreeMap;
use swarmy_common::*;

const TABLE_CLASS: &str = "min-w-full text-xs text-gray-400";

/// The strategy tables, the columns keep their width and the text wraps inside them.
const FIXED_TABLE_CLASS: &str = "w-full table-fixed text-xs text-gray-400";

/// The choices shown for each strategy.
const TOP_CHOICES: usize = 3;

fn percent(part: usize, total: usize) -> String {
    if total == 0 {
        return "-".to_string();
    }
    format!("{:.0}%", part as f64 * 100. / total as f64)
}

/// A choice with the games it was made and won in.
#[derive(Debug, Clone, PartialEq)]
struct Choice {
    name: String,
    games: usize,
    wins: usize,
}

/// Adds up the choices of every context, i.e. the openings of every matchup, the most made first.
fn top_choices(shares: &[ChoiceShare]) -> Vec<Choice> {
    let mut choices: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for share in shares {
        let entry = choices.entry(share.choice.as_str()).or_default();
        entry.0 += share.games;
        entry.1 += share.wins;
    }
    count_choices(choices.into_iter().map(|(name, (games, wins))| Choice {
        name: name.to_string(),
        games,
        wins,
    }))
}

fn count_choices(choices: impl Iterator<Item = Choice>) -> Vec<Choice> {
    let mut choices: Vec<Choice> = choices.collect();
    choices.sort_by(|a, b| b.games.cmp(&a.games).then(a.name.cmp(&b.name)));
    choices.truncate(TOP_CHOICES);
    choices
}

/// The choices made in the games between the players, by one of them.
fn head_to_head_choices(
    games: &[HeadToHeadGame],
    side: impl Fn(&HeadToHeadGame) -> &HeadToHeadSide,
    choice: impl Fn(&HeadToHeadSide) -> Option<String>,
) -> Vec<Choice> {
    let mut choices: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for game in games {
        let side = side(game);
        if let Some(name) = choice(side) {
            let entry = choices.entry(name).or_default();
            entry.0 += 1;
            entry.1 += side.won as usize;
        }
    }
    count_choices(
        choices
            .into_iter()
            .map(|(name, (games, wins))| Choice { name, games, wins }),
    )
}

/// The choices as a list, with the share of games and the win rate.
fn choice_list(choices: Vec<Choice>, total: usize, units: bool) -> impl IntoView {
    if choices.is_empty() {
        return view! { <span>"-"</span> }.into_any();
    }
    view! {
        <ul>
            {choices
                .into_iter()
                .map(|choice| view! {
                    <li class="break-words">
                        <span class="text-white">
                            {if units {
                                view! { <UnitLink unit_name=choice.name /> }.into_any()
                            } else {
                                choice.name.into_any()
                            }}
                        </span>
                        <span class="text-gray-500">
                            {format!(
                                " · {} of games, {} won",
                                percent(choice.games, total),
                                percent(choice.wins, choice.games),
                            )}
                        </span>
                    </li>
                })
                .collect_view()}
        </ul>
    }
    .into_any()
}

fn opening(side: &HeadToHeadSide) -> Option<String> {
    (!side.opening.is_empty()).then(|| side.opening.join(" → "))
}

fn optional_time(seconds: Option<f32>) -> String {
    seconds
        .map(|seconds| format_game_time(seconds as u32))
        .unwrap_or_else(|| "-".to_string())
}

/// Fetches the second player and the games between both, and shows them side by side.
#[component]
pub fn PlayerComparison(player: PlayerDetail, opponent_name: String) -> impl IntoView {
    let opponent = RwSignal::new(None::<PlayerDetail>);
    let head_to_head = RwSignal::new(None::<HeadToHead>);
    let error = RwSignal::new(None::<String>);
    let player_name = player.player_name.clone();
    spawn_local(async move {
        let opponent_detail = invoke_json::<PlayerDetail>(
            "query_player_detail",
            &PlayerDetailQuery {
                player_name: opponent_name.clone(),
            },
        )
        .await;
        let games = invoke_json::<HeadToHead>(
            "query_head_to_head",
            &HeadToHeadQuery {
                player_name,
                opponent_name,
            },
        )
        .await;
        match (opponent_detail, games) {
            (Ok(opponent_detail), Ok(games)) => {
                opponent.set(Some(opponent_detail));
                head_to_head.set(Some(games));
            }
            (Err(err), _) | (_, Err(err)) => error.set(Some(err)),
        }
    });
    view! {
        <div class="rounded-md border border-white/15 bg-gray-800/40">
            <ErrorMessage error />
            {move || match (opponent.get(), head_to_head.get()) {
                (Some(opponent), Some(head_to_head)) => {
                    view! { <ComparisonContent player=player.clone() opponent head_to_head /> }
                        .into_any()
                }
                _ => view! { <Loading message="Analysing the games of both players..." /> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn ComparisonContent(
    player: PlayerDetail,
    opponent: PlayerDetail,
    head_to_head: HeadToHead,
) -> impl IntoView {
    let games = head_to_head.games;
    let player_wins = games.iter().filter(|game| game.player.won).count();
    // A row of the strategies table, the same strategy of both players side by side.
    let strategy_row = |label: &'static str, detail: &dyn Fn(&PlayerDetail) -> AnyView| {
        view! {
            <tr class="align-top">
                <td class="pr-3 py-1 text-gray-300">{label}</td>
                <td class="pr-3 py-1 break-words">{detail(&player)}</td>
                <td class="py-1 break-words">{detail(&opponent)}</td>
            </tr>
        }
    };
    let analyzed = |detail: &PlayerDetail| detail.insights.analyzed_games;
    let h2h_row = |label: &'static str,
                   units: bool,
                   choice: fn(&HeadToHeadSide) -> Option<String>| {
        let total = games.len();
        view! {
            <tr class="align-top">
                <td class="pr-3 py-1 text-gray-300">{label}</td>
                <td class="pr-3 py-1 break-words">
                    {choice_list(head_to_head_choices(&games, |game| &game.player, choice), total, units)}
                </td>
                <td class="py-1 break-words">
                    {choice_list(head_to_head_choices(&games, |game| &game.opponent, choice), total, units)}
                </td>
            </tr>
        }
    };
    let header = view! {
        <tr class="text-left text-white">
            <th class="pr-3 py-1"></th>
            <th class="pr-3 py-1"><PlayerLink player_name=player.player_name.clone() /></th>
            <th class="py-1"><PlayerLink player_name=opponent.player_name.clone() /></th>
        </tr>
    };
    let header_h2h = view! {
        <tr class="text-left text-white">
            <th class="pr-3 py-1"></th>
            <th class="pr-3 py-1"><PlayerLink player_name=player.player_name.clone() /></th>
            <th class="py-1"><PlayerLink player_name=opponent.player_name.clone() /></th>
        </tr>
    };
    view! {
        <h2 class="px-2 py-1 text-base/7 font-semibold text-white border-b border-white/15 bg-gray-800/80 rounded-t-md">
            {format!(
                "{} vs {}: {} games between them, {player_wins} – {}",
                clean_player_name_clan_escapes(&player.player_name),
                clean_player_name_clan_escapes(&opponent.player_name),
                games.len(),
                games.len() - player_wins,
            )}
        </h2>
        <div class="grid grid-cols-1 2xl:grid-cols-2 gap-4 p-2">
            <div class="min-w-0">
                <h3 class="text-sm font-semibold text-white">"Preferred strategies, all their games"</h3>
                <table class=FIXED_TABLE_CLASS>
                    <colgroup><col class="w-40" /><col /><col /></colgroup>
                    <thead>{header}</thead>
                    <tbody class="divide-y divide-white/5">
                        {strategy_row("Games", &|detail| {
                            format!("{} games, {} won", detail.games, percent(detail.wins, detail.games)).into_any()
                        })}
                        {strategy_row("Races", &|detail| {
                            detail
                                .races
                                .iter()
                                .map(|race| view! {
                                    <span class="mr-2">
                                        <RaceLink race=race.label.clone() />
                                        {format!(" {}", percent(race.games, detail.games))}
                                    </span>
                                })
                                .collect_view()
                                .into_any()
                        })}
                        {strategy_row("Openings", &|detail| {
                            choice_list(top_choices(&detail.insights.openings_by_matchup), analyzed(detail), false).into_any()
                        })}
                        {strategy_row("First army unit", &|detail| {
                            choice_list(top_choices(&detail.insights.first_army_units), analyzed(detail), true).into_any()
                        })}
                        {strategy_row("Favourite army unit", &|detail| {
                            choice_list(top_choices(&detail.insights.favourite_units_by_map), analyzed(detail), true).into_any()
                        })}
                        {strategy_row("First upgrade", &|detail| {
                            let total: usize = detail.insights.upgrade_priorities.iter().map(|upgrade| upgrade.count).sum();
                            choice_list(
                                detail
                                    .insights
                                    .upgrade_priorities
                                    .iter()
                                    .take(TOP_CHOICES)
                                    .map(|upgrade| Choice { name: upgrade.unit.clone(), games: upgrade.count, wins: 0 })
                                    .collect(),
                                total,
                                false,
                            )
                            .into_any()
                        })}
                        {strategy_row("Proxies", &|detail| {
                            format!("{} of games", percent(detail.insights.proxy_games, analyzed(detail))).into_any()
                        })}
                        {strategy_row("First scout · first kill", &|detail| {
                            format!(
                                "{} · {}",
                                optional_time(detail.insights.avg_first_scout_seconds),
                                optional_time(detail.insights.avg_first_kill_seconds),
                            )
                            .into_any()
                        })}
                        {strategy_row("Commands per minute", &|detail| {
                            format!("{:.0}", detail.insights.avg_commands_per_minute).into_any()
                        })}
                        {strategy_row("Supply blocked · floating", &|detail| {
                            format!(
                                "{} · {}",
                                format_game_time(detail.insights.avg_supply_blocked_seconds as u32),
                                format_game_time(detail.insights.avg_floating_seconds as u32),
                            )
                            .into_any()
                        })}
                    </tbody>
                </table>
            </div>
            <div class="min-w-0">
                <h3 class="text-sm font-semibold text-white">"In the games between them"</h3>
                <table class=FIXED_TABLE_CLASS>
                    <colgroup><col class="w-40" /><col /><col /></colgroup>
                    <thead>{header_h2h}</thead>
                    <tbody class="divide-y divide-white/5">
                        {h2h_row("Openings", false, opening)}
                        {h2h_row("First army unit", true, |side| side.first_army_unit.clone())}
                        {h2h_row("Favourite army unit", true, |side| side.favourite_army_unit.clone())}
                        {h2h_row("First upgrade", false, |side| side.first_upgrade.clone())}
                    </tbody>
                </table>
            </div>
        </div>
        <div class="p-2">
            <h3 class="text-sm font-semibold text-white">"Games between them"</h3>
            <div class="scroll-section max-h-96 overflow-x-auto">
                <HeadToHeadGames games player_name=player.player_name opponent_name=opponent.player_name />
            </div>
        </div>
    }
}

/// The games between the players, newest first, with what each one played.
#[component]
fn HeadToHeadGames(
    games: Vec<HeadToHeadGame>,
    player_name: String,
    opponent_name: String,
) -> impl IntoView {
    let side_cells = |side: HeadToHeadSide| {
        view! {
            <td class="pr-2 py-1 whitespace-nowrap">
                <ResultMarker result=if side.won { "Win" } else { "Loss" }.to_string() />
                <RaceLink race=side.race.clone() />
            </td>
            <td class="pr-2 py-1 min-w-40">{opening(&side).unwrap_or_else(|| "-".to_string())}</td>
            <td class="pr-2 py-1 whitespace-nowrap">
                {side.first_army_unit.map(|unit| view! { <UnitLink unit_name=unit /> })}
            </td>
            <td class="pr-3 py-1 font-mono">
                {side.first_expansion_seconds.map(format_game_time).unwrap_or_else(|| "-".to_string())}
            </td>
        }
    };
    view! {
        <table class=TABLE_CLASS>
            <thead class="sticky top-0 bg-gray-800">
                <tr class="text-left text-gray-300">
                    <th class="pr-2" colspan="4"></th>
                    <th class="pr-2 text-white" colspan="4">{clean_player_name_clan_escapes(&player_name)}</th>
                    <th class="pr-2 text-white" colspan="4">{clean_player_name_clan_escapes(&opponent_name)}</th>
                </tr>
                <tr class="text-left text-gray-300">
                    <th class="pr-2">"Replay"</th>
                    <th class="pr-2">"Date"</th>
                    <th class="pr-2">"Map"</th>
                    <th class="pr-3">"Length"</th>
                    <th class="pr-2">"Race"</th>
                    <th class="pr-2">"Opening"</th>
                    <th class="pr-2">"First army"</th>
                    <th class="pr-3">"Expand"</th>
                    <th class="pr-2">"Race"</th>
                    <th class="pr-2">"Opening"</th>
                    <th class="pr-2">"First army"</th>
                    <th class="pr-3">"Expand"</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-white/5">
                {games
                    .into_iter()
                    .map(|game| view! {
                        <tr>
                            <td class="pr-2 py-1"><ReplayLink ext_fs_id=game.ext_fs_id /></td>
                            <td class="pr-2 py-1 whitespace-nowrap">{game.date}</td>
                            <td class="pr-2 py-1 whitespace-nowrap"><MapLink map_title=game.map_title /></td>
                            <td class="pr-3 py-1 font-mono">{format_game_time(game.duration_seconds)}</td>
                            {side_cells(game.player)}
                            {side_cells(game.opponent)}
                        </tr>
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}
