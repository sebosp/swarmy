//! Statistics of a player across the snapshot.

use crate::data::*;
use crate::replay_detail::data::unit_effectiveness;
use crate::replay_rerun::decisions::{
    base_unit_name_expr, latest_balance_units, unit_values_frame,
};
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use swarmy_common::*;

/// Reads the protocol version (base build) from the replay header, 0 if it can not be read.
pub fn read_protocol_version(replay_location: &str) -> u32 {
    let read = || -> Option<u32> {
        let contents = s2protocol::read_file(&PathBuf::from(replay_location)).ok()?;
        let (_, mpq) = s2protocol::parser::parse(&contents).ok()?;
        let (_, header) = s2protocol::read_protocol_header(&mpq).ok()?;
        Some(header.m_version.m_base_build)
    };
    read().unwrap_or_default()
}

/// The players change their clan tags, they are identified by the name without it.
pub fn name_without_clan(player_name: &str) -> &str {
    player_name.rsplit("<sp/>").next().unwrap_or(player_name)
}

/// The `player_name` column without the clan tag.
fn player_name_without_clan() -> Expr {
    col("player_name").str().split(lit("<sp/>")).list().last()
}

pub fn add_game(counts: &mut BTreeMap<String, GamesWins>, label: String, won: bool) {
    let entry = counts.entry(label.clone()).or_insert_with(|| GamesWins {
        label,
        ..Default::default()
    });
    entry.games += 1;
    entry.wins += won as usize;
}

/// The accounts with a name like the player's listed to group with it.
const SIMILAR_ACCOUNTS: u32 = 20;

/// The games of a player: of its accounts (`groups`, by account, from the personal database), or
/// of the account of the name; `player_name` is a player or a name one of its accounts played as.
pub fn try_query_player_detail(
    replay_path: String,
    player_name: String,
    groups: HashMap<String, String>,
) -> Result<PlayerDetail, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    let mut details = details_with_players(&ipc_path, &groups)?;
    let player_name = resolve_player(&details, name_without_clan(&player_name))?;
    // Snapshots written before s2protocol-rs stored the protocol version in details.arrow.
    let has_protocol_version = details.collect_schema()?.contains("ext_protocol_version");
    let protocol_version = if has_protocol_version {
        col("ext_protocol_version")
    } else {
        lit(0u32).alias("ext_protocol_version")
    };
    let player_rows = details
        .clone()
        .filter(col("player").eq(lit(player_name.clone())));
    let games = player_rows.clone().select([
        protocol_version,
        col("name").alias("played_as"),
        col("ext_fs_id"),
        col("player_race"),
        col("player_result"),
        col("player_team_id"),
        col("title"),
        col("ext_fs_file_name"),
        col("ext_datetime").dt().to_string("%Y-%m-%d").alias("date"),
    ]);
    let opponents = details
        .clone()
        .filter(col("player").neq(lit(player_name.clone())))
        .select([
            col("ext_fs_id"),
            col("player").alias("opponent"),
            col("player_race").alias("opponent_race"),
            col("player_team_id").alias("opponent_team_id"),
        ]);
    let matches_df = games
        .join(
            opponents,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        // Teammates are not opponents.
        .filter(
            col("opponent_team_id")
                .is_null()
                .or(col("opponent_team_id").neq(col("player_team_id"))),
        )
        .sort(
            ["date"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .collect()?;

    let ext_fs_ids = u64_values(&matches_df, "ext_fs_id")?;
    let files = str_values(&matches_df, "ext_fs_file_name")?;
    let snapshot_versions = u32_values(&matches_df, "ext_protocol_version")?;
    let mut versions: HashMap<u64, u32> = HashMap::new();
    for ((ext_fs_id, file), version) in ext_fs_ids.iter().zip(&files).zip(snapshot_versions) {
        versions.entry(*ext_fs_id).or_insert_with(|| match version {
            0 => read_protocol_version(file),
            version => version,
        });
    }
    let files_by_id: HashMap<u64, String> = ext_fs_ids.iter().copied().zip(files).collect();
    let matches: Vec<PlayerMatch> = ext_fs_ids
        .iter()
        .zip(str_values(&matches_df, "date")?)
        .zip(str_values(&matches_df, "title")?)
        .zip(str_values(&matches_df, "player_race")?)
        .zip(str_values(&matches_df, "opponent")?)
        .zip(str_values(&matches_df, "opponent_race")?)
        .zip(str_values(&matches_df, "player_result")?)
        .zip(str_values(&matches_df, "played_as")?)
        .map(
            |(
                ((((((ext_fs_id, date), map_title), race), opponent), opponent_race), result),
                played_as,
            )| {
                PlayerMatch {
                    played_as,
                    ext_fs_id: *ext_fs_id,
                    date,
                    map_title,
                    race: normalize_race(&race),
                    opponent,
                    opponent_race: normalize_race(&opponent_race),
                    won: result == "Win",
                    tie: result == "Tie",
                    protocol_version: versions[ext_fs_id],
                }
            },
        )
        .collect();

    // A game against several opponents is a single game.
    let mut seen_games = HashSet::new();
    let mut races = BTreeMap::new();
    let mut by_month = BTreeMap::new();
    let mut by_version = BTreeMap::new();
    let mut wins = 0;
    let mut ties = 0;
    for game in matches
        .iter()
        .filter(|game| seen_games.insert(game.ext_fs_id))
    {
        wins += game.won as usize;
        ties += game.tie as usize;
        add_game(&mut races, game.race.clone(), game.won);
        add_game(&mut by_month, game.date.chars().take(7).collect(), game.won);
        add_game(&mut by_version, game.protocol_version.to_string(), game.won);
    }
    let mut races: Vec<GamesWins> = races.into_values().collect();
    races.sort_by_key(|race| std::cmp::Reverse(race.games));
    let mut by_version: Vec<GamesWins> = by_version.into_values().collect();
    by_version.sort_by_key(|version| version.label.parse::<u32>().unwrap_or_default());

    let mut opponents: BTreeMap<String, OpponentRecord> = BTreeMap::new();
    for game in matches.iter().filter(|game| !game.opponent.is_empty()) {
        let record = opponents
            .entry(game.opponent.clone())
            .or_insert_with(|| OpponentRecord {
                opponent: game.opponent.clone(),
                ..Default::default()
            });
        record.games += 1;
        record.wins += game.won as usize;
    }
    let mut opponents: Vec<OpponentRecord> = opponents.into_values().collect();
    opponents.sort_by(|a, b| b.games.cmp(&a.games).then(a.opponent.cmp(&b.opponent)));

    let game_ids: Vec<u64> = seen_games.into_iter().collect();
    // The names the accounts of the player played as.
    let names: Vec<String> = matches
        .iter()
        .map(|game| game.played_as.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let units = query_unit_effectiveness(&ipc_path, &names, &game_ids)?;
    let accounts = accounts_of(player_rows, &groups, 100)?;
    let lowercase = player_name.to_lowercase();
    let similar_accounts = accounts_of(
        details.filter(
            col("player").neq(lit(player_name.clone())).and(
                col("name")
                    .str()
                    .to_lowercase()
                    .str()
                    .contains_literal(lit(lowercase)),
            ),
        ),
        &groups,
        SIMILAR_ACCOUNTS,
    )?;
    let insights =
        super::insights::player_insights(&ipc_path, &player_name, &matches, &files_by_id)?;
    Ok(PlayerDetail {
        player_name,
        games: game_ids.len(),
        wins,
        ties,
        races,
        by_month: by_month.into_values().collect(),
        by_version,
        units,
        opponents,
        matches,
        insights,
        accounts,
        similar_accounts,
    })
}

/// Counts the units produced by the player and the enemy units they killed.
fn query_unit_effectiveness(
    ipc_path: &str,
    names: &[String],
    game_ids: &[u64],
) -> Result<Vec<UnitEffectiveness>, SwarmyError> {
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let born = scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
        .filter(
            is_in_replays(game_ids).and(player_name_without_clan().is_in(
                lit(Series::new("names".into(), names)).implode(false),
                false,
            )),
        )
        .select([
            col("ext_fs_id"),
            col("control_player_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            base_unit_name_expr("unit_type_name").alias("unit"),
        ])
        .cache();
    // Morphs, burrowing, etc register the same unit again, each unit is counted once.
    let produced = born
        .clone()
        .unique(
            Some(cols([
                "ext_fs_id",
                "unit_tag_index",
                "unit_tag_recycle",
                "unit",
            ])),
            UniqueKeepStrategy::Any,
        )
        .group_by([col("unit")])
        .agg([len().cast(DataType::UInt64).alias("produced")])
        .collect()?;
    // The tracker player id of the player in each game, kills are attributed by it.
    let player_ids = born
        .select([
            col("ext_fs_id"),
            col("control_player_id").alias("killer_player_id"),
        ])
        .drop_nulls(None)
        .unique(None, UniqueKeepStrategy::Any);
    let balance = latest_balance_units()?;
    // Larva, eggs, broodlings, etc are not worth counting, they have no value.
    let kills = scan_snapshot_ipc(ipc_path, UNIT_DIED_IPC)?
        .filter(is_in_replays(game_ids).and(col("killer_player_id").is_not_null()))
        .select([
            col("ext_fs_id"),
            col("killer_player_id"),
            base_unit_name_expr("unit_killer_name").alias("unit"),
            base_unit_name_expr("unit_died_name").alias("died"),
        ])
        .join(
            player_ids,
            [col("ext_fs_id"), col("killer_player_id")],
            [col("ext_fs_id"), col("killer_player_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            unit_values_frame(balance)?.lazy(),
            [col("died")],
            [col("unit")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("unit")])
        .agg([
            len().cast(DataType::UInt64).alias("kills"),
            col("value")
                .sum()
                .cast(DataType::UInt64)
                .alias("value_killed"),
        ])
        .collect()?;
    let produced: BTreeMap<String, usize> = str_values(&produced, "unit")?
        .into_iter()
        .zip(u64_values(&produced, "produced")?)
        .map(|(unit, count)| (unit, count as usize))
        .collect();
    let kills: BTreeMap<String, (usize, u32)> = str_values(&kills, "unit")?
        .into_iter()
        .zip(u64_values(&kills, "kills")?)
        .zip(u64_values(&kills, "value_killed")?)
        .map(|((unit, kills), value)| (unit, (kills as usize, value as u32)))
        .collect();
    Ok(unit_effectiveness(produced, &kills, balance))
}

#[cfg(test)]
mod player_snapshot_tests {
    /// Runs against a real snapshot: `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test -- --ignored`
    #[test]
    #[ignore]
    fn it_groups_the_accounts_of_a_player() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let alone = super::try_query_player_detail(
            replay_path.clone(),
            "Clem".to_string(),
            Default::default(),
        )
        .unwrap();
        eprintln!(
            "alone: {} {} games, accounts {:?}",
            alone.player_name,
            alone.games,
            alone
                .accounts
                .iter()
                .map(|a| (&a.account, &a.names, a.games))
                .collect::<Vec<_>>()
        );
        let first = alone.accounts.first().unwrap().account.clone();
        let other = alone.similar_accounts.first().unwrap().account.clone();
        eprintln!(
            "similar: {:?}",
            alone
                .similar_accounts
                .iter()
                .take(5)
                .map(|a| (&a.account, &a.names, a.games))
                .collect::<Vec<_>>()
        );
        let groups = std::collections::HashMap::from([
            (first.clone(), "Clem".to_string()),
            (other.clone(), "Clem".to_string()),
        ]);
        let start = std::time::Instant::now();
        let grouped =
            super::try_query_player_detail(replay_path, "Clem".to_string(), groups).unwrap();
        eprintln!(
            "{:?} grouped: {} {} games",
            start.elapsed(),
            grouped.player_name,
            grouped.games
        );
        assert_eq!(grouped.player_name, "Clem");
        assert!(grouped
            .accounts
            .iter()
            .any(|account| account.account == other));
        assert!(
            grouped
                .accounts
                .iter()
                .filter(|account| account.grouped_as == "Clem")
                .count()
                >= 2
        );
        assert!(grouped.games > alone.games);
        assert!(grouped
            .matches
            .iter()
            .all(|game| !game.played_as.is_empty()));
    }

    #[test]
    #[ignore]
    fn it_finds_the_games_of_the_accounts_of_a_player() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        // The main account of LiquidClem grouped under Clem.
        let groups =
            std::collections::HashMap::from([("2-1-3141896".to_string(), "Clem".to_string())]);
        let query = swarmy_common::ReplayListQuery {
            player_name: "Clem".to_string(),
            ..Default::default()
        };
        let by_name = crate::replay_list::data::try_query_replay_list(
            replay_path.clone(),
            query.clone(),
            None,
            &Default::default(),
        )
        .unwrap();
        let by_player = crate::replay_list::data::try_query_replay_list(
            replay_path.clone(),
            query,
            None,
            &groups,
        )
        .unwrap();
        eprintln!(
            "replay list Clem: {} by name, {} by player",
            by_name.total, by_player.total
        );
        assert!(by_player.total >= by_name.total);
        let head_to_head = |groups: std::collections::HashMap<String, String>| {
            crate::player_detail::head_to_head::try_query_head_to_head(
                replay_path.clone(),
                swarmy_common::HeadToHeadQuery {
                    player_name: "Clem".to_string(),
                    opponent_name: "Serral".to_string(),
                },
                groups,
            )
            .unwrap()
            .games
            .len()
        };
        let (alone, grouped) = (
            head_to_head(Default::default()),
            head_to_head(groups.clone()),
        );
        eprintln!("Clem vs Serral: {alone} games by name, {grouped} with the grouped account");
        assert!(grouped >= alone);
    }
}
