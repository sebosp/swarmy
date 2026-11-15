#![doc = include_str!("games/README.md")]

use crate::data::*;
use crate::player_detail::player_rows;
use polars::prelude::*;
use std::collections::{HashMap, HashSet};
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The sort columns of the games, the ext_fs_id breaks the ties so that the order is stable.
pub fn sort_columns(sort_by: ReplayListSort) -> Vec<&'static str> {
    let column = match sort_by {
        ReplayListSort::Id => "ext_fs_id",
        ReplayListSort::Map => "title",
        ReplayListSort::Players => "players_sort",
        ReplayListSort::Duration => "game_seconds",
        ReplayListSort::Races => "matchup",
        ReplayListSort::Date => "ext_datetime",
    };
    if column == "ext_fs_id" {
        vec![column]
    } else {
        vec![column, "ext_fs_id"]
    }
}

/// The games that match the query, one row each, sorted as the query says: `ext_fs_id`,
/// `title`, `ext_datetime`, `replay_date`, the lists `player_names`, `player_results` and
/// `player_races` (aligned), `replay_location`, `map_info_sha256`, `replay_sha256`,
/// `players_sort`, `matchup`, `players`, `teams` and `game_seconds`. `replays` are the sha256 of
/// the replays to keep (the notes, tags and messages of the personal database), none to keep all;
/// `groups` the accounts grouped under a player, so that the search of a player finds the games
/// of all their accounts. The view shows the first [`REPLAY_LIST_LIMIT`] rows.
pub fn plan(
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    query: &ReplayListQuery,
    replays: Option<&HashSet<String>>,
) -> Result<LazyFrame, SwarmyError> {
    // The observers watched the game, they are not its players.
    let mut details = source
        .table(DETAILS_IPC)?
        .filter(col("player_observe").eq(lit(0u8)));
    if !query.map_title.is_empty() {
        details = details.filter(
            col("title")
                .str()
                .to_lowercase()
                .str()
                .contains_literal(lit(query.map_title.to_lowercase())),
        );
    }
    if let Some(min_date) = query.min_date {
        details = details.filter(col("ext_datetime").dt().date().gt_eq(lit(min_date)));
    }
    if let Some(max_date) = query.max_date {
        details = details.filter(col("ext_datetime").dt().date().lt_eq(lit(max_date)));
    }
    if let Some(replays) = replays {
        let replays: Vec<String> = replays.iter().cloned().collect();
        details = details.filter(col("ext_fs_sha256").is_in(
            lit(Series::new("replays".into(), replays)).implode(false),
            false,
        ));
    }
    if let Some(protocol_version) = query.protocol_version {
        details = details.filter(col("ext_protocol_version").eq(lit(protocol_version)));
    }
    if !query.player_name.is_empty() {
        // The games of the players or of the names that match, with all their players: a player
        // is its accounts, the search of Clem finds the games his accounts played as LiquidClem.
        let search = query.player_name.to_lowercase();
        let matches = |column: &str| {
            col(column)
                .str()
                .to_lowercase()
                .str()
                .contains_literal(lit(search.clone()))
        };
        let found = player_rows::plan(source, groups)?
            .filter(matches("player").or(matches("name")))
            .select([col("ext_fs_id")])
            .unique(None, UniqueKeepStrategy::Any);
        details = details.join(
            found,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Semi),
        );
    }
    let mut games = details
        .with_column(race_letter_expr("player_race").alias("race_letter"))
        .group_by([col("ext_fs_id")])
        .agg([
            col("title").first(),
            col("ext_datetime").max(),
            col("ext_datetime")
                .first()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("replay_date"),
            // Lists, so that the names, results and races of each player stay aligned.
            col("player_name").alias("player_names"),
            col("player_result").alias("player_results"),
            col("ext_fs_file_name").first().alias("replay_location"),
            col("map_info_sha256").first().alias("map_info_sha256"),
            col("ext_fs_sha256").first().alias("replay_sha256"),
            col("player_race").alias("player_races"),
            col("player_name")
                .str()
                .to_lowercase()
                .sort(Default::default())
                .str()
                .join(",", true)
                .alias("players_sort"),
            // The races sorted, i.e. `PvT`, the same for both players.
            col("race_letter")
                .sort(Default::default())
                .str()
                .join("v", true)
                .alias("matchup"),
            len().alias("players"),
            col("player_team_id").n_unique().alias("teams"),
        ]);
    let matchup = normalize_matchup(&query.matchup);
    if !matchup.is_empty() {
        games = games.filter(col("matchup").eq(lit(matchup)));
    }
    // A matchup is of 1v1 games.
    let team_size = match (query.team_size, query.matchup.is_empty()) {
        (Some(team_size), _) => Some(team_size),
        (None, false) => Some(1),
        (None, true) => None,
    };
    if let Some(team_size) = team_size {
        games = games.filter(
            col("teams")
                .eq(lit(2u32))
                .and(col("players").eq(lit(2 * team_size as u32))),
        );
    }
    let durations = source
        .table(GAME_PLAYERS_IPC)?
        .group_by([col("ext_fs_id")])
        .agg([col("game_seconds").min()]);
    Ok(games
        .join(
            durations,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .sort(
            sort_columns(query.sort_by),
            SortMultipleOptions::default()
                .with_order_descending(query.descending)
                .with_nulls_last(true),
        ))
}

/// The rows as [`ReplayListEntry`]s; the personal fields (rating, tags, best rank) are filled
/// by the command.
pub fn shape(frame: DataFrame) -> Result<Vec<ReplayListEntry>, SwarmyError> {
    let ids = u64_values(&frame, "ext_fs_id")?;
    let titles = str_values(&frame, "title")?;
    let dates = str_values(&frame, "replay_date")?;
    let locations = str_values(&frame, "replay_location")?;
    let map_shas = str_values(&frame, "map_info_sha256")?;
    let replay_shas = str_values(&frame, "replay_sha256")?;
    let seconds: Vec<Option<u32>> = frame.column("game_seconds")?.u32()?.iter().collect();
    let names = list_str_values(&frame, "player_names")?;
    let results = list_str_values(&frame, "player_results")?;
    let races = list_str_values(&frame, "player_races")?;
    (0..frame.height())
        .map(|row| {
            Ok(ReplayListEntry {
                ext_fs_id: ids[row],
                map_title: titles[row].clone(),
                replay_date: chrono::NaiveDate::parse_from_str(&dates[row], "%Y-%m-%d")
                    .map_err(|err| SwarmyError::Other(format!("date {:?}: {err}", dates[row])))?,
                replay_location: locations[row].clone(),
                replay_file_name: std::path::Path::new(&locations[row])
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
                player_list: names[row].clone(),
                sha256_sum: map_shas[row].clone(),
                duration: seconds[row].unwrap_or_default() as i32,
                player_results: results[row].clone(),
                player_races: races[row].clone(),
                replay_sha256: replay_shas[row].clone(),
                ..Default::default()
            })
        })
        .collect()
}

/// The strings of a list column, row by row.
fn list_str_values(frame: &DataFrame, name: &str) -> Result<Vec<Vec<String>>, SwarmyError> {
    let lists = frame.column(name)?.list()?;
    Ok((0..lists.len())
        .map(|row| {
            lists
                .get_as_series(row)
                .and_then(|values| {
                    values.str().ok().map(|values| {
                        values
                            .iter()
                            .map(|value| value.unwrap_or_default().to_string())
                            .collect()
                    })
                })
                .unwrap_or_default()
        })
        .collect())
}

#[cfg(test)]
mod tests;
