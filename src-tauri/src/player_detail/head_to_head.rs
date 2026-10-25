//! The games between two players and what each one played in them.

use super::data::name_without_clan;
use crate::data::*;
use crate::replay_detail::data::analyze_replay_cached;
use polars::prelude::*;
use rayon::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

/// The players of the snapshot by games played, identified by their name without the clan tag.
pub fn try_list_players(replay_path: String) -> Result<Vec<GamesWins>, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let players = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .select([
            col("player_name")
                .str()
                .split(lit("<sp/>"))
                .list()
                .last()
                .alias("player"),
            col("ext_fs_id"),
            col("player_result"),
        ])
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

pub fn try_query_head_to_head(
    replay_path: String,
    query: HeadToHeadQuery,
) -> Result<HeadToHead, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let player_name = name_without_clan(&query.player_name).to_string();
    let opponent_name = name_without_clan(&query.opponent_name).to_string();
    let rows = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .select([
            col("ext_fs_id"),
            col("player_name")
                .str()
                .split(lit("<sp/>"))
                .list()
                .last()
                .alias("player"),
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
    // The games with both players, the teams are checked in the analysis.
    let mut games: BTreeMap<u64, (Vec<String>, String, String, String)> = BTreeMap::new();
    for ((((ext_fs_id, player), file), title), date) in u64_values(&rows, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&rows, "player")?)
        .zip(str_values(&rows, "ext_fs_file_name")?)
        .zip(str_values(&rows, "title")?)
        .zip(str_values(&rows, "date")?)
    {
        games
            .entry(ext_fs_id)
            .or_insert_with(|| (vec![], file, title, date))
            .0
            .push(player);
    }
    let mut games: Vec<HeadToHeadGame> = games
        .into_par_iter()
        .filter(|(_, (players, ..))| {
            players.contains(&player_name) && players.contains(&opponent_name)
        })
        .filter_map(|(ext_fs_id, (_, file, map_title, date))| {
            let detail = match analyze_replay_cached(&file) {
                Ok(detail) => detail,
                Err(err) => {
                    tracing::warn!("Unable to analyse {file}: {err:?}");
                    return None;
                }
            };
            let (player_team, player) = side(&detail, &player_name)?;
            let (opponent_team, opponent) = side(&detail, &opponent_name)?;
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
