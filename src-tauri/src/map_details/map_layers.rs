//! What happens where on a map, from the games of the snapshot, drawn by swarmy-bevy as bars on
//! the terrain:
//!
//! - fights: where the units that cost resources died.
//! - proxies: the production structures built before 04:00 away from the start location.
//! - creep: where the Queens and the creep tumors placed creep tumors.
//! - early buildings: the structures of the first 03:00 near the start location, the walls at
//!   the ramps and the buildings around the main.

use crate::data::*;
use crate::patch_overview::unit_stats::unit_variants;
use crate::replay_detail::data::{
    EXPANSION_TOWNHALLS, PROXY_DISTANCE, PROXY_MAX_SECONDS, PROXY_STRUCTURES,
};
use crate::replay_rerun::decisions::{has_cost, latest_balance_units};
use crate::replay_rerun::viewer::GAME_LOOPS_PER_SECOND;
use polars::prelude::*;
use swarmy_common::*;

/// The spots of a layer, the most counted first.
pub const MAX_LAYER_SPOTS: u32 = 40;

/// The early buildings are the ones of the first seconds.
pub const EARLY_BUILDING_SECONDS: u32 = 180;

/// The creep tumors, the one of the Queen and the ones a tumor spreads.
const CREEP_TUMORS: [&str; 2] = ["CreepTumorQueen", "CreepTumor"];

fn seconds_loop(seconds: u32) -> i64 {
    (seconds as f64 * GAME_LOOPS_PER_SECOND) as i64
}

/// The spots of the rows, by their `x` and `y` in map cells.
fn layer_spots(rows: LazyFrame) -> LazyFrame {
    rows.select([
        col("ext_fs_id"),
        (col("x").cast(DataType::UInt32) / lit(MAP_LAYER_CELLS)).alias("x"),
        (col("y").cast(DataType::UInt32) / lit(MAP_LAYER_CELLS)).alias("y"),
    ])
    .group_by([col("x"), col("y")])
    .agg([
        len().cast(DataType::UInt32).alias("count"),
        col("ext_fs_id")
            .n_unique()
            .cast(DataType::UInt32)
            .alias("games"),
    ])
    .sort(
        ["count"],
        SortMultipleOptions::default().with_order_descending(true),
    )
    .limit(MAX_LAYER_SPOTS)
}

fn to_spots(df: &DataFrame) -> Result<Vec<MapLayerSpot>, SwarmyError> {
    Ok(u32_values(df, "x")?
        .into_iter()
        .zip(u32_values(df, "y")?)
        .zip(u32_values(df, "count")?)
        .zip(u32_values(df, "games")?)
        .map(|(((x, y), count), games)| MapLayerSpot {
            x,
            y,
            count: count as usize,
            games: games as usize,
        })
        .collect())
}

/// The structures of every race but the townhalls.
fn structures() -> Result<Vec<String>, SwarmyError> {
    let balance = latest_balance_units()?;
    Ok(balance
        .iter()
        .filter(|(name, unit)| {
            !EXPANSION_TOWNHALLS.contains(&name.as_str())
                && unit
                    .attributes
                    .inner
                    .iter()
                    .any(|attribute| attribute.kind.as_deref() == Some("Structure"))
        })
        .map(|(name, _)| name.clone())
        .collect())
}

pub fn try_query_map_layers(
    replay_path: String,
    map_title: String,
) -> Result<MapLayers, SwarmyError> {
    let ipc_path = build_ipc_path(&sanitize_replay_path(&replay_path)?)?;
    let games = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .filter(col("title").eq(lit(map_title.clone())))
        .select([col("ext_fs_id")])
        .unique(None, UniqueKeepStrategy::Any)
        .collect()?;
    let game_ids = u64_values(&games, "ext_fs_id")?;
    if game_ids.is_empty() {
        return Err(SwarmyError::Other(format!(
            "No games found for map {map_title}"
        )));
    }
    let born = scan_snapshot_ipc(&ipc_path, UNIT_BORN_IPC)?.filter(is_in_replays(&game_ids));
    // The start location of each player: their townhall at the start of the game.
    let starts = born
        .clone()
        .filter(
            col("ext_replay_loop")
                .eq(lit(0))
                .and(is_one_of("unit_type_name", &EXPANSION_TOWNHALLS)),
        )
        .select([
            col("ext_fs_id"),
            col("control_player_id"),
            col("x").alias("start_x"),
            col("y").alias("start_y"),
        ]);
    let distance = ((col("x") - col("start_x")).pow(2) + (col("y") - col("start_y")).pow(2)).sqrt();
    let near_start = |seconds: u32, names: &[String]| {
        born.clone()
            .filter(
                col("ext_replay_loop")
                    .gt(lit(0))
                    .and(col("ext_replay_loop").lt_eq(lit(seconds_loop(seconds))))
                    .and(is_one_of("unit_type_name", names)),
            )
            .join(
                starts.clone(),
                [col("ext_fs_id"), col("control_player_id")],
                [col("ext_fs_id"), col("control_player_id")],
                JoinArgs::new(JoinType::Inner),
            )
            .with_column(distance.clone().alias("distance"))
    };
    let proxy_names: Vec<String> = PROXY_STRUCTURES
        .iter()
        .map(|name| name.to_string())
        .collect();
    let proxies =
        near_start(PROXY_MAX_SECONDS, &proxy_names).filter(col("distance").gt(lit(PROXY_DISTANCE)));
    let early = near_start(EARLY_BUILDING_SECONDS, &structures()?)
        .filter(col("distance").lt_eq(lit(PROXY_DISTANCE)));
    let creep = born
        .clone()
        .filter(is_one_of("unit_type_name", &CREEP_TUMORS));
    let balance = latest_balance_units()?;
    let costed: Vec<String> = balance
        .keys()
        .filter(|name| has_cost(balance, name))
        .flat_map(|name| unit_variants(name))
        .collect();
    let fights = scan_snapshot_ipc(&ipc_path, UNIT_DIED_IPC)?.filter(
        is_in_replays(&game_ids)
            .and(col("killer_player_id").is_not_null())
            .and(is_one_of("unit_died_name", &costed)),
    );
    let frames = collect_all(vec![
        layer_spots(fights),
        layer_spots(proxies),
        layer_spots(creep),
        layer_spots(early),
    ])?;
    let kinds = [
        (
            "fights",
            "Fights",
            "Where the units that cost resources died, the paths and the places the armies meet",
        ),
        (
            "proxies",
            "Proxies",
            "Production structures built before 04:00 more than 50 cells away from the start location",
        ),
        (
            "creep",
            "Creep tumors",
            "Where the Queens and the tumors placed creep tumors",
        ),
        (
            "early_buildings",
            "Early buildings",
            "The structures of the first 03:00 near the start location, the walls at the ramps",
        ),
    ];
    let layers = kinds
        .iter()
        .zip(&frames)
        .map(
            |((kind, label, description), df)| -> Result<MapLayer, SwarmyError> {
                Ok(MapLayer {
                    kind: kind.to_string(),
                    label: label.to_string(),
                    description: description.to_string(),
                    spots: to_spots(df)?,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MapLayers {
        map_title,
        spot_cells: MAP_LAYER_CELLS,
        games: game_ids.len(),
        layers,
    })
}

#[cfg(test)]
mod snapshot_tests {
    /// `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO SWARMY_TEST_MAP="Pylon LE" cargo test map_layers_of -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn map_layers_of_a_map() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let map = std::env::var("SWARMY_TEST_MAP").unwrap();
        let start = std::time::Instant::now();
        let layers = super::try_query_map_layers(replay_path, map).unwrap();
        eprintln!("{:?} {} games", start.elapsed(), layers.games);
        for layer in &layers.layers {
            eprintln!(
                "{} {:?}",
                layer.kind,
                &layer.spots[..layer.spots.len().min(3)]
            );
        }
        if let Ok(out) = std::env::var("SWARMY_TEST_OUT") {
            std::fs::write(out, serde_json::to_string(&layers).unwrap()).unwrap();
        }
        assert!(layers.layers.iter().any(|layer| !layer.spots.is_empty()));
    }
}
