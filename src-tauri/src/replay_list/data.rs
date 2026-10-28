//! Dataframe for map statistics.

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

/// The sort columns of the games, the ext_fs_id breaks the ties so that the order is stable.
fn sort_columns(sort_by: ReplayListSort) -> Vec<&'static str> {
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

/// The replays that match the query, `replays` are the sha256 of the replays to keep, from the
/// notes, tags and messages of the personal database, none to keep all of them.
pub fn try_query_replay_list(
    replay_path: String,
    query: ReplayListQuery,
    replays: Option<std::collections::HashSet<String>>,
) -> Result<ReplayList, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;

    tracing::info!("Querying the replay list from {ipc_path}: {query:?}");

    let mut details_query = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?;

    if !query.map_title.is_empty() {
        details_query = details_query.filter(
            col("title")
                .str()
                .to_lowercase()
                .str()
                .contains_literal(lit(query.map_title.to_lowercase())),
        );
    }
    if let Some(min_date) = query.min_date {
        details_query = details_query.filter(col("ext_datetime").dt().date().gt_eq(lit(min_date)));
    }
    if let Some(max_date) = query.max_date {
        details_query = details_query.filter(col("ext_datetime").dt().date().lt_eq(lit(max_date)));
    }
    if let Some(replays) = replays {
        let replays: Vec<String> = replays.into_iter().collect();
        details_query = details_query.filter(col("ext_fs_sha256").is_in(
            lit(Series::new("replays".into(), replays)).implode(false),
            false,
        ));
    }
    if let Some(protocol_version) = query.protocol_version {
        details_query = details_query.filter(col("ext_protocol_version").eq(lit(protocol_version)));
    }
    if !query.player_name.is_empty() {
        // Keep all the players of the games in which a player matches.
        details_query = details_query.filter(
            col("player_name")
                .str()
                .to_lowercase()
                .str()
                .contains_literal(lit(query.player_name.to_lowercase()))
                .any(true)
                .over([col("ext_fs_id")])?,
        );
    }
    let mut games = details_query
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
    let durations = scan_snapshot_ipc(&ipc_path, STATS_IPC)?
        .group_by([col("ext_fs_id")])
        .agg([game_seconds().max()]);
    let games = games.join(
        durations,
        [col("ext_fs_id")],
        [col("ext_fs_id")],
        JoinArgs::new(JoinType::Left),
    );
    let total = games
        .clone()
        .select([len().alias("games")])
        .collect()?
        .column("games")?
        .cast(&DataType::UInt64)?
        .u64()?
        .get(0)
        .unwrap_or(0) as usize;
    let res = games
        .sort(
            sort_columns(query.sort_by),
            SortMultipleOptions::default()
                .with_order_descending(query.descending)
                .with_nulls_last(true),
        )
        .limit(REPLAY_LIST_LIMIT)
        .collect()?;
    let replays: Vec<ReplayListEntry> = (0..res.height())
        .map(|idx| extract_replay_list_from_df_row(&res.slice(idx as i64, 1)))
        .collect::<Result<_, _>>()?;
    Ok(ReplayList { total, replays })
}

/// The protocol versions of the games of the snapshot, the newest first.
pub fn try_list_protocol_versions(replay_path: String) -> Result<Vec<GamesWins>, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let versions = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .group_by([col("ext_protocol_version")])
        .agg([col("ext_fs_id")
            .n_unique()
            .cast(DataType::UInt64)
            .alias("games")])
        .sort(
            ["ext_protocol_version"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .collect()?;
    Ok(u32_values(&versions, "ext_protocol_version")?
        .into_iter()
        .zip(u64_values(&versions, "games")?)
        .map(|(version, games)| GamesWins {
            label: version.to_string(),
            games: games as usize,
            wins: 0,
        })
        .collect())
}

/// The years of the games of the snapshot, the newest first.
pub fn try_list_years(replay_path: String) -> Result<Vec<i32>, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let years = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .select([col("ext_datetime").dt().year().unique().alias("year")])
        .sort(
            ["year"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .collect()?;
    Ok(years.column("year")?.i32()?.iter().flatten().collect())
}

fn extract_replay_list_from_df_row(row: &DataFrame) -> Result<ReplayListEntry, SwarmyError> {
    let ext_fs_id = row.column("ext_fs_id")?.u64()?.get(0).unwrap_or(0);
    let map_title = row
        .column("title")?
        .str()?
        .get(0)
        .unwrap_or("Empty Title")
        .to_string();
    let replay_date = col_ymd_to_naive_date(row, "replay_date")?;
    let player_list = list_str_values(row, "player_names")?;
    let sha256_sum = row
        .column("map_info_sha256")?
        .str()?
        .get(0)
        .unwrap_or("")
        .to_string();
    let replay_location = row
        .column("replay_location")?
        .str()?
        .get(0)
        .unwrap_or("")
        .to_string();
    let duration = row
        .column("game_seconds")?
        .cast(&DataType::Int32)?
        .i32()?
        .get(0)
        .unwrap_or_default();
    let player_results = list_str_values(row, "player_results")?;

    let replay_file_name: String =
        match std::path::PathBuf::from(replay_location.clone()).file_name() {
            Some(val) => format!("{}", val.display()),
            None => format!(
                "Unable to locate file_name for replay_location: {}",
                replay_location
            ),
        };
    let player_races = list_str_values(row, "player_races")?;
    let replay_sha256 = row
        .column("replay_sha256")?
        .str()?
        .get(0)
        .unwrap_or("")
        .to_string();
    Ok(ReplayListEntry {
        ext_fs_id,
        map_title,
        replay_date,
        replay_location,
        replay_file_name,
        player_list,
        sha256_sum,
        duration,
        player_results,
        player_races,
        replay_sha256,
        ..Default::default()
    })
}

/// Reads the list of strings in the first row of a list column.
fn list_str_values(row: &DataFrame, name: &str) -> Result<Vec<String>, SwarmyError> {
    let Some(values) = row.column(name)?.list()?.get_as_series(0) else {
        return Ok(vec![]);
    };
    Ok(values
        .str()?
        .iter()
        .map(|value| value.unwrap_or_default().to_string())
        .collect())
}
