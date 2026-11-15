#![doc = include_str!("game_uses/README.md")]

use crate::ability_usage::ALL_PATCHES;
use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The table name of the [`tracked_abilities`] frame, for the SQL of the README.
pub const TRACKED_ABILITIES_IPC: &str = "tracked_abilities.arrow";

/// The tracked abilities and their race (`ability`, `race`), a frame parameter of [`plan`]: to
/// keep the uses of the abilities of the race of the player, a Zerg that took a Ghost with a
/// Neural Parasite does not use EMP.
pub fn tracked_abilities() -> DataFrame {
    let abilities: Vec<&str> = TRACKED_ABILITIES.iter().map(|t| t.ability).collect();
    let races: Vec<&str> = TRACKED_ABILITIES.iter().map(|t| t.race).collect();
    DataFrame::new(
        abilities.len(),
        vec![
            Column::new("ability".into(), abilities),
            Column::new("race".into(), races),
        ],
    )
    .expect("two columns of the same length")
}

/// The players of the games of every patch, and once more under [`ALL_PATCHES`] (0) as the games
/// of all the patches: `ext_fs_id`, `player_name`, `race` and `patch`. The base of the three
/// queries of the ability usage.
pub fn players_of_both(source: &dyn SnapshotSource) -> Result<LazyFrame, SwarmyError> {
    let players = snapshot_players_of(source)?.select([
        col("ext_fs_id"),
        col("player_name"),
        col("race"),
        col("patch"),
    ]);
    Ok(concat(
        [
            players.clone(),
            players.with_column(lit(ALL_PATCHES).cast(DataType::UInt32).alias("patch")),
        ],
        UnionArgs::default(),
    )?)
}

/// The uses of each tracked ability by each player in each game, in the patch of the game and
/// again under [`ALL_PATCHES`]: `patch`, `race`, `ability`, `ext_fs_id`, `player_name` and `uses`
/// (u32); by patch, race, ability, game and player. The abilities of other races are left out.
pub fn plan(source: &dyn SnapshotSource, tracked: DataFrame) -> Result<LazyFrame, SwarmyError> {
    let abilities: Vec<&str> = TRACKED_ABILITIES.iter().map(|t| t.ability).collect();
    // The cmd files have one row per unit selected when the command was given, the aggregations
    // counted the commands once by their sequence number.
    let uses = source
        .table(GAME_ABILITIES_IPC)?
        .filter(is_one_of("ability", &abilities))
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("ability"),
            col("uses"),
        ]);
    Ok(uses
        .join(
            players_of_both(source)?,
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            tracked.lazy(),
            [col("ability"), col("race")],
            [col("ability"), col("race")],
            JoinArgs::new(JoinType::Inner),
        )
        .select([
            col("patch"),
            col("race"),
            col("ability"),
            col("ext_fs_id"),
            col("player_name"),
            col("uses"),
        ])
        .sort_by_exprs(
            [
                col("patch"),
                col("race"),
                col("ability"),
                col("ext_fs_id"),
                col("player_name"),
            ],
            Default::default(),
        ))
}

#[cfg(test)]
mod tests;
