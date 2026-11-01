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
    match tracked_ability(&ability) {
        Some(tracked) => view! {
            <span class="text-base-content" title=format!("{} of the {}, {}", tracked.cost.label(), tracked.unit, tracked.ability)>
                {tracked.label}
            </span>
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

/// The abilities a player used in a replay, compared to the games of the race in the patch.
#[component]
pub fn AbilityUses(abilities: Vec<AbilityUse>) -> impl IntoView {
    if abilities.is_empty() {
        return view! {
            <p class="text-xs text-muted">"No ability with energy or a cooldown used."</p>
        }
        .into_any();
    }
    view! {
        <table class=TABLE_CLASS>
            <thead>
                <tr class="text-left text-muted">
                    <th class="pr-2">"Ability"</th>
                    <th class="num" title="The uses in this game, ▲ when 25% or more above the patch average, ▼ when 25% or more below">"Uses"</th>
                    <th class="pr-2"></th>
                    <th class="pr-2 num" title="The uses per game of the players of the race in the games of the patch">"Patch avg"</th>
                    <th class="pr-2 num" title="The rank of this game among the games of the race in the patch, 1 is the most uses">"Game rank"</th>
                    <th class="pr-2 num" title="The share of the games of the patch with as many uses or more: 5% is a game among the 5% with the most uses">"Top"</th>
                    <th class="pr-2 num" title=format!("The rank of the player by uses per game in the patch, with at least {ABILITY_RANK_MIN_GAMES} games of the race")>"Player rank"</th>
                    <th></th>
                </tr>
            </thead>
            <tbody>
                {abilities
                    .into_iter()
                    .map(|ability| {
                        let average = ability.patch.as_ref().map(|patch| patch.per_game());
                        // The color repeats the arrow, it is not the only cue.
                        let (color, arrow) = match average {
                            Some(average) if ability.uses as f64 >= average * 1.25 => ("text-success", "▲"),
                            Some(average) if (ability.uses as f64) < average * 0.75 => ("text-error", "▼"),
                            _ => ("text-base-content", ""),
                        };
                        let is_top = ability.is_top_player();
                        view! {
                            <tr>
                                <td class="pr-2"><AbilityName ability=ability.ability.clone() /></td>
                                <td class=format!("num {color}")>{ability.uses}</td>
                                <td class=format!("pr-2 pl-0.5 {color}")>{arrow}</td>
                                <td class="pr-2 num">
                                    {average.map(|average| format!("{average:.1}")).unwrap_or_else(|| "-".to_string())}
                                </td>
                                <td class="pr-2 num">
                                    {ability.game_rank.map(|rank| rank_of(rank.rank, rank.of)).unwrap_or_else(|| "-".to_string())}
                                </td>
                                <td class="pr-2 num">
                                    {ability.game_rank.map(|rank| top_percent(rank.top_percent())).unwrap_or_else(|| "-".to_string())}
                                </td>
                                <td class="pr-2 num">
                                    {ability.player_rank.map(|rank| rank_of(rank.rank, rank.of)).unwrap_or_else(|| "-".to_string())}
                                </td>
                                <td>
                                    {is_top.then(|| view! {
                                        <span class="badge badge-xs badge-accent whitespace-nowrap" title=format!("Among the {ABILITY_TOP_K} players of the patch with the most uses per game")>
                                            {format!("top {ABILITY_TOP_K} player")}
                                        </span>
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
fn rank_of(rank: usize, of: usize) -> String {
    format!("{} / {}", group_digits(rank as u64), group_digits(of as u64))
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
