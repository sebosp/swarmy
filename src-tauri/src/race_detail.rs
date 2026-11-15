//! Race Detail: a race in the games of the snapshot, its matchups, the maps that favour it, the
//! units it makes and the abilities it uses. Each statistic is a query file of this directory
//! (AGENTS.md, "A query is a file"); this file runs them and puts the answer together. The
//! abilities come from the ability usage computed over the snapshot (`ability_usage`).

pub mod by_patch;
pub mod maps;
pub mod matchups;
pub mod player_games;
pub mod top_players;
pub mod units;

use crate::ability_usage::ability_usage_cached;
use crate::data::*;
use crate::host::SwarmyHost;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use polars::prelude::*;
use swarmy_common::*;

/// The queries of the view, for the debug links of the panels.
pub const QUERIES: [QueryMeta; 6] = [
    player_games::META,
    matchups::META,
    by_patch::META,
    maps::META,
    top_players::META,
    units::META,
];

/// The games and wins of a group of player rows: `games` (u32) and `wins` (Int32, the type of
/// the SQL `SUM(CASE WHEN ... THEN 1 ELSE 0 END)` of the READMEs).
pub(crate) fn games_wins() -> [Expr; 2] {
    [
        len().alias("games"),
        col("player_result")
            .eq(lit("Win"))
            .sum()
            .cast(DataType::Int32)
            .alias("wins"),
    ]
}

pub(crate) fn games_wins_rows(df: &DataFrame, label: &str) -> Result<Vec<GamesWins>, SwarmyError> {
    let labels: Vec<String> = match df.column(label)?.dtype() {
        DataType::String => str_values(df, label)?,
        _ => df
            .column(label)?
            .cast(&DataType::String)?
            .str()?
            .iter()
            .map(|value| value.unwrap_or_default().to_string())
            .collect(),
    };
    Ok(labels
        .into_iter()
        .zip(u32_values(df, "games")?)
        .zip(i32_values(df, "wins")?)
        .map(|((label, games), wins)| GamesWins {
            label,
            games: games as usize,
            wins: wins as usize,
        })
        .collect())
}

/// The rows of the race in the 1v1 games of the snapshot (two players of a StarCraft II race),
/// with `opponent`, the letter of the other player's race: the `mine` CTE of the READMEs.
pub(crate) fn one_vs_one(
    source: &dyn SnapshotSource,
    race: &str,
) -> Result<LazyFrame, SwarmyError> {
    let players = snapshot_players_of(source)?;
    // The two rows of a game numbered, so that the other row is found even when two players have
    // the same name (two A.I. of the same level).
    let pairs = players
        .filter(len().over([col("ext_fs_id")])?.eq(lit(2u32)))
        .with_column(
            int_range(lit(0), len(), 1, DataType::UInt32)
                .over([col("ext_fs_id")])?
                .alias("side"),
        );
    let others = pairs.clone().select([
        col("ext_fs_id"),
        col("side").alias("other_side"),
        col("race").str().slice(lit(0), lit(1)).alias("opponent"),
    ]);
    Ok(pairs
        .filter(col("race").eq(lit(race)))
        .join(
            others,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("other_side").neq(col("side")))
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("player_result"),
            col("title"),
            col("patch"),
            col("opponent"),
        ]))
}

/// The matchup of a row of [`one_vs_one`], the race first: `ZvT`.
pub(crate) fn matchup_expr(race: &str) -> Expr {
    concat_str([lit(&race[..1]), lit("v"), col("opponent")], "", false).alias("matchup")
}

/// The rows of [`one_vs_one`] against another race.
pub(crate) fn not_mirror(race: &str) -> Expr {
    col("opponent").neq(lit(&race[..1]))
}

pub fn try_query_race_detail(replay_path: &str, race: &str) -> Result<RaceDetail, SwarmyError> {
    if !RACE_NAMES.contains(&race) {
        return Err(SwarmyError::Other(format!("Unknown race {race}")));
    }
    let source = IpcSnapshot::open(replay_path)?;
    let player_games = player_games::shape(player_games::plan(&source, race)?.collect()?)?;
    let matchups = matchups::shape(matchups::plan(&source, race)?.collect()?)?;
    let by_patch = by_patch::shape(by_patch::plan(&source, race)?.collect()?)?;
    let maps = maps::shape(maps::plan(&source, race)?.collect()?)?;
    let top_players = top_players::shape(top_players::plan(&source, race)?.collect()?)?;
    let units = units::shape(units::plan(&source, race, units::race_units(race)?)?.collect()?)?;
    let usage = ability_usage_cached(source.ipc_path())?;
    let abilities = race_abilities(race)
        .filter_map(|tracked| usage.all_patches.get(tracked.ability).cloned())
        .collect();
    Ok(RaceDetail {
        race: race.to_string(),
        player_games,
        matchups,
        by_patch,
        maps,
        units,
        abilities,
        top_players,
    })
}

pub async fn query_race_detail(
    host: &dyn SwarmyHost,
    race: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(try_query_race_detail(&app_config.replay_path, &race))
    })
    .await
}
