//! The abilities with energy or a cooldown, i.e. Abduct or Yamato Cannon, and how often they are
//! used in the games of the snapshot.

use crate::components::links::{PlayerLink, RaceLink, UnitLink};
use crate::*;
use leptos::prelude::*;
use swarmy_common::*;

const TABLE_CLASS: &str = "min-w-full text-xs text-muted";

/// The use of the tracked abilities by patch, it does not change until the snapshot is optimized
/// again.
pub async fn fetch_ability_usage() -> Result<AbilityUsageTable, String> {
    invoke_json::<AbilityUsageTable>("query_ability_usage", &()).await
}

/// The ability name with the cost, i.e. `Abduct` and `Energy` as a title.
#[component]
pub fn AbilityName(ability: String) -> impl IntoView {
    let navigation = crate::navigation::use_navigation();
    match tracked_ability(&ability) {
        Some(tracked) => view! {
            <a
                href="#"
                class="nav-link text-base-content"
                title=format!("{} of the {}, {}: open the Ability view", tracked.cost.label(), tracked.unit, tracked.ability)
                on:click=move |ev: leptos::ev::MouseEvent| {
                    ev.prevent_default();
                    navigation.open_ability(tracked.ability);
                }
            >
                {tracked.label}
            </a>
        }
        .into_any(),
        None => view! { <span>{ability}</span> }.into_any(),
    }
}

/// The use of the abilities in the games of a patch, or of all the patches.
#[component]
pub fn AbilityUsageRows(
    rows: Vec<AbilityPatchUsage>,
    /// Shows the race of each ability, for the tables of all the races.
    #[prop(default = true)]
    show_race: bool,
) -> impl IntoView {
    if rows.is_empty() {
        return view! {
            <p class="text-xs text-muted">"No games of the snapshot used these abilities."</p>
        }
        .into_any();
    }
    view! {
        <table class=TABLE_CLASS>
            <thead>
                <tr class="text-left text-muted">
                    {show_race.then(|| view! { <th class="pr-3">"Race"</th> })}
                    <th class="pr-3">"Ability"</th>
                    <th class="pr-3">"Unit"</th>
                    <th class="pr-3">"Cost"</th>
                    <th class="pr-3 text-right" title="The uses in the games of the players of the race, including the games in which it was not used">"Per game"</th>
                    <th class="pr-3 text-right" title="The games of the players of the race in which it was used at least once">"Used in"</th>
                    <th class="pr-3 text-right" title="The uses in the games in which it was used">"Per game used"</th>
                    <th title="The most uses per game, with at least 3 games of the race">"Top players"</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-base-content/5">
                {rows
                    .into_iter()
                    .map(|row| {
                        let tracked = row.tracked();
                        let race = tracked.map(|tracked| tracked.race).unwrap_or_default();
                        let unit = tracked.map(|tracked| tracked.unit).unwrap_or_default();
                        let cost = tracked.map(|tracked| tracked.cost.label()).unwrap_or_default();
                        view! {
                            <tr>
                                {show_race.then(|| view! { <td class="pr-3 py-1"><RaceLink race=race.to_string() /></td> })}
                                <td class="pr-3 py-1"><AbilityName ability=row.ability.clone() /></td>
                                <td class="pr-3 py-1"><UnitLink unit_name=unit.to_string() /></td>
                                <td class="pr-3 py-1">{cost}</td>
                                <td class="pr-3 py-1 num text-base-content">{format!("{:.2}", row.per_game())}</td>
                                <td class="pr-3 py-1 num">{format!("{:.0}% of {}", row.used_percent(), row.player_games)}</td>
                                <td class="pr-3 py-1 num">{format!("{:.2}", row.per_game_used())}</td>
                                <td class="py-1">
                                    {row
                                        .top_players
                                        .iter()
                                        .map(|player| view! {
                                            <span class="mr-2" title=format!("{} uses in {} games", player.uses, player.games)>
                                                <PlayerLink player_name=player.player_name.clone() />
                                                {format!(" {:.1}", player.per_game())}
                                            </span>
                                        })
                                        .collect_view()}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
    .into_any()
}

/// The badge of a rank among the players or the games of a patch: the best, top 5, top 10 or
/// top 100, in the `--rank-*` colors (styles.css) and in words, `what` is "player" or "game". A
/// tier is only shown when it is a small part of the others, at least twice as many (the best of
/// 10 or more): of 95 games every one is in the top 100.
#[component]
pub fn RankBadge(rank: Rank, what: &'static str, title: String) -> impl IntoView {
    let (class, label, tier) = match rank.rank {
        1 => ("rank-1", format!("best {what}"), 5),
        2..=5 => ("rank-5", format!("top 5 {what}"), 5),
        6..=10 => ("rank-10", format!("top 10 {what}"), 10),
        11..=100 => ("rank-100", format!("top 100 {what}"), 100),
        _ => return None,
    };
    if rank.of < 2 * tier {
        return None;
    }
    Some(view! { <span class=format!("rank-badge {class}") title=title>{label}</span> })
}

/// What a game rank means, i.e. "1 / 5: the most uses of the 5 games...".
pub fn game_rank_title(rank: Rank, race: &str) -> String {
    format!(
        "{}: of the {} games of {race} players on this patch, {} had more uses than this one. {} of the games had as many uses or more.",
        rank_of(rank.rank, rank.of),
        group_digits(rank.of as u64),
        match rank.rank - 1 {
            0 => "none".to_string(),
            more => group_digits(more as u64),
        },
        top_percent(rank.top_percent()),
    )
}

/// What a player rank means.
pub fn player_rank_title(rank: Rank, race: &str) -> String {
    format!(
        "{}: of the {} {race} players with at least {ABILITY_RANK_MIN_GAMES} games on this patch, ranked by uses per game, this player is number {}.",
        rank_of(rank.rank, rank.of),
        group_digits(rank.of as u64),
        rank.rank,
    )
}

/// The abilities a player used in a replay, compared to the games of the race in the patch, the
/// most used first.
#[component]
pub fn AbilityUses(mut abilities: Vec<AbilityUse>, #[prop(into)] race: String) -> impl IntoView {
    if abilities.is_empty() {
        return view! {
            <p class="text-xs text-muted">"No ability with energy or a cooldown used."</p>
        }
        .into_any();
    }
    abilities.sort_by(|a, b| b.uses.cmp(&a.uses).then(a.ability.cmp(&b.ability)));
    view! {
        <table class=TABLE_CLASS data-server-sort>
            <thead>
                <tr class="text-left text-muted">
                    <th class="pr-2">"Ability"</th>
                    <th class="num" title="The uses in this game, the most used first: ▲ 25% or more above the patch average, ▼ 25% or more below, = the same as the average">"Uses"</th>
                    <th class="pr-2"></th>
                    <th class="pr-2 num" title="The uses per game of the players of the race in the games of the patch">"Patch avg"</th>
                    <th class="pr-2 num" title="This game among the games of the race on the patch, by uses: 1 / 5 is the most uses of 5 games. Hover a rank for its meaning">"Game rank"</th>
                    <th class="pr-2 num" title="The share of the games of the patch with as many uses or more: 5% is a game among the 5% with the most uses">"Top"</th>
                    <th class="pr-2 num" title=format!("The player among the players of the race on the patch with at least {ABILITY_RANK_MIN_GAMES} games, by uses per game: 1 / 40 is the player who uses it the most. Hover a rank for its meaning")>"Player rank"</th>
                    <th></th>
                </tr>
            </thead>
            <tbody>
                {abilities
                    .into_iter()
                    .map(|ability| {
                        let average = ability.patch.as_ref().map(|patch| patch.per_game());
                        // The color repeats the sign, it is not the only cue.
                        let (color, sign, sign_title) = match average {
                            Some(average) if ability.uses as f64 >= average * 1.25 => ("text-success", "▲", "25% or more above the patch average"),
                            Some(average) if (ability.uses as f64) < average * 0.75 => ("text-error", "▼", "25% or more below the patch average"),
                            Some(average) if ability.uses as f64 == average.round() => ("text-info", "=", "The same as the patch average"),
                            _ => ("text-base-content", "", ""),
                        };
                        let race = race.clone();
                        view! {
                            <tr>
                                <td class="pr-2"><AbilityName ability=ability.ability.clone() /></td>
                                <td class=format!("num {color}")>{ability.uses}</td>
                                <td class=format!("pr-2 pl-0.5 {color}") title=sign_title>{sign}</td>
                                <td class="pr-2 num">
                                    {average.map(|average| format!("{average:.1}")).unwrap_or_else(|| "-".to_string())}
                                </td>
                                <td class="pr-2 num" title=ability.game_rank.map(|rank| game_rank_title(rank, &race))>
                                    {ability.game_rank.map(|rank| rank_of(rank.rank, rank.of)).unwrap_or_else(|| "-".to_string())}
                                </td>
                                <td class="pr-2 num">
                                    {ability.game_rank.map(|rank| top_percent(rank.top_percent())).unwrap_or_else(|| "-".to_string())}
                                </td>
                                <td class="pr-2 num" title=ability.player_rank.map(|rank| player_rank_title(rank, &race))>
                                    {ability.player_rank.map(|rank| rank_of(rank.rank, rank.of)).unwrap_or_else(|| "-".to_string())}
                                </td>
                                <td>
                                    {ability.player_rank.map(|rank| {
                                        let title = player_rank_title(rank, &race);
                                        view! { <RankBadge rank what="player" title /> }
                                    })}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
    .into_any()
}

/// A rank among others, i.e. `12 / 3,450`.
pub fn rank_of(rank: usize, of: usize) -> String {
    format!(
        "{} / {}",
        group_digits(rank as u64),
        group_digits(of as u64)
    )
}

/// The share of the games ranked as high or higher, with one decimal under 10%, so that 0.4% is
/// not shown as 0%.
fn top_percent(percent: f64) -> String {
    if percent < 10. {
        format!("{percent:.1}%")
    } else {
        format!("{percent:.0}%")
    }
}

/// The digits in groups of three, i.e. `3,450`.
pub fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::new();
    for (idx, digit) in digits.chars().enumerate() {
        if idx > 0 && (digits.len() - idx) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}
