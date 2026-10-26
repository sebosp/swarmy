//! Where the players send their units early in the game, across the games in a map.

use crate::data::*;
use crate::replay_detail::data::EXPANSION_TOWNHALLS;
use crate::replay_rerun::decisions::{
    base_unit_name_expr, latest_balance_units, unit_values_frame, MORPH_SOURCES,
};
use crate::replay_rerun::terrain::Terrain;
use polars::prelude::*;
use swarmy_common::*;

/// The side, in map units, of the cells the commands are counted in.
pub const SCOUT_CELL_SIZE: u32 = 4;

/// The game event coordinates are fixed point.
const GAME_EVENT_POS_RATIO: f32 = 4096.;

/// Rotates the positions of players that start on the top right half of the map by 180 degrees,
/// so that every player starts on the bottom left. Most ladder maps are rotationally symmetric.
/// The positions are in the columns `x` and `y`, the start of their player in `start_x` and
/// `start_y`.
fn oriented(center: (f32, f32)) -> [Expr; 2] {
    let top_right =
        ((col("start_x") - lit(center.0)) + (col("start_y") - lit(center.1))).gt(lit(0.));
    [("x", center.0), ("y", center.1)].map(|(axis, center)| {
        when(top_right.clone())
            .then(lit(2. * center) - col(axis))
            .otherwise(col(axis))
            .alias(axis)
    })
}

/// The start location of each player in the games, where its townhall is at the start.
fn start_locations(ipc_path: &str, game_ids: &[u64]) -> Result<LazyFrame, SwarmyError> {
    Ok(scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
        .filter(
            is_in_replays(game_ids)
                .and(col("ext_replay_loop").eq(lit(0)))
                .and(col("unit_type_name").is_in(
                    lit(Series::new("townhalls".into(), EXPANSION_TOWNHALLS)).implode(false),
                    false,
                )),
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("x").cast(DataType::Float32).alias("start_x"),
            col("y").cast(DataType::Float32).alias("start_y"),
        ])
        .unique(
            Some(cols(["ext_fs_id", "player_name"])),
            UniqueKeepStrategy::Last,
        ))
}

fn contains_lowercase(column: &str, value: &str) -> Expr {
    col(column)
        .str()
        .to_lowercase()
        .str()
        .contains_literal(lit(value.to_lowercase()))
}

pub fn try_query_map_scout(
    replay_path: String,
    cache_path: String,
    query: MapScoutQuery,
) -> Result<MapScout, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    if query.map_title.is_empty() {
        return Err(SwarmyError::Other("Choose a map to scout.".to_string()));
    }
    let mut details = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?
        .filter(contains_lowercase("title", &query.map_title));
    if !query.player_name.is_empty() {
        details = details.filter(contains_lowercase("player_name", &query.player_name));
    }
    let details = details
        .select([
            col("ext_fs_id"),
            col("title"),
            col("cache_handles"),
            col("lobby_slot_map_size_x"),
            col("lobby_slot_map_size_y"),
        ])
        .unique(
            Some(Selector::Matches("ext_fs_id".into())),
            UniqueKeepStrategy::Any,
        )
        .collect()?;
    let game_ids = u64_values(&details, "ext_fs_id")?;
    if game_ids.is_empty() {
        return Err(SwarmyError::Other(format!(
            "No games found for map {} and player {}",
            query.map_title, query.player_name
        )));
    }
    let map_title = str_values(&details, "title")?.remove(0);
    let width = u8_values(&details, "lobby_slot_map_size_x")?[0].unwrap_or_default() as u32;
    let height = u8_values(&details, "lobby_slot_map_size_y")?[0].unwrap_or_default() as u32;
    let cache_handles: Vec<String> = str_values(&details, "cache_handles")?[0]
        .split(',')
        .map(str::to_string)
        .collect();
    let terrain = Terrain::load(&cache_path, &cache_handles)
        .inspect_err(|err| tracing::warn!("No terrain for {map_title}: {err:?}"))
        .ok();
    // The start locations are symmetric around the center of the playable area.
    let center = match &terrain {
        Some(terrain) => {
            let (left, bottom, right, top) = terrain.playable;
            ((left + right) as f32 / 2., (bottom + top) as f32 / 2.)
        }
        None => (width as f32 / 2., height as f32 / 2.),
    };

    let starts = start_locations(&ipc_path, &game_ids)?.cache();
    let points = match query.layer {
        ScoutLayer::Commands => command_points(&ipc_path, &game_ids, &query)?,
        ScoutLayer::Deaths => death_points(&ipc_path, &game_ids, &query)?,
    };
    let cell = |axis: &str| {
        col(axis)
            .cast(DataType::UInt32)
            .floor_div(lit(SCOUT_CELL_SIZE))
            .alias(axis)
    };
    // Without a start location the position can not be oriented.
    let cells = points
        .join(
            starts.clone(),
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .with_columns(oriented(center))
        .filter(
            col("x")
                .gt_eq(lit(0.))
                .and(col("y").gt_eq(lit(0.)))
                .and(col("x").lt(lit(width as f32)))
                .and(col("y").lt(lit(height as f32))),
        )
        .group_by([cell("x"), cell("y")])
        .agg([
            len().cast(DataType::UInt32).alias("count"),
            col("game_seconds")
                .cast(DataType::Float32)
                .mean()
                .alias("avg_seconds"),
        ])
        .sort(["x", "y"], Default::default())
        .collect()?;
    let counts = u32_values(&cells, "count")?;
    let total_commands = counts.iter().map(|count| *count as usize).sum();
    let cells = u32_values(&cells, "x")?
        .into_iter()
        .zip(u32_values(&cells, "y")?)
        .zip(counts)
        .zip(f32_values(&cells, "avg_seconds")?)
        .map(|(((x, y), count), avg_seconds)| ScoutCell {
            x: x * SCOUT_CELL_SIZE,
            y: y * SCOUT_CELL_SIZE,
            count,
            avg_seconds,
        })
        .collect();
    let own_start = average_start(starts, center, &query.player_name)?;
    Ok(MapScout {
        map_title,
        games: game_ids.len(),
        commands: total_commands,
        width,
        height,
        cell_size: SCOUT_CELL_SIZE,
        cells,
        enemy_start: own_start.map(|(x, y)| (2. * center.0 - x, 2. * center.1 - y)),
        own_start,
        terrain: terrain.as_ref().map(Terrain::minimap),
    })
}

/// The targets of the commands issued by the players: `ext_fs_id`, `player_name`, `x`, `y` and
/// `game_seconds`.
fn command_points(
    ipc_path: &str,
    game_ids: &[u64],
    query: &MapScoutQuery,
) -> Result<LazyFrame, SwarmyError> {
    let mut commands = scan_snapshot_ipc(ipc_path, CMD_TARGET_POINT_IPC)?
        .filter(is_in_replays(game_ids).and(game_seconds().lt_eq(lit(query.max_seconds))));
    if !query.player_name.is_empty() {
        commands = commands.filter(contains_lowercase("player_name", &query.player_name));
    }
    if !query.unit_name.is_empty() {
        commands = commands.filter(contains_lowercase("unit_name", &query.unit_name));
    }
    Ok(commands.select([
        col("ext_fs_id"),
        col("player_name"),
        (col("target_point_x").cast(DataType::Float32) / lit(GAME_EVENT_POS_RATIO)).alias("x"),
        (col("target_point_y").cast(DataType::Float32) / lit(GAME_EVENT_POS_RATIO)).alias("y"),
        game_seconds(),
    ]))
}

/// Where the units of the players died, the owner is the player the unit was born for, with the
/// columns of [`command_points`].
fn death_points(
    ipc_path: &str,
    game_ids: &[u64],
    query: &MapScoutQuery,
) -> Result<LazyFrame, SwarmyError> {
    let unit_key = [
        col("ext_fs_id"),
        col("unit_tag_index"),
        col("unit_tag_recycle"),
    ];
    let mut owners = scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
        .filter(is_in_replays(game_ids).and(col("player_name").is_not_null()));
    if !query.player_name.is_empty() {
        owners = owners.filter(contains_lowercase("player_name", &query.player_name));
    }
    let owners = owners
        .select([
            col("ext_fs_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            col("player_name"),
        ])
        .unique(None, UniqueKeepStrategy::First);
    let mut deaths = scan_snapshot_ipc(ipc_path, UNIT_DIED_IPC)?
        .filter(is_in_replays(game_ids).and(game_seconds().lt_eq(lit(query.max_seconds))));
    if !query.unit_name.is_empty() {
        deaths = deaths.filter(contains_lowercase("unit_died_name", &query.unit_name));
    }
    // A Drone morphing into a structure or templars merging die without a killer.
    let morph_sources = Series::new("morph_sources".into(), MORPH_SOURCES);
    // InvisibleTargetDummy, Larva, Broodlings, etc are not losses, they have no value.
    let valued_units = unit_values_frame(latest_balance_units()?)?
        .lazy()
        .select([col("unit")]);
    Ok(deaths
        .filter(
            col("unit_died_name")
                .is_in(lit(morph_sources).implode(false), false)
                .and(col("killer_unit_tag_index").is_null())
                .not(),
        )
        .select([
            col("ext_fs_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            base_unit_name_expr("unit_died_name").alias("unit"),
            col("x").cast(DataType::Float32),
            col("y").cast(DataType::Float32),
            game_seconds(),
        ])
        .join(
            valued_units,
            [col("unit")],
            [col("unit")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            owners,
            unit_key.clone(),
            unit_key,
            JoinArgs::new(JoinType::Inner),
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("x"),
            col("y"),
            col("game_seconds"),
        ]))
}

/// The average start location of the players, after orienting every player to the bottom left.
fn average_start(
    starts: LazyFrame,
    center: (f32, f32),
    player_name: &str,
) -> Result<Option<(f32, f32)>, SwarmyError> {
    let average = starts
        .filter(contains_lowercase("player_name", player_name))
        .with_columns([col("start_x").alias("x"), col("start_y").alias("y")])
        .with_columns(oriented(center))
        .select([col("x").mean(), col("y").mean()])
        .collect()?;
    let x = f32_values(&average, "x")?.first().copied();
    let y = f32_values(&average, "y")?.first().copied();
    Ok(x.zip(y).filter(|(x, y)| x.is_finite() && y.is_finite()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_orients_the_start_to_the_bottom_left() {
        let positions = df!(
            "x" => [150f32, 50.],
            "y" => [140f32, 60.],
            "start_x" => [150f32, 50.],
            "start_y" => [140f32, 60.],
        )
        .unwrap()
        .lazy()
        .with_columns(oriented((100., 100.)))
        .collect()
        .unwrap();
        assert_eq!(f32_values(&positions, "x").unwrap(), vec![50., 50.]);
        assert_eq!(f32_values(&positions, "y").unwrap(), vec![60., 60.]);
    }
}
