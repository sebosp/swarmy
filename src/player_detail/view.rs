//! Races, results over time and protocol versions, units and opponents of a player.

use super::compare::PlayerComparison;
use super::insights::PlayerInsightsView;
use crate::components::charts::GamesWinsChart;
use crate::components::links::{
    MapLink, PatchLink, PlayerLink, RaceLink, ReplayLink, ResultMarker,
};
use crate::components::loading::Loading;
use crate::components::name_select::NameSelect;
use crate::components::paged::{page_of, PagedSort, Pager, SortKey, SortTh};
use crate::components::section::Panel;
use crate::components::units::UnitsTable;
use crate::navigation::use_navigation;
use crate::*;
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

/// Units produced less than this are not considered for the most effective unit.
const MIN_PRODUCED_FOR_EFFECTIVENESS: usize = 10;

/// The rows shown in the units table.
const TOP_UNITS: usize = 15;

const HEADING_CLASS: &str = "text-base/7 font-semibold text-base-content";
const TABLE_CLASS: &str = "min-w-full text-xs text-muted";

fn win_rate(wins: usize, games: usize) -> String {
    if games == 0 {
        return "-".to_string();
    }
    format!("{:.0}%", wins as f64 * 100. / games as f64)
}

#[component]
pub fn PlayerDetailView() -> impl IntoView {
    let navigation = use_navigation();
    let detail = RwSignal::new(None::<PlayerDetail>);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    // The opponent the matches are filtered by, empty for all.
    let opponent = RwSignal::new(String::new());
    // The player compared with, empty for none.
    let compare_with = RwSignal::new(String::new());
    // Asks for the player again, i.e. after its accounts changed.
    let reload = RwSignal::new(0u32);
    Effect::new(move |_| {
        reload.track();
        let player_name = navigation.player_name.get();
        if player_name.is_empty() {
            return;
        }
        loading.set(true);
        error.set(None);
        opponent.set(String::new());
        spawn_local(async move {
            match invoke_json::<PlayerDetail>(
                "query_player_detail",
                &PlayerDetailQuery { player_name },
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
        <div class="p-2 space-y-4">
            <div class="flex flex-wrap gap-4 items-end">
                <NameSelect
                    label="Player"
                    command="list_players"
                    value=navigation.player_name
                    on_select=Callback::new(move |name: String| navigation.open_player(&name))
                />
                <NameSelect
                    label="Compare with"
                    command="list_players"
                    value=compare_with
                    on_select=Callback::new(move |name: String| compare_with.set(name))
                    empty_label="Nobody"
                />
            </div>
            <Show when=move || navigation.player_name.get().is_empty()>
                <p class="text-sm">"Choose a player above or by clicking on its name in any view."</p>
            </Show>
            <Show when=move || loading.get()>
                <Loading message="Analysing every game of the player, the first time takes a few seconds..." />
            </Show>
            <ErrorMessage error />
            {move || detail.get().map(|detail| view! {
                <PlayerHeader detail=detail.clone() />
                <AccountsPanel detail=detail.clone() reload />
                <super::battlenet::BattleNetProfiles accounts=detail.accounts.clone() />
                {
                    let player = detail.clone();
                    move || {
                        let other = compare_with.get();
                        (!other.is_empty() && other != player.player_name).then(|| view! {
                            <PlayerComparison player=player.clone() opponent_name=other />
                        })
                    }
                }
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <h2 class=HEADING_CLASS>"Games and wins per month"</h2>
                        <GamesWinsChart data=detail.by_month.clone() />
                    </div>
                    <div>
                        <h2 class=HEADING_CLASS>"Games and wins per protocol version"</h2>
                        <GamesWinsChart data=detail.by_version.clone() />
                    </div>
                </div>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <h2 class=HEADING_CLASS>"Units"</h2>
                        <UnitsTable
                            units=detail.units.clone()
                            min_produced=MIN_PRODUCED_FOR_EFFECTIVENESS
                            max_rows=TOP_UNITS
                        />
                    </div>
                    <OpponentsTable opponents=detail.opponents.clone() opponent />
                </div>
                <PlayerInsightsView insights=detail.insights.clone() />
                <MatchesTable matches=detail.matches.clone() opponent />
            })}
        </div>
    }
}

#[component]
fn PlayerHeader(detail: PlayerDetail) -> impl IntoView {
    let races = detail
        .races
        .iter()
        .map(|race| {
            format!(
                "{} {} games ({})",
                race.label,
                race.games,
                win_rate(race.wins, race.games)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    view! {
        <div>
            <h1 class="text-xl font-semibold text-base-content">
                {clean_player_name_clan_escapes(&detail.player_name)}
            </h1>
            // The wins, and the ties only when there are any: the losses are the rest.
            <p class="text-sm text-muted">
                <span class="font-medium text-base-content tabular-nums">{detail.games}</span>
                " games, "
                <span class="font-medium text-success tabular-nums">{detail.wins}</span>
                {format!(" wins ({})", win_rate(detail.wins, detail.games))}
                {(detail.ties > 0)
                    .then(|| {
                        view! {
                            ", "
                            <span class="font-medium text-warning tabular-nums">{detail.ties}</span>
                            {if detail.ties == 1 { " tie" } else { " ties" }}
                        }
                    })}
                {format!(" · {races}")}
            </p>
        </div>
    }
}

/// The accounts of the player, the names they played as, and the accounts with a similar name to
/// group with it: a player has accounts in several regions and for tournaments.
#[component]
fn AccountsPanel(detail: PlayerDetail, reload: RwSignal<u32>) -> impl IntoView {
    let player = detail.player_name.clone();
    let status = RwSignal::new(String::new());
    let change = move |account: String, grouped: bool, player: String| {
        #[derive(serde::Serialize)]
        struct Args {
            changes: Vec<PlayerAccountChange>,
        }
        let args = Args {
            changes: vec![PlayerAccountChange {
                player,
                account,
                grouped,
            }],
        };
        spawn_local(async move {
            match invoke_response("set_player_accounts", &args).await {
                Ok(_) => {
                    status.set(String::new());
                    reload.update(|reload| *reload += 1);
                }
                Err(err) => status.set(err),
            }
        });
    };
    let row = move |account: PlayerAccount, action: AnyView| {
        view! {
            <tr>
                <td class="pr-3 tabular-nums text-base-content" title="region-realm-id of the account">
                    {account.account.clone()}
                </td>
                <td class="pr-3">{account.region()}</td>
                <td class="pr-3 text-base-content">{account.names.join(", ")}</td>
                <td class="pr-3 num">{account.games}</td>
                <td>{action}</td>
            </tr>
        }
    };
    let accounts = detail
        .accounts
        .iter()
        .cloned()
        .map(|account| {
            let action = if account.grouped_as == player {
                let key = account.account.clone();
                let player = player.clone();
                view! {
                    <button
                        class="btn btn-ghost btn-xs"
                        title=format!("This account is not {player}: it is shown by its own name again")
                        on:click=move |_| change(key.clone(), false, player.clone())
                    >
                        "Remove"
                    </button>
                }
                .into_any()
            } else {
                view! { <span class="text-faint" title="Its name is the player's">"by name"</span> }.into_any()
            };
            row(account, action)
        })
        .collect_view();
    let similar = detail
        .similar_accounts
        .iter()
        .cloned()
        .map(|account| {
            let key = account.account.clone();
            let player = player.clone();
            let action = view! {
                <button
                    class="btn btn-primary btn-xs"
                    title=format!("This account is {player}: its games count as {player}'s")
                    on:click=move |_| change(key.clone(), true, player.clone())
                >
                    "Add"
                </button>
            }
            .into_any();
            row(account, action)
        })
        .collect_view();
    let has_similar = !detail.similar_accounts.is_empty();
    view! {
        <Panel title="Accounts">
            <p class="text-xs text-muted">
                "The StarCraft II accounts of the player, with the names they played as: an account keeps its games when its name or clan tag changes. Add the other accounts of the same player, i.e. of another region or of a tournament, so that they count as one player; the accounts with the same name already do."
            </p>
            <table class=TABLE_CLASS data-sort-key="player-accounts">
                <thead>
                    <tr class="text-left text-muted">
                        <th class="pr-3">"Account"</th>
                        <th class="pr-3">"Region"</th>
                        <th class="pr-3">"Names"</th>
                        <th class="pr-3 num" aria-sort="descending">"Games"</th>
                        <th data-no-sort></th>
                    </tr>
                </thead>
                <tbody>{accounts}</tbody>
            </table>
            {has_similar
                .then(|| {
                    view! {
                        <h3 class="mt-2 text-sm font-semibold text-base-content">
                            {format!("Accounts with a name like {}", clean_player_name_clan_escapes(&detail.player_name))}
                        </h3>
                        <table class=TABLE_CLASS data-sort-key="player-similar-accounts">
                            <thead>
                                <tr class="text-left text-muted">
                                    <th class="pr-3">"Account"</th>
                                    <th class="pr-3">"Region"</th>
                                    <th class="pr-3">"Names"</th>
                                    <th class="pr-3 num" aria-sort="descending">"Games"</th>
                                    <th data-no-sort></th>
                                </tr>
                            </thead>
                            <tbody>{similar}</tbody>
                        </table>
                    }
                })}
            <span class="text-xs text-error">{move || status.get()}</span>
        </Panel>
    }
}

#[component]
fn OpponentsTable(opponents: Vec<OpponentRecord>, opponent: RwSignal<String>) -> impl IntoView {
    let sort = PagedSort::new("player-opponents", (1, true));
    let count = opponents.len();
    let rows = move || {
        page_of(&opponents, sort, |record, column| match column {
            0 => SortKey::Text(record.opponent.to_lowercase()),
            2 => SortKey::Number(record.wins as f64),
            3 => SortKey::Number(record.wins as f64 / record.games.max(1) as f64),
            _ => SortKey::Number(record.games as f64),
        })
        .0
    };
    view! {
        <div>
            <h2 class=HEADING_CLASS>"Opponents"</h2>
            <p class="text-xs text-faint">"Click on the record to filter the matches below."</p>
            <table class=TABLE_CLASS data-server-sort>
                <thead>
                    <tr class="text-left text-muted">
                        <SortTh label="Opponent" column=0 sort />
                        <SortTh label="Games" column=1 sort descending_first=true class="pr-2 num" />
                        <SortTh label="W - L" column=2 sort descending_first=true />
                        <SortTh label="Win rate" column=3 sort descending_first=true class="" />
                    </tr>
                </thead>
                <tbody>
                    {move || rows()
                        .into_iter()
                        .map(|record| {
                            let name = record.opponent.clone();
                            let selected = {
                                let name = name.clone();
                                move || opponent.get() == name
                            };
                            view! {
                                <tr class:bg-base-200=selected>
                                    <td class="pr-2"><PlayerLink player_name=record.opponent.clone() /></td>
                                    <td class="pr-2 num">{record.games}</td>
                                    <td class="pr-2">
                                        <a
                                            href="#"
                                            class="hover:text-primary"
                                            title="Show the matches against this opponent"
                                            on:click=move |ev: MouseEvent| {
                                                ev.prevent_default();
                                                if opponent.get() == name {
                                                    opponent.set(String::new());
                                                } else {
                                                    opponent.set(name.clone());
                                                }
                                            }
                                        >
                                            <span class="text-success">{record.wins}</span>
                                            " - "
                                            <span class="text-error">{record.games - record.wins}</span>
                                        </a>
                                    </td>
                                    <td>{win_rate(record.wins, record.games)}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
            <Pager rows=Signal::derive(move || count) sort />
        </div>
    }
}

#[component]
fn MatchesTable(matches: Vec<PlayerMatch>, opponent: RwSignal<String>) -> impl IntoView {
    let sort = PagedSort::new("player-matches", (1, true));
    let visible_matches = Memo::new(move |_| {
        let opponent = opponent.get();
        matches
            .iter()
            .filter(|game| opponent.is_empty() || game.opponent == opponent)
            .cloned()
            .collect::<Vec<_>>()
    });
    // Another opponent: back to the first page.
    Effect::new(move |_| {
        opponent.track();
        sort.page.set(0);
    });
    let rows = move || {
        visible_matches.with(|games| {
            page_of(games, sort, |game, column| match column {
                0 => SortKey::Number(game.ext_fs_id as f64),
                2 => SortKey::Number(game.protocol_version as f64),
                3 => SortKey::Text(game.map_title.clone()),
                4 => SortKey::Text(game.race.clone()),
                5 => SortKey::Text(game.opponent.to_lowercase()),
                // The date, then the id for the games of a day.
                _ => SortKey::Text(format!("{} {:012}", game.date, game.ext_fs_id)),
            })
            .0
        })
    };
    view! {
        <div>
            <h2 class=HEADING_CLASS>
                "Matches"
                {move || {
                    let opponent = opponent.get();
                    (!opponent.is_empty())
                        .then(|| format!(" against {}", clean_player_name_clan_escapes(&opponent)))
                }}
            </h2>
            <table class=TABLE_CLASS data-server-sort>
                <thead>
                    <tr class="text-left text-muted">
                        <SortTh label="ID" column=0 sort descending_first=true />
                        <SortTh label="Date" column=1 sort descending_first=true />
                        <SortTh label="Protocol" column=2 sort descending_first=true />
                        <SortTh label="Map" column=3 sort />
                        <SortTh label="Race" column=4 sort />
                        <SortTh label="Opponent" column=5 sort class="" />
                    </tr>
                </thead>
                <tbody>
                    {move || rows()
                        .into_iter()
                        .map(|game| view! {
                            <tr>
                                <td class="pr-2"><ReplayLink ext_fs_id=game.ext_fs_id /></td>
                                <td class="pr-2">{game.date}</td>
                                <td class="pr-2"><PatchLink version=game.protocol_version /></td>
                                <td class="pr-2 text-base-content"><MapLink map_title=game.map_title /></td>
                                <td class="pr-2">
                                    <ResultMarker result=if game.won { "Win" } else { "Loss" }.to_string() />
                                    <RaceLink race=game.race />
                                </td>
                                <td>
                                    <PlayerLink player_name=game.opponent />
                                    " ("
                                    <RaceLink race=game.opponent_race />
                                    ")"
                                </td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
            <Pager rows=Signal::derive(move || visible_matches.with(Vec::len)) sort />
        </div>
    }
}
