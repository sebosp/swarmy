//! Dataframe for map statistics.

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub fn try_query_replay_list(
    replay_path: String,
    query: ReplayListQuery,
) -> Result<Vec<ReplayListEntry>, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;

    tracing::info!(
        "Querying map stats from replay path: {} for map_title: {} and player_name: {}",
        ipc_path,
        query.map_title,
        query.player_name
    );

    let mut details_query = LazyFrame::scan_ipc(
        PlRefPath::new(&format!("{}/{}", ipc_path, DETAILS_IPC)),
        Default::default(),
        Default::default(),
    )?;

    if !query.map_title.is_empty() {
        details_query = details_query.filter(
            col("title")
                .str()
                .to_lowercase()
                .str()
                .contains(lit(query.map_title.to_lowercase()), false),
        );
    }
    if !query.player_name.is_empty() {
        details_query = details_query.filter(
            col("player_name")
                .str()
                .to_lowercase()
                .str()
                .contains(lit(query.player_name.to_lowercase()), false),
        );
    }
    details_query = details_query.unique(
        Some(Selector::Matches("ext_fs_id".into())),
        UniqueKeepStrategy::Any,
    );

    let res = details_query
        .group_by([col("ext_fs_id")])
        .agg([
            col("title").first(),
            col("ext_datetime")
                .first()
                .dt()
                .to_string("%Y-%m-%d")
                .alias("replay_date"),
            col("player_name")
                .str()
                .join(",", true)
                .alias("player_list"),
            col("ext_fs_file_name").first().alias("replay_location"),
            col("ext_fs_file_name").first().alias("sha256_sum"),
            col("player_name")
                .filter(col("player_result") == lit("Win"))
                .str()
                .join(", ", true)
                .alias("winner_list"),
        ])
        .sort(
            ["replay_date"],
            SortMultipleOptions::default()
                .with_order_descending(true)
                .with_nulls_last(true),
        )
        .limit(1000)
        .collect()?;
    println!("{res}");
    let res: Vec<ReplayListEntry> = (0..res.height())
        .map(|idx| extract_replay_list_from_df_row(&res.slice(idx as i64, 1)))
        .collect::<Result<_, _>>()?;
    Ok(res)
}

fn extract_replay_list_from_df_row(row: &DataFrame) -> Result<ReplayListEntry, SwarmyError> {
    let map_title = row
        .column("title")?
        .str()?
        .get(0)
        .unwrap_or("Empty Title")
        .to_string();
    let replay_date = col_ymd_to_naive_date(row, "replay_date")?;
    let player_list = row
        .column("player_list")?
        .str()?
        .get(0)
        .unwrap_or("")
        .split(",")
        .map(|val| val.to_string())
        .collect();
    let sha256_sum = row
        .column("sha256_sum")?
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
    let duration = 0;
    let winner_list = row
        .column("winner_list")?
        .str()?
        .get(0)
        .unwrap_or("")
        .split(",")
        .map(|val| val.to_string())
        .collect();
    let replay_file_name: String =
        match std::path::PathBuf::from(replay_location.clone()).file_name() {
            Some(val) => format!("{}", val.display()),
            None => format!(
                "Unable to locate file_name for replay_location: {}",
                replay_location
            ),
        };
    Ok(ReplayListEntry {
        map_title,
        replay_date,
        replay_location,
        replay_file_name,
        player_list,
        sha256_sum,
        duration,
        winner_list,
    })
}
