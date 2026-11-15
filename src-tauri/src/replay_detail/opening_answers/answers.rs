#![doc = include_str!("answers/README.md")]

use crate::data::*;
use crate::replay_detail::data::{OPENING_STRUCTURES, is_tech_structure};
use crate::replay_rerun::decisions::latest_balance_units;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The answers with fewer games are not shown.
pub const MIN_ANSWER_GAMES: u32 = 5;

/// The openings are joined with this, as in Player Detail.
pub const OPENING_SEPARATOR: &str = " → ";

/// The table name of the [`tech_structures`] frame, for the SQL of the README.
pub const TECH_STRUCTURES_IPC: &str = "tech_structures.arrow";

/// The tech structures of every race (`unit`), see `replay_detail::data::is_tech_structure`: a
/// frame parameter of [`plan`].
pub fn tech_structures() -> Result<DataFrame, SwarmyError> {
    let balance = latest_balance_units()?;
    let mut units: Vec<&str> = balance
        .keys()
        .filter(|name| is_tech_structure(name, balance))
        .map(String::as_str)
        .collect();
    units.sort_unstable();
    Ok(df!("unit" => units)?)
}

/// In the 1v1 games of the snapshot, how often each opening of a race (its first three tech
/// structures) won against each opening of the enemy race: `race`, `enemy_race`,
/// `enemy_opening`, `opening`, `games` (u32, at least [`MIN_ANSWER_GAMES`]) and `wins` (u32); by
/// race, enemy race, enemy opening and opening.
pub fn plan(source: &dyn SnapshotSource, tech: DataFrame) -> Result<LazyFrame, SwarmyError> {
    // The tracker ids of the players, the structures are by control player id.
    let ids = source
        .table(GAME_PLAYERS_IPC)?
        .select([col("ext_fs_id"), col("player_name"), col("player_id")])
        .unique(
            Some(cols(["ext_fs_id", "player_name"])),
            UniqueKeepStrategy::Any,
        );
    let players = source
        .table(DETAILS_IPC)?
        .filter(col("player_observe").eq(lit(0u8)))
        .join(
            ids,
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .select([
            col("ext_fs_id"),
            col("player_id"),
            race_name_expr("player_race").alias("race"),
            col("player_result"),
        ]);
    let one_vs_ones = players
        .clone()
        .group_by([col("ext_fs_id")])
        .agg([len().alias("players")])
        .filter(col("players").eq(lit(2u32)))
        .select([col("ext_fs_id")]);
    // The first time each tech structure finishes for each player, the first three of them in
    // that order are the opening.
    let firsts = source
        .table(UNIT_BORN_IPC)?
        .filter(col("ext_replay_loop").gt(lit(0i64)))
        .join(
            tech.lazy().select([col("unit")]),
            [col("unit_type_name")],
            [col("unit")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([
            col("ext_fs_id"),
            col("control_player_id").alias("player_id"),
            col("unit_type_name"),
        ])
        .agg([col("ext_replay_loop").min().alias("first_loop")])
        .sort_by_exprs(
            [
                col("ext_fs_id"),
                col("player_id"),
                col("first_loop"),
                col("unit_type_name"),
            ],
            Default::default(),
        )
        .with_column(
            (int_range(lit(0), len(), 1, DataType::UInt32) + lit(1u32))
                .over([col("ext_fs_id"), col("player_id")])?
                .alias("position"),
        )
        .filter(col("position").lt_eq(lit(OPENING_STRUCTURES as u32)));
    let openings = firsts
        .group_by([col("ext_fs_id"), col("player_id")])
        .agg([
            col("unit_type_name")
                .str()
                .join(OPENING_SEPARATOR, true)
                .alias("opening"),
            len().alias("structures"),
        ])
        // The games that ended before the third tech structure are not compared.
        .filter(col("structures").eq(lit(OPENING_STRUCTURES as u32)))
        .select([col("ext_fs_id"), col("player_id"), col("opening")]);
    let played = players
        .join(
            one_vs_ones,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            openings,
            [col("ext_fs_id"), col("player_id")],
            [col("ext_fs_id"), col("player_id")],
            JoinArgs::new(JoinType::Inner),
        );
    let enemies = played.clone().select([
        col("ext_fs_id"),
        col("player_id").alias("enemy_id"),
        col("race").alias("enemy_race"),
        col("opening").alias("enemy_opening"),
    ]);
    Ok(played
        .join(
            enemies,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("player_id").neq(col("enemy_id")))
        .group_by([
            col("race"),
            col("enemy_race"),
            col("enemy_opening"),
            col("opening"),
        ])
        .agg([
            len().alias("games"),
            col("player_result")
                .eq(lit("Win"))
                .sum()
                .cast(DataType::UInt32)
                .alias("wins"),
        ])
        .filter(col("games").gt_eq(lit(MIN_ANSWER_GAMES)))
        .sort_by_exprs(
            [
                col("race"),
                col("enemy_race"),
                col("enemy_opening"),
                col("opening"),
            ],
            Default::default(),
        ))
}

/// By race, enemy race and enemy opening: the openings of the race with their games and wins.
pub type AnswersByOpening = std::collections::HashMap<(String, String, String), Vec<OpeningAnswer>>;

/// The answers the best first (`OpeningAnswer::ranking`, then the most played).
pub fn shape(frame: DataFrame) -> Result<AnswersByOpening, SwarmyError> {
    let mut result = AnswersByOpening::new();
    for (((((race, enemy_race), enemy_opening), opening), games), wins) in
        str_values(&frame, "race")?
            .into_iter()
            .zip(str_values(&frame, "enemy_race")?)
            .zip(str_values(&frame, "enemy_opening")?)
            .zip(str_values(&frame, "opening")?)
            .zip(u32_values(&frame, "games")?)
            .zip(u32_values(&frame, "wins")?)
    {
        result
            .entry((race, enemy_race, enemy_opening))
            .or_default()
            .push(OpeningAnswer {
                opening,
                games,
                wins,
            });
    }
    for answers in result.values_mut() {
        answers.sort_by(|a, b| {
            b.ranking()
                .total_cmp(&a.ranking())
                .then(b.games.cmp(&a.games))
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests;
