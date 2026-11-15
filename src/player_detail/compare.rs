//! Two players side by side: their preferred strategies in all their games and what each one
//! played in the games between them, with links to the replays.

use crate::components::links::{MapLink, PlayerLink, RaceLink, ReplayLink, ResultMarker, UnitLink};
use crate::components::loading::Loading;
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::BTreeMap;
use swarmy_common::*;

const TABLE_CLASS: &str = "min-w-full text-xs text-muted";

/// The strategy tables, the columns keep their width and the text wraps inside them.
const FIXED_TABLE_CLASS: &str = "w-full table-fixed text-xs text-muted";

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
                        <span class="text-base-content">
                            {if units {
                                view! { <UnitLink unit_name=choice.name /> }.into_any()
                            } else {
                                choice.name.into_any()
                            }}
                        </span>
                        <span class="text-faint">
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

/// Fetches the second player and the games between both, and shows them side by side. The
/// strategies of both are shown as soon as the second player is known, the games between them when
/// they are analysed: two players who never met still compare.
#[component]
pub fn PlayerComparison(player: PlayerDetail, opponent_name: String) -> impl IntoView {
    let opponent = RwSignal::new(None::<PlayerDetail>);
    let head_to_head = RwSignal::new(None::<HeadToHead>);
    let error = RwSignal::new(None::<String>);
    let player_name = player.player_name.clone();
    spawn_local({
        let opponent_name = opponent_name.clone();
        async move {
            match invoke_json::<PlayerDetail>(
                "query_player_detail",
                &PlayerDetailQuery {
                    player_name: opponent_name,
                },
            )
            .await
            {
                Ok(detail) => opponent.set(Some(detail)),
                Err(err) => error.set(Some(err)),
            }
        }
    });
    spawn_local(async move {
        match invoke_json::<HeadToHead>(
            "query_head_to_head",
            &HeadToHeadQuery {
                player_name,
                opponent_name,
            },
        )
        .await
        {
            Ok(games) => head_to_head.set(Some(games)),
            Err(err) => error.set(Some(err)),
        }
    });
    view! {
        <div class="rounded-md border border-base-content/15 bg-base-200/40">
            <ErrorMessage error />
            {move || match opponent.get() {
                Some(opponent) => {
                    view! { <ComparisonContent player=player.clone() opponent head_to_head /> }
                        .into_any()
                }
                None => view! { <Loading message="Analysing the games of the other player..." /> }.into_any(),
            }}
        </div>
    }
}

#[component]
fn ComparisonContent(
    player: PlayerDetail,
    opponent: PlayerDetail,
    /// The games between them, `None` while they are analysed.
    head_to_head: RwSignal<Option<HeadToHead>>,
) -> impl IntoView {
    let player_name = player.player_name.clone();
    let opponent_name = opponent.player_name.clone();
    let (games_player, games_opponent) = (player_name.clone(), opponent_name.clone());
    // The games between them in the heading, once known.
    let games_summary = move || {
        head_to_head.get().map(|head_to_head| {
            let games = head_to_head.games.len();
            let player_wins = head_to_head
                .games
                .iter()
                .filter(|game| game.player.won)
                .count();
            match games {
                0 => ": no games between them in this snapshot".to_string(),
                1 => format!(": 1 game between them, {player_wins} – {}", 1 - player_wins),
                _ => format!(
                    ": {games} games between them, {player_wins} – {}",
                    games - player_wins
                ),
            }
        })
    };
    // A row of the strategies table, the same strategy of both players side by side.
    let strategy_row = |label: &'static str, detail: &dyn Fn(&PlayerDetail) -> AnyView| {
        view! {
            <tr class="align-top">
                <td class="pr-3 py-1 text-muted">{label}</td>
                <td class="pr-3 py-1 break-words">{detail(&player)}</td>
                <td class="py-1 break-words">{detail(&opponent)}</td>
            </tr>
        }
    };
    let analyzed = |detail: &PlayerDetail| detail.insights.analyzed_games;
    let header = view! {
        <tr class="text-left text-base-content">
            <th class="pr-3 py-1"></th>
            <th class="pr-3 py-1"><PlayerLink player_name=player.player_name.clone() /></th>
            <th class="py-1"><PlayerLink player_name=opponent.player_name.clone() /></th>
        </tr>
    };
    view! {
        <h2 class="px-2 py-1 text-base/7 font-semibold text-base-content border-b border-base-content/15 bg-base-200/80 rounded-t-md">
            {format!(
                "{} vs {}",
                clean_player_name_clan_escapes(&player.player_name),
                clean_player_name_clan_escapes(&opponent.player_name),
            )}
            {games_summary}
        </h2>
        <div class="grid grid-cols-1 2xl:grid-cols-2 gap-4 p-2">
            <div class="min-w-0">
                <h3 class="text-sm font-semibold text-base-content">"Preferred strategies, all their games"</h3>
                <table class=FIXED_TABLE_CLASS>
                    <colgroup><col class="w-40" /><col /><col /></colgroup>
                    <thead>{header}</thead>
                    <tbody class="divide-y divide-base-content/5">
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
                <h3 class="text-sm font-semibold text-base-content">"In the games between them"</h3>
                {move || match head_to_head.get() {
                    None => view! { <Loading message="Analysing the games between them..." /> }.into_any(),
                    Some(head_to_head) if head_to_head.games.is_empty() => {
                        view! {
                            <p class="text-xs text-muted">
                                "They never met in the games of this snapshot: compare their preferred strategies on the left."
                            </p>
                        }
                            .into_any()
                    }
                    Some(head_to_head) => {
                        view! {
                            <HeadToHeadChoices
                                games=head_to_head.games
                                player_name=player_name.clone()
                                opponent_name=opponent_name.clone()
                            />
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
        {move || {
            head_to_head
                .get()
                .filter(|head_to_head| !head_to_head.games.is_empty())
                .map(|head_to_head| {
                    view! {
                        <div class="p-2">
                            <h3 class="text-sm font-semibold text-base-content">"Games between them"</h3>
                            <div class="scroll-section max-h-96 overflow-x-auto">
                                <HeadToHeadGames
                                    games=head_to_head.games
                                    player_name=games_player.clone()
                                    opponent_name=games_opponent.clone()
                                />
                            </div>
                        </div>
                    }
                })
        }}
    }
}

/// What each player chose in the games between them, side by side.
#[component]
fn HeadToHeadChoices(
    games: Vec<HeadToHeadGame>,
    player_name: String,
    opponent_name: String,
) -> impl IntoView {
    let total = games.len();
    let h2h_row = |label: &'static str,
                   units: bool,
                   choice: fn(&HeadToHeadSide) -> Option<String>| {
        view! {
            <tr class="align-top">
                <td class="pr-3 py-1 text-muted">{label}</td>
                <td class="pr-3 py-1 break-words">
                    {choice_list(head_to_head_choices(&games, |game| &game.player, choice), total, units)}
                </td>
                <td class="py-1 break-words">
                    {choice_list(head_to_head_choices(&games, |game| &game.opponent, choice), total, units)}
                </td>
            </tr>
        }
    };
    view! {
        <table class=FIXED_TABLE_CLASS>
            <colgroup><col class="w-40" /><col /><col /></colgroup>
            <thead>
                <tr class="text-left text-base-content">
                    <th class="pr-3 py-1"></th>
                    <th class="pr-3 py-1"><PlayerLink player_name=player_name /></th>
                    <th class="py-1"><PlayerLink player_name=opponent_name /></th>
                </tr>
            </thead>
            <tbody class="divide-y divide-base-content/5">
                {h2h_row("Openings", false, opening)}
                {h2h_row("First army unit", true, |side| side.first_army_unit.clone())}
                {h2h_row("Favourite army unit", true, |side| side.favourite_army_unit.clone())}
                {h2h_row("First upgrade", false, |side| side.first_upgrade.clone())}
            </tbody>
        </table>
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
            <thead class="sticky top-0 bg-base-200">
                <tr class="text-left text-muted">
                    <th class="pr-2" colspan="4"></th>
                    <th class="pr-2 text-base-content" colspan="4">{clean_player_name_clan_escapes(&player_name)}</th>
                    <th class="pr-2 text-base-content" colspan="4">{clean_player_name_clan_escapes(&opponent_name)}</th>
                </tr>
                <tr class="text-left text-muted">
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
            <tbody class="divide-y divide-base-content/5">
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
