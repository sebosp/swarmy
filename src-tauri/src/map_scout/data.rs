//! Where the players send their units early in the game, across the games in a map.

use crate::data::*;
use crate::replay_detail::data::EXPANSION_TOWNHALLS;
use crate::replay_rerun::decisions::{
    base_unit_name, has_cost, latest_balance_units, MORPH_SOURCES,
};
use crate::replay_rerun::terrain::Terrain;
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap};
use swarmy_common::*;

/// The side, in map units, of the cells the commands are counted in.
pub const SCOUT_CELL_SIZE: u32 = 4;

/// The game event coordinates are fixed point.
const GAME_EVENT_POS_RATIO: f32 = 4096.;

/// Rotates the positions of players that start on the top right half of the map by 180 degrees,
/// so that every player starts on the bottom left. Most ladder maps are rotationally symmetric.
#[derive(Debug, Clone, Copy)]
struct Orientation {
    center: (f32, f32),
    rotate: bool,
}

impl Orientation {
    fn new(center: (f32, f32), start: (f32, f32)) -> Self {
        let rotate = (start.0 - center.0) + (start.1 - center.1) > 0.;
        Self { center, rotate }
    }

    fn apply(&self, (x, y): (f32, f32)) -> (f32, f32) {
        if self.rotate {
            (2. * self.center.0 - x, 2. * self.center.1 - y)
        } else {
            (x, y)
        }
    }
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

    let starts = scan_snapshot_ipc(&ipc_path, UNIT_BORN_IPC)?
        .filter(
            is_in_replays(&game_ids)
                .and(col("ext_replay_loop").eq(lit(0)))
                .and(col("unit_type_name").is_in(
                    lit(Series::new("townhalls".into(), EXPANSION_TOWNHALLS)).implode(false),
                    false,
                )),
        )
        .select([col("ext_fs_id"), col("player_name"), col("x"), col("y")])
        .collect()?;
    let mut orientations: HashMap<(u64, String), Orientation> = HashMap::new();
    for (((ext_fs_id, player), x), y) in u64_values(&starts, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&starts, "player_name")?)
        .zip(f32_values(&starts, "x")?)
        .zip(f32_values(&starts, "y")?)
    {
        orientations.insert((ext_fs_id, player), Orientation::new(center, (x, y)));
    }

    let points = match query.layer {
        ScoutLayer::Commands => command_points(&ipc_path, &game_ids, &query)?,
        ScoutLayer::Deaths => death_points(&ipc_path, &game_ids, &query)?,
    };
    let mut cells: BTreeMap<(u32, u32), (u32, f32)> = BTreeMap::new();
    let mut total_commands = 0;
    for (ext_fs_id, player, (x, y), seconds) in points {
        // Without a start location the command can not be oriented.
        let Some(orientation) = orientations.get(&(ext_fs_id, player)) else {
            continue;
        };
        let (x, y) = orientation.apply((x, y));
        if x < 0. || y < 0. || x >= width as f32 || y >= height as f32 {
            continue;
        }
        let cell = cells
            .entry((x as u32 / SCOUT_CELL_SIZE, y as u32 / SCOUT_CELL_SIZE))
            .or_default();
        cell.0 += 1;
        cell.1 += seconds as f32;
        total_commands += 1;
    }

    let own_start = average_start(&ipc_path, &game_ids, &orientations, &query.player_name)?;
    Ok(MapScout {
        map_title,
        games: game_ids.len(),
        commands: total_commands,
        width,
        height,
        cell_size: SCOUT_CELL_SIZE,
        cells: cells
            .into_iter()
            .map(|((x, y), (count, total_seconds))| ScoutCell {
                x: x * SCOUT_CELL_SIZE,
                y: y * SCOUT_CELL_SIZE,
                count,
                avg_seconds: total_seconds / count as f32,
            })
            .collect(),
        enemy_start: own_start.map(|(x, y)| (2. * center.0 - x, 2. * center.1 - y)),
        own_start,
        terrain: terrain.as_ref().map(Terrain::minimap),
    })
}

/// A position of a player in a game at a game second.
type ScoutPoint = (u64, String, (f32, f32), u32);

/// The targets of the commands issued by the players.
fn command_points(
    ipc_path: &str,
    game_ids: &[u64],
    query: &MapScoutQuery,
) -> Result<Vec<ScoutPoint>, SwarmyError> {
    let mut commands = scan_snapshot_ipc(ipc_path, CMD_TARGET_POINT_IPC)?
        .filter(is_in_replays(game_ids).and(game_seconds().lt_eq(lit(query.max_seconds))));
    if !query.player_name.is_empty() {
        commands = commands.filter(contains_lowercase("player_name", &query.player_name));
    }
    if !query.unit_name.is_empty() {
        commands = commands.filter(contains_lowercase("unit_name", &query.unit_name));
    }
    let commands = commands
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("target_point_x"),
            col("target_point_y"),
            game_seconds(),
        ])
        .collect()?;
    Ok(u64_values(&commands, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&commands, "player_name")?)
        .zip(i64_values(&commands, "target_point_x")?)
        .zip(i64_values(&commands, "target_point_y")?)
        .zip(u32_values(&commands, "game_seconds")?)
        .map(|((((ext_fs_id, player), x), y), seconds)| {
            let position = (
                x as f32 / GAME_EVENT_POS_RATIO,
                y as f32 / GAME_EVENT_POS_RATIO,
            );
            (ext_fs_id, player, position, seconds)
        })
        .collect())
}

/// Where the units of the players died, the owner is the player the unit was born for.
fn death_points(
    ipc_path: &str,
    game_ids: &[u64],
    query: &MapScoutQuery,
) -> Result<Vec<ScoutPoint>, SwarmyError> {
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
    let deaths = deaths
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
            col("unit_died_name"),
            col("x"),
            col("y"),
            game_seconds(),
        ])
        .join(
            owners,
            unit_key.clone(),
            unit_key,
            JoinArgs::new(JoinType::Inner),
        )
        .collect()?;
    // InvisibleTargetDummy, Larva, Broodlings, etc are not losses.
    let balance = latest_balance_units()?;
    let xs = u8_values(&deaths, "x")?;
    let ys = u8_values(&deaths, "y")?;
    Ok(u64_values(&deaths, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&deaths, "player_name")?)
        .zip(str_values(&deaths, "unit_died_name")?)
        .zip(xs.into_iter().zip(ys))
        .zip(u32_values(&deaths, "game_seconds")?)
        .filter(|((((_, _), name), _), _)| has_cost(balance, base_unit_name(name)))
        .map(|((((ext_fs_id, player), _), (x, y)), seconds)| {
            let position = (x.unwrap_or_default() as f32, y.unwrap_or_default() as f32);
            (ext_fs_id, player, position, seconds)
        })
        .collect())
}

/// The average start location after orienting every player to the bottom left.
fn average_start(
    ipc_path: &str,
    game_ids: &[u64],
    orientations: &HashMap<(u64, String), Orientation>,
    player_name: &str,
) -> Result<Option<(f32, f32)>, SwarmyError> {
    let starts = scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
        .filter(
            is_in_replays(game_ids)
                .and(col("ext_replay_loop").eq(lit(0)))
                .and(col("unit_type_name").is_in(
                    lit(Series::new("townhalls".into(), EXPANSION_TOWNHALLS)).implode(false),
                    false,
                ))
                .and(contains_lowercase("player_name", player_name)),
        )
        .select([col("ext_fs_id"), col("player_name"), col("x"), col("y")])
        .collect()?;
    let mut sum = (0., 0.);
    let mut count = 0;
    for (((ext_fs_id, player), x), y) in u64_values(&starts, "ext_fs_id")?
        .into_iter()
        .zip(str_values(&starts, "player_name")?)
        .zip(f32_values(&starts, "x")?)
        .zip(f32_values(&starts, "y")?)
    {
        if let Some(orientation) = orientations.get(&(ext_fs_id, player)) {
            let (x, y) = orientation.apply((x, y));
            sum = (sum.0 + x, sum.1 + y);
            count += 1;
        }
    }
    Ok((count > 0).then(|| (sum.0 / count as f32, sum.1 / count as f32)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_orients_the_start_to_the_bottom_left() {
        let center = (100., 100.);
        let top_right = Orientation::new(center, (150., 140.));
        assert_eq!(top_right.apply((150., 140.)), (50., 60.));
        let bottom_left = Orientation::new(center, (50., 60.));
        assert_eq!(bottom_left.apply((50., 60.)), (50., 60.));
    }
}
