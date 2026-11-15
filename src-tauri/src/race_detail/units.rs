#![doc = include_str!("units/README.md")]

use crate::data::*;
use crate::patch_overview::data::race_name;
use crate::patch_overview::unit_stats::unit_variants;
use crate::replay_rerun::decisions::{army_supply, base_unit_name, has_cost, latest_balance_units};
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The table of the army units of the race for the SQL of the README, `race_units`: `unit` (a
/// variant, i.e. `SiegeTankSieged`) and `base` (`SiegeTank`), from the balance data.
pub const RACE_UNITS_IPC: &str = "race_units.arrow";

/// The army units of the race that cost resources, and the names of their variants, i.e. a
/// SiegeTankSieged is a SiegeTank.
pub fn race_units(race: &str) -> Result<DataFrame, SwarmyError> {
    let balance = latest_balance_units()?;
    let (variants, bases): (Vec<String>, Vec<String>) = balance
        .iter()
        .filter(|(name, unit)| {
            race_name(unit) == race
                && has_cost(balance, name)
                && army_supply(balance, name).is_some()
        })
        .flat_map(|(name, _)| {
            unit_variants(name)
                .into_iter()
                .map(|variant| (variant, name.clone()))
                .collect::<Vec<_>>()
        })
        .unzip();
    Ok(DataFrame::new(
        variants.len(),
        vec![
            Column::new("unit".into(), variants),
            Column::new("base".into(), bases),
        ],
    )?)
}

/// The army units made by the players of the race over the games of the snapshot: `unit`,
/// `games` (Int32, the player-games the unit was made in), `wins` (Int32, the ones won), `born`
/// and `kills` (u32), the most games first. `units` is [`race_units`] of the race.
pub fn plan(
    source: &dyn SnapshotSource,
    race: &str,
    units: DataFrame,
) -> Result<LazyFrame, SwarmyError> {
    let race_players = snapshot_players_of(source)?
        .filter(col("race").eq(lit(race)))
        .select([col("ext_fs_id"), col("player_name"), col("player_result")]);
    let made = col("born").gt(lit(0u32));
    Ok(source
        .table(GAME_UNITS_IPC)?
        .join(
            units.lazy().select([col("unit")]),
            [col("unit")],
            [col("unit")],
            JoinArgs::new(JoinType::Semi),
        )
        .join(
            race_players,
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("unit")])
        .agg([
            made.clone().sum().cast(DataType::Int32).alias("games"),
            made.and(col("player_result").eq(lit("Win")))
                .sum()
                .cast(DataType::Int32)
                .alias("wins"),
            col("born").sum(),
            col("kills").sum(),
        ])
        .filter(col("games").gt(lit(0)))
        .sort_by_exprs(
            [col("games"), col("unit")],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        ))
}

pub fn shape(frame: DataFrame) -> Result<Vec<RaceUnitStats>, SwarmyError> {
    Ok(str_values(&frame, "unit")?
        .into_iter()
        .zip(i32_values(&frame, "games")?)
        .zip(i32_values(&frame, "wins")?)
        .zip(
            u32_values(&frame, "born")?
                .into_iter()
                .zip(u32_values(&frame, "kills")?),
        )
        .map(|(((unit, games), wins), (born, kills))| RaceUnitStats {
            unit: base_unit_name(&unit).to_string(),
            games: games as usize,
            born: born as usize,
            kills: kills as usize,
            wins: wins as usize,
        })
        .collect())
}

#[cfg(test)]
mod tests;
