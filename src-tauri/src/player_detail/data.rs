//! Statistics of a player across the snapshot.

use crate::data::*;
use crate::replay_detail::data::unit_effectiveness;
use crate::replay_rerun::decisions::{base_unit_name, has_cost, latest_balance_units, unit_cost};
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

pub fn try_query_player_detail(
    replay_path: String,
    player_name: String,
) -> Result<PlayerDetail, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    let player_name = name_without_clan(&player_name).to_string();
    let mut details = scan_snapshot_ipc(&ipc_path, DETAILS_IPC)?;
    // Snapshots written before s2protocol-rs stored the protocol version in details.arrow.
    let has_protocol_version = details.collect_schema()?.contains("ext_protocol_version");
    let protocol_version = if has_protocol_version {
        col("ext_protocol_version")
    } else {
        lit(0u32).alias("ext_protocol_version")
    };
    let games = details
        .clone()
        .filter(player_name_without_clan().eq(lit(player_name.clone())))
        .select([
            protocol_version,
            col("ext_fs_id"),
            col("player_race"),
            col("player_result"),
            col("player_team_id"),
            col("title"),
            col("ext_fs_file_name"),
            col("ext_datetime").dt().to_string("%Y-%m-%d").alias("date"),
        ]);
    let opponents = details
        .filter(player_name_without_clan().neq(lit(player_name.clone())))
        .select([
            col("ext_fs_id"),
            player_name_without_clan().alias("opponent"),
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
        .map(
            |((((((ext_fs_id, date), map_title), race), opponent), opponent_race), result)| {
                PlayerMatch {
                    ext_fs_id: *ext_fs_id,
                    date,
                    map_title,
                    race: normalize_race(&race),
                    opponent,
                    opponent_race: normalize_race(&opponent_race),
                    won: result == "Win",
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
    for game in matches
        .iter()
        .filter(|game| seen_games.insert(game.ext_fs_id))
    {
        wins += game.won as usize;
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
    let units = query_unit_effectiveness(&ipc_path, &player_name, &game_ids)?;
    let insights =
        super::insights::player_insights(&ipc_path, &player_name, &matches, &files_by_id)?;
    Ok(PlayerDetail {
        player_name,
        games: game_ids.len(),
        wins,
        races,
        by_month: by_month.into_values().collect(),
        by_version,
        units,
        opponents,
        matches,
        insights,
    })
}

/// Counts the units produced by the player and the enemy units they killed.
fn query_unit_effectiveness(
    ipc_path: &str,
    player_name: &str,
    game_ids: &[u64],
) -> Result<Vec<UnitEffectiveness>, SwarmyError> {
    let born = scan_snapshot_ipc(ipc_path, UNIT_BORN_IPC)?
        .filter(
            is_in_replays(game_ids)
                .and(player_name_without_clan().eq(lit(player_name.to_string()))),
        )
        .select([
            col("ext_fs_id"),
            col("control_player_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            col("unit_type_name"),
        ])
        .collect()?;
    let born_ids = u64_values(&born, "ext_fs_id")?;
    let control_player_ids = u8_values(&born, "control_player_id")?;
    let tag_indexes = u32_values(&born, "unit_tag_index")?;
    let tag_recycles = u32_values(&born, "unit_tag_recycle")?;
    let unit_names = str_values(&born, "unit_type_name")?;
    // The tracker player id of the player in each game, kills are attributed by it.
    let mut player_ids: HashSet<(u64, u8)> = HashSet::new();
    // Morphs, burrowing, etc register the same unit again, each unit is counted once.
    let mut produced_units: HashSet<(u64, u32, u32, &str)> = HashSet::new();
    let mut produced: BTreeMap<String, usize> = BTreeMap::new();
    for idx in 0..born.height() {
        if let Some(player_id) = control_player_ids[idx] {
            player_ids.insert((born_ids[idx], player_id));
        }
        let name = base_unit_name(&unit_names[idx]);
        if produced_units.insert((born_ids[idx], tag_indexes[idx], tag_recycles[idx], name)) {
            *produced.entry(name.to_string()).or_default() += 1;
        }
    }

    let died = scan_snapshot_ipc(ipc_path, UNIT_DIED_IPC)?
        .filter(is_in_replays(game_ids).and(col("killer_player_id").is_not_null()))
        .select([
            col("ext_fs_id"),
            col("killer_player_id"),
            col("unit_killer_name"),
            col("unit_died_name"),
        ])
        .collect()?;
    let died_ids = u64_values(&died, "ext_fs_id")?;
    let killer_player_ids = u8_values(&died, "killer_player_id")?;
    let killer_names = str_values(&died, "unit_killer_name")?;
    let died_names = str_values(&died, "unit_died_name")?;
    let balance = latest_balance_units()?;
    let mut kills: BTreeMap<String, (usize, u32)> = BTreeMap::new();
    for idx in 0..died.height() {
        let Some(killer_player_id) = killer_player_ids[idx] else {
            continue;
        };
        // Larva, eggs, broodlings, etc are not worth counting.
        if !player_ids.contains(&(died_ids[idx], killer_player_id))
            || !has_cost(balance, base_unit_name(&died_names[idx]))
        {
            continue;
        }
        let cost = unit_cost(balance, base_unit_name(&died_names[idx]));
        let unit_kills = kills
            .entry(base_unit_name(&killer_names[idx]).to_string())
            .or_default();
        unit_kills.0 += 1;
        unit_kills.1 += cost.minerals + cost.vespene;
    }
    let units = unit_effectiveness(produced, &kills, balance);
    Ok(units)
}
