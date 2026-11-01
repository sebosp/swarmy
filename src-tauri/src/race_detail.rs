//! A race in the games of the snapshot: its matchups, the maps that favour it, the units it makes
//! and the abilities it uses.

use crate::ability_usage::ability_usage_cached;
use crate::data::*;
use crate::patch_overview::data::race_name;
use crate::patch_overview::unit_stats::unit_variants;
use crate::replay_rerun::decisions::{army_supply, base_unit_name, has_cost, latest_balance_units};
use crate::run_query;
use crate::settings::load_stored_app_settings;
use polars::prelude::*;
use std::collections::BTreeMap;
use swarmy_common::*;

/// The players with the most games of the race shown.
const TOP_PLAYERS: usize = 10;

/// The army units of the race that cost resources, and the names of their variants, i.e. a
/// SiegeTankSieged is a SiegeTank.
fn race_units(race: &str) -> Result<Vec<(String, String)>, SwarmyError> {
    let balance = latest_balance_units()?;
    Ok(balance
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
        .collect())
}

/// The units that cost resources, of every race, the kills of the other units are not counted,
/// i.e. a Larva or a Broodling.
fn costed_units() -> Result<Vec<String>, SwarmyError> {
    let balance = latest_balance_units()?;
    Ok(balance
        .keys()
        .filter(|name| has_cost(balance, name))
        .flat_map(|name| unit_variants(name))
        .collect())
}

/// The base name of the variants of the column, i.e. SiegeTankSieged is SiegeTank.
fn base_name_expr(column: &str, variants: &[(String, String)]) -> Expr {
    let (names, bases): (Vec<&str>, Vec<&str>) = variants
        .iter()
        .map(|(variant, base)| (variant.as_str(), base.as_str()))
        .unzip();
    col(column).replace(
        lit(Series::new("variants".into(), names)).implode(false),
        lit(Series::new("bases".into(), bases)).implode(false),
    )
}

fn games_wins() -> [Expr; 2] {
    [
        len().cast(DataType::UInt64).alias("games"),
        col("player_result")
            .eq(lit("Win"))
            .cast(DataType::UInt64)
            .sum()
            .alias("wins"),
    ]
}

fn games_wins_rows(df: &DataFrame, label: &str) -> Result<Vec<GamesWins>, SwarmyError> {
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
        .zip(u64_values(df, "games")?)
        .zip(u64_values(df, "wins")?)
        .map(|((label, games), wins)| GamesWins {
            label,
            games: games as usize,
            wins: wins as usize,
        })
        .collect())
}

pub fn try_query_race_detail(replay_path: String, race: String) -> Result<RaceDetail, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    if !RACE_NAMES.contains(&race.as_str()) {
        return Err(SwarmyError::Other(format!("Unknown race {race}")));
    }
    let letter = &race[..1];
    let game = [col("ext_fs_id")];
    let players = snapshot_players(&ipc_path)?;
    let race_players = players.clone().filter(col("race").eq(lit(race.clone())));
    // The 1v1 games, with the race of the other player.
    let one_vs_one = players
        .filter(len().over(game.clone())?.eq(lit(2)))
        .with_column(
            col("race")
                .reverse()
                .over(game)?
                .str()
                .slice(lit(0), lit(1))
                .alias("opponent"),
        )
        .filter(col("race").eq(lit(race.clone())));
    let not_mirror = col("opponent").neq(lit(letter));
    let matchup = concat_str([lit(letter), lit("v"), col("opponent")], "", false).alias("matchup");

    let player_games = race_players.clone().select([len().alias("player_games")]);
    let matchups = one_vs_one
        .clone()
        .group_by([matchup.clone()])
        .agg(games_wins())
        .sort(["matchup"], Default::default());
    let by_patch = one_vs_one
        .clone()
        .filter(not_mirror.clone())
        .group_by([col("patch")])
        .agg(games_wins())
        .sort(["patch"], Default::default());
    let maps = one_vs_one
        .filter(not_mirror)
        .group_by([col("title"), matchup])
        .agg(games_wins());
    let top_players = race_players
        .clone()
        .group_by([col("player_name")])
        .agg(games_wins())
        .sort(
            ["games", "player_name"],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        )
        .limit(TOP_PLAYERS as IdxSize);

    // The units made by the players of the race, each unit once: burrowing, sieging, etc register
    // the same unit again.
    let variants = race_units(&race)?;
    let variant_names: Vec<&str> = variants.iter().map(|(name, _)| name.as_str()).collect();
    let player_key = [col("ext_fs_id"), col("player_name")];
    let units = scan_snapshot_ipc(&ipc_path, UNIT_BORN_IPC)?
        .filter(is_one_of("unit_type_name", &variant_names).and(col("player_name").is_not_null()))
        .unique(
            Some(cols(["ext_fs_id", "unit_tag_index", "unit_tag_recycle"])),
            UniqueKeepStrategy::Any,
        )
        .select([
            col("ext_fs_id"),
            col("player_name"),
            base_name_expr("unit_type_name", &variants).alias("unit"),
        ])
        .join(
            race_players.clone().select([
                col("ext_fs_id"),
                col("player_name"),
                col("player_result"),
            ]),
            player_key.clone(),
            player_key,
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("unit"), col("ext_fs_id"), col("player_name")])
        .agg([
            len().cast(DataType::UInt64).alias("born"),
            col("player_result").first(),
        ])
        .group_by([col("unit")])
        .agg(
            games_wins()
                .into_iter()
                .chain([col("born").sum()])
                .collect::<Vec<_>>(),
        );
    // The units that cost resources killed by the units of the race, own units killed are rare
    // and counted too.
    let id_key = [col("ext_fs_id"), col("player_id")];
    let kills = scan_snapshot_ipc(&ipc_path, UNIT_DIED_IPC)?
        .filter(
            is_one_of("unit_killer_name", &variant_names)
                .and(is_one_of("unit_died_name", &costed_units()?))
                .and(col("killer_player_id").is_not_null()),
        )
        .select([
            col("ext_fs_id"),
            col("killer_player_id").alias("player_id"),
            base_name_expr("unit_killer_name", &variants).alias("unit"),
        ])
        .join(
            race_players.select([col("ext_fs_id"), col("player_id")]),
            id_key.clone(),
            id_key,
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("unit")])
        .agg([len().cast(DataType::UInt64).alias("kills")]);
    let units = units
        .join(
            kills,
            [col("unit")],
            [col("unit")],
            JoinArgs::new(JoinType::Left),
        )
        .with_column(col("kills").fill_null(lit(0u64)))
        .sort(
            ["games", "unit"],
            SortMultipleOptions::default().with_order_descending_multi([true, false]),
        );

    // One plan, the players and the details are read once for all the results.
    let [player_games, matchups, by_patch, maps, top_players, units]: [DataFrame; 6] =
        collect_all(vec![
            player_games,
            matchups,
            by_patch,
            maps,
            top_players,
            units,
        ])?
        .try_into()
        .map_err(|_| SwarmyError::Other("The race queries returned no results".to_string()))?;

    let mut by_map: BTreeMap<String, Vec<GamesWins>> = BTreeMap::new();
    for (title, matchup) in str_values(&maps, "title")?
        .into_iter()
        .zip(games_wins_rows(&maps, "matchup")?)
    {
        by_map.entry(title).or_default().push(matchup);
    }
    let mut maps: Vec<RaceMapStats> = by_map
        .into_iter()
        .map(|(map_title, mut matchups)| {
            matchups.sort_by(|a, b| a.label.cmp(&b.label));
            RaceMapStats {
                map_title,
                matchups,
            }
        })
        .filter(|map| map.games() >= RACE_MAP_MIN_GAMES)
        .collect();
    maps.sort_by(|a, b| {
        b.win_percent()
            .total_cmp(&a.win_percent())
            .then(b.games().cmp(&a.games()))
    });

    let units = str_values(&units, "unit")?
        .into_iter()
        .zip(u64_values(&units, "games")?)
        .zip(u64_values(&units, "wins")?)
        .zip(
            u64_values(&units, "born")?
                .into_iter()
                .zip(u64_values(&units, "kills")?),
        )
        .map(|(((unit, games), wins), (born, kills))| RaceUnitStats {
            unit: base_unit_name(&unit).to_string(),
            games: games as usize,
            born: born as usize,
            kills: kills as usize,
            wins: wins as usize,
        })
        .collect();

    let usage = ability_usage_cached(&ipc_path)?;
    let abilities = race_abilities(&race)
        .filter_map(|tracked| usage.all_patches.get(tracked.ability).cloned())
        .collect();

    Ok(RaceDetail {
        player_games: player_games
            .column("player_games")?
            .cast(&DataType::UInt64)?
            .u64()?
            .get(0)
            .unwrap_or_default() as usize,
        matchups: games_wins_rows(&matchups, "matchup")?,
        by_patch: games_wins_rows(&by_patch, "patch")?,
        maps,
        units,
        abilities,
        top_players: games_wins_rows(&top_players, "player_name")?,
        race,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn query_race_detail(
    app_handle: tauri::AppHandle,
    race: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .process_result(try_query_race_detail(app_config.replay_path, race))
    })
    .await
}

#[cfg(test)]
mod snapshot_tests {
    /// Runs against a real snapshot: `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test -- --ignored`
    #[test]
    #[ignore]
    fn it_queries_a_race_of_a_snapshot() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let start = std::time::Instant::now();
        let detail = super::try_query_race_detail(replay_path, "Zerg".to_string()).unwrap();
        eprintln!("{:?}", start.elapsed());
        eprintln!(
            "player games {} matchups {:?}",
            detail.player_games, detail.matchups
        );
        eprintln!("patches {:?}", detail.by_patch);
        for map in detail.maps.iter().take(3) {
            eprintln!(
                "{} {:.1}% {:?}",
                map.map_title,
                map.win_percent(),
                map.matchups
            );
        }
        for unit in detail.units.iter().take(8) {
            eprintln!(
                "{:<14} games {:>4} born/game {:>5.1} kills/unit {:>4.2} win {:>4.1}%",
                unit.unit,
                unit.games,
                unit.born_per_game(),
                unit.kills_per_unit(),
                unit.win_percent()
            );
        }
        eprintln!("top {:?}", detail.top_players.first());
        assert!(detail.player_games > 0);
        assert!(detail.units.iter().any(|unit| unit.unit == "Zergling"));
        assert!(!detail.abilities.is_empty());
    }
}
