//! The Battle.net profiles of the accounts of a player: the best 1v1 finish, the games and the
//! ladders of the season with their MMR, from the StarCraft II community API (Config has the
//! credentials of the Battle.net application).

use crate::components::section::Panel;
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

/// The accounts whose profile is read when the view opens, the most played.
const PROFILES_SHOWN: usize = 3;

/// The profiles of the most played accounts of the player.
#[component]
pub fn BattleNetProfiles(accounts: Vec<PlayerAccount>) -> impl IntoView {
    let accounts: Vec<String> = accounts
        .into_iter()
        .take(PROFILES_SHOWN)
        .map(|account| account.account)
        .collect();
    if accounts.is_empty() {
        return None;
    }
    Some(view! {
        <Panel title="Battle.net">
            <p class="text-xs text-muted">
                "The public profiles of the accounts on Battle.net: the best 1v1 finish, the games and the ladders of the current season."
            </p>
            <div class="mt-1 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                {accounts
                    .into_iter()
                    .map(|account| view! { <BattleNetProfileCard account /> })
                    .collect_view()}
            </div>
        </Panel>
    })
}

#[component]
fn BattleNetProfileCard(account: String) -> impl IntoView {
    let profile = RwSignal::new(None::<Result<BattleNetProfile, String>>);
    {
        let account = account.clone();
        spawn_local(async move {
            #[derive(serde::Serialize)]
            struct Args {
                account: String,
            }
            profile.set(Some(
                invoke_json::<BattleNetProfile>("query_battlenet_profile", &Args { account }).await,
            ));
        });
    }
    let figure = |value: String| view! { <span class="font-medium text-base-content tabular-nums">{value}</span> };
    view! {
        <div class="rounded border border-base-content/15 p-2 text-xs text-muted">
            {move || match profile.get() {
                None => view! { <p>{format!("Reading {account} on Battle.net...")}</p> }.into_any(),
                Some(Err(err)) => view! {
                    <p>
                        <span class="text-base-content tabular-nums">{account.clone()}</span>
                        ": "
                        <span class="text-error">{err}</span>
                    </p>
                }
                .into_any(),
                Some(Ok(profile)) => {
                    let name = if profile.clan_tag.is_empty() {
                        profile.display_name.clone()
                    } else {
                        format!("[{}] {}", profile.clan_tag, profile.display_name)
                    };
                    view! {
                        <div class="flex items-baseline justify-between gap-2">
                            <span class="text-sm font-medium text-base-content">{name}</span>
                            <a
                                href=profile.profile_url.clone()
                                target="_blank"
                                rel="noopener noreferrer"
                                class="nav-link tabular-nums"
                                title="The profile on starcraft2.blizzard.com"
                            >
                                {profile.account.clone()}
                            </a>
                        </div>
                        <p class="mt-1">
                            "Best 1v1: "
                            {if profile.best_1v1_league.is_empty() {
                                view! { <span>"-"</span> }.into_any()
                            } else {
                                view! {
                                    {figure(league_label(&profile.best_1v1_league))}
                                    {format!(", {} times", profile.best_1v1_times)}
                                }
                                .into_any()
                            }}
                        </p>
                        <p>
                            {figure(crate::components::abilities::group_digits(profile.total_career_games as u64))}
                            " games, "
                            {figure(profile.games_this_season.to_string())}
                            " this season"
                        </p>
                        {(!profile.ladders.is_empty()).then(|| view! {
                            <table class="mt-1 min-w-full">
                                <thead>
                                    <tr class="text-left">
                                        <th class="pr-2 font-normal">"Ladder"</th>
                                        <th class="pr-2 font-normal num">"Rank"</th>
                                        <th class="pr-2 font-normal num">"W - L"</th>
                                        <th class="font-normal num" title="The matchmaking rating of the account in that ladder">"MMR"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {profile
                                        .ladders
                                        .iter()
                                        .map(|ladder| view! {
                                            <tr>
                                                <td class="pr-2 text-base-content">{format!("{} {}", ladder.mode, league_label(&ladder.league))}</td>
                                                <td class="pr-2 num">{ladder.rank}</td>
                                                <td class="pr-2 num">
                                                    <span class="text-success">{ladder.wins}</span>
                                                    " - "
                                                    <span class="text-error">{ladder.losses}</span>
                                                </td>
                                                <td class="num text-base-content">
                                                    {ladder.mmr.map(|mmr| mmr.to_string()).unwrap_or_else(|| "-".to_string())}
                                                </td>
                                            </tr>
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        })}
                    }
                    .into_any()
                }
            }}
        </div>
    }
}

/// `GRANDMASTER` as `Grandmaster`.
fn league_label(league: &str) -> String {
    let lower = league.to_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
