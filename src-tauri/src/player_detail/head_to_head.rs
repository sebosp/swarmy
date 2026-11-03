//! The games between two players and what each one played in them.

use super::data::name_without_clan;
use crate::data::*;
use crate::replay_detail::data::analyze_replay_cached;
use polars::prelude::*;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use swarmy_common::*;

/// The players of the snapshot by games played, identified by their name without the clan tag.
/// The players of the snapshot by their accounts, see `details_with_players`.
pub fn try_list_players(
    replay_path: String,
    groups: std::collections::HashMap<String, String>,
) -> Result<Vec<GamesWins>, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let players = details_with_players(&ipc_path, &groups)?
        .select([col("player"), col("ext_fs_id"), col("player_result")])
        .filter(col("player").neq(lit("")))
        .group_by([col("player")])
        .agg([
            col("ext_fs_id")
                .n_unique()
                .cast(DataType::UInt64)
                .alias("games"),
            col("player_result")
                .eq(lit("Win"))
                .sum()
                .cast(DataType::UInt64)
                .alias("wins"),
        ])
        .sort(
            ["games", "player"],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        )
        .collect()?;
    Ok(str_values(&players, "player")?
        .into_iter()
        .zip(u64_values(&players, "games")?)
        .zip(u64_values(&players, "wins")?)
        .map(|((label, games), wins)| GamesWins {
            label,
            games: games as usize,
            wins: wins as usize,
        })
        .collect())
}

/// The side of the player in the analysis of a game.
fn side(detail: &ReplayDetail, player_name: &str) -> Option<(u8, HeadToHeadSide)> {
    let summary = detail
        .players
        .iter()
        .find(|summary| name_without_clan(&summary.player.name) == player_name)?;
    Some((
        summary.player.team_id,
        HeadToHeadSide {
            race: summary.player.race.clone(),
            won: summary.player.result == "Win",
            opening: summary.opening.clone(),
            first_army_unit: summary.first_army_unit.clone(),
            favourite_army_unit: summary.favourite_army_unit.clone(),
            first_upgrade: summary.first_upgrade.clone(),
            first_expansion_seconds: summary.expansions.first().map(|entry| entry.seconds),
        },
    ))
}

/// The games of two players against each other, by their accounts (`groups`, see
/// `details_with_players`).
pub fn try_query_head_to_head(
    replay_path: String,
    query: HeadToHeadQuery,
    groups: std::collections::HashMap<String, String>,
) -> Result<HeadToHead, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let players = details_with_players(&ipc_path, &groups)?;
    let player_name = resolve_player(&players, name_without_clan(&query.player_name))?;
    let opponent_name = resolve_player(&players, name_without_clan(&query.opponent_name))?;
    let rows = players
        .select([
            col("ext_fs_id"),
            col("player"),
            col("name"),
            col("ext_fs_file_name"),
            col("title"),
            col("ext_datetime").dt().to_string("%Y-%m-%d").alias("date"),
        ])
        .filter(
            col("player")
                .eq(lit(player_name.clone()))
                .or(col("player").eq(lit(opponent_name.clone()))),
        )
        .collect()?;
    // The games with both players and the names they played as, the teams are checked in the
    // analysis.
    type Game = (HashMap<String, String>, String, String, String);
    let mut games: BTreeMap<u64, Game> = BTreeMap::new();
    for (((((ext_fs_id, player), name), file), title), date) in u64_values(&rows, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&rows, "player")?)
        .zip(str_values(&rows, "name")?)
        .zip(str_values(&rows, "ext_fs_file_name")?)
        .zip(str_values(&rows, "title")?)
        .zip(str_values(&rows, "date")?)
    {
        games
            .entry(ext_fs_id)
            .or_insert_with(|| (HashMap::new(), file, title, date))
            .0
            .insert(player, name);
    }
    let mut games: Vec<HeadToHeadGame> = games
        .into_par_iter()
        .filter_map(|(ext_fs_id, (names, file, map_title, date))| {
            let player_as = names.get(&player_name)?.clone();
            let opponent_as = names.get(&opponent_name)?.clone();
            let detail = match analyze_replay_cached(&file) {
                Ok(detail) => detail,
                Err(err) => {
                    tracing::warn!("Unable to analyse {file}: {err:?}");
                    return None;
                }
            };
            let (player_team, player) = side(&detail, &player_as)?;
            let (opponent_team, opponent) = side(&detail, &opponent_as)?;
            // Teammates are not head to head.
            (player_team != opponent_team).then_some(HeadToHeadGame {
                ext_fs_id,
                date,
                map_title,
                duration_seconds: detail.duration_seconds,
                player,
                opponent,
            })
        })
        .collect();
    games.sort_by(|a, b| b.date.cmp(&a.date).then(b.ext_fs_id.cmp(&a.ext_fs_id)));
    Ok(HeadToHead {
        player_name,
        opponent_name,
        games,
    })
}
