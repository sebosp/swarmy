//! The spots of a map where Siege Tanks sieged and killed the most, from the games of the
//! snapshot, drawn as bars by swarmy-bevy.
//!
//! A tank that sieges is registered again as a SiegeTankSieged at its position, and again as a
//! SiegeTank when it unsieges. Each kill of a SiegeTankSieged belongs to the last siege of the
//! same tank before the kill.

use crate::data::*;
use crate::patch_overview::unit_stats::unit_variants;
use crate::replay_rerun::decisions::{has_cost, latest_balance_units};
use polars::prelude::*;
use swarmy_common::*;

const SIEGED: &str = "SiegeTankSieged";

/// The key of a unit in a game.
fn tank_key() -> [&'static str; 3] {
    ["ext_fs_id", "unit_tag_index", "unit_tag_recycle"]
}

/// The units that cost resources, i.e. not a Larva or a Broodling.
fn costed_units() -> Result<Vec<String>, SwarmyError> {
    let balance = latest_balance_units()?;
    Ok(balance
        .keys()
        .filter(|name| has_cost(balance, name))
        .flat_map(|name| unit_variants(name))
        .collect())
}

pub fn try_query_siege_spots(
    replay_path: String,
    map_title: String,
) -> Result<SiegeSpots, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    // The games of the map are few, as a literal list their ids are pushed down to the scans.
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

    let sieges = scan_snapshot_ipc(&ipc_path, UNIT_BORN_IPC)?
        .filter(is_in_replays(&game_ids).and(col("unit_type_name").eq(lit(SIEGED))))
        .select([
            col("ext_fs_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            col("ext_replay_loop").alias("siege_loop"),
            (col("x").cast(DataType::UInt32) / lit(SIEGE_SPOT_CELLS)).alias("x"),
            (col("y").cast(DataType::UInt32) / lit(SIEGE_SPOT_CELLS)).alias("y"),
        ])
        .sort(["siege_loop"], Default::default());

    let kills = scan_snapshot_ipc(&ipc_path, UNIT_DIED_IPC)?
        .filter(
            is_in_replays(&game_ids)
                .and(col("unit_killer_name").eq(lit(SIEGED)))
                .and(is_one_of("unit_died_name", &costed_units()?))
                .and(col("killer_unit_tag_index").is_not_null()),
        )
        .select([
            col("ext_fs_id"),
            col("unit_tag_index").alias("victim_tag_index"),
            col("unit_tag_recycle").alias("victim_tag_recycle"),
            col("killer_player_id"),
            col("killer_unit_tag_index").alias("unit_tag_index"),
            col("killer_unit_tag_recycle").alias("unit_tag_recycle"),
            col("ext_replay_loop").alias("kill_loop"),
        ]);
    // The tanks splash their own units too, the owners of the units killed come from their
    // births, only the ones of the units killed are read.
    let victim_key = [
        col("ext_fs_id"),
        col("victim_tag_index"),
        col("victim_tag_recycle"),
    ];
    let owners = scan_snapshot_ipc(&ipc_path, UNIT_BORN_IPC)?
        .filter(is_in_replays(&game_ids))
        .select([
            col("ext_fs_id"),
            col("unit_tag_index").alias("victim_tag_index"),
            col("unit_tag_recycle").alias("victim_tag_recycle"),
            col("control_player_id").alias("victim_player_id"),
        ])
        .join(
            kills.clone().select(victim_key.clone()),
            victim_key.clone(),
            victim_key.clone(),
            JoinArgs::new(JoinType::Semi),
        )
        .unique(
            Some(cols([
                "ext_fs_id",
                "victim_tag_index",
                "victim_tag_recycle",
            ])),
            UniqueKeepStrategy::First,
        );
    let enemy_kills = kills
        .join(
            owners,
            victim_key.clone(),
            victim_key,
            JoinArgs::new(JoinType::Left),
        )
        .filter(
            col("victim_player_id")
                .is_null()
                .or(col("victim_player_id").neq(col("killer_player_id"))),
        )
        .select([
            col("ext_fs_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            col("kill_loop"),
        ])
        .sort(["kill_loop"], Default::default());
    // The last siege of the tank before each kill.
    let by: Vec<PlSmallStr> = tank_key().iter().map(|name| (*name).into()).collect();
    let kills_by_siege = enemy_kills
        .join_builder()
        .with(sieges.clone())
        .left_on([col("kill_loop")])
        .right_on([col("siege_loop")])
        .how(JoinType::AsOf(Box::new(AsOfOptions {
            strategy: AsofStrategy::Backward,
            left_by: Some(by.clone()),
            right_by: Some(by),
            allow_eq: true,
            ..Default::default()
        })))
        .finish()
        .filter(col("siege_loop").is_not_null())
        .group_by(
            tank_key()
                .iter()
                .map(|name| col(*name))
                .chain([col("siege_loop")])
                .collect::<Vec<_>>(),
        )
        .agg([len().cast(DataType::UInt64).alias("kills")]);
    let siege_key: Vec<Expr> = tank_key()
        .iter()
        .map(|name| col(*name))
        .chain([col("siege_loop")])
        .collect();
    let spots = sieges
        .join(
            kills_by_siege,
            siege_key.clone(),
            siege_key,
            JoinArgs::new(JoinType::Left),
        )
        .with_column(col("kills").fill_null(lit(0u64)))
        .group_by([col("x"), col("y")])
        .agg([
            len().cast(DataType::UInt64).alias("sieges"),
            col("kills").sum(),
            col("ext_fs_id")
                .n_unique()
                .cast(DataType::UInt64)
                .alias("games"),
        ])
        .sort(
            ["kills", "sieges", "x", "y"],
            SortMultipleOptions::default().with_order_descending_multi([true, true, false, false]),
        );
    let totals = spots
        .clone()
        .select([col("sieges").sum(), col("kills").sum()]);
    let siege_games = scan_snapshot_ipc(&ipc_path, UNIT_BORN_IPC)?
        .filter(is_in_replays(&game_ids).and(col("unit_type_name").eq(lit(SIEGED))))
        .select([col("ext_fs_id")
            .n_unique()
            .cast(DataType::UInt64)
            .alias("games")]);
    let [spots, totals, siege_games]: [DataFrame; 3] =
        collect_all(vec![spots, totals, siege_games])?
            .try_into()
            .map_err(|_| SwarmyError::Other("The siege spots returned no results".to_string()))?;

    let first = |df: &DataFrame, name: &str| -> Result<usize, SwarmyError> {
        Ok(df
            .column(name)?
            .cast(&DataType::UInt64)?
            .u64()?
            .get(0)
            .unwrap_or_default() as usize)
    };
    let spot_list = u32_values(&spots, "x")?
        .into_iter()
        .zip(u32_values(&spots, "y")?)
        .zip(u64_values(&spots, "sieges")?)
        .zip(
            u64_values(&spots, "kills")?
                .into_iter()
                .zip(u64_values(&spots, "games")?),
        )
        .map(|(((x, y), sieges), (kills, games))| SiegeSpot {
            x,
            y,
            sieges: sieges as usize,
            kills: kills as usize,
            games: games as usize,
        })
        .collect();
    Ok(SiegeSpots {
        map_title,
        spot_cells: SIEGE_SPOT_CELLS,
        games: first(&siege_games, "games")?,
        sieges: first(&totals, "sieges")?,
        kills: first(&totals, "kills")?,
        spots: spot_list,
    })
}

#[cfg(test)]
mod snapshot_tests {
    /// Runs against a real snapshot: `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test -- --ignored`
    #[test]
    #[ignore]
    fn it_finds_the_siege_spots_of_a_map() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let map_title =
            std::env::var("SWARMY_TEST_MAP").unwrap_or_else(|_| "Taito Citadel LE".to_string());
        let start = std::time::Instant::now();
        let spots = super::try_query_siege_spots(replay_path, map_title).unwrap();
        eprintln!("{:?}", start.elapsed());
        eprintln!(
            "games {} sieges {} kills {} spots {}",
            spots.games,
            spots.sieges,
            spots.kills,
            spots.spots.len()
        );
        for spot in spots.spots.iter().take(8) {
            eprintln!("{spot:?} {:.2} kills per siege", spot.kills_per_siege());
        }
        // SWARMY_TEST_SIEGE_JSON writes the spots for swarmy-bevy --siege-spots.
        if let Ok(path) = std::env::var("SWARMY_TEST_SIEGE_JSON") {
            std::fs::write(path, serde_json::to_string(&spots).unwrap()).unwrap();
        }
        assert!(spots.sieges > 0);
        assert_eq!(
            spots.kills,
            spots.spots.iter().map(|spot| spot.kills).sum::<usize>()
        );
    }
}
