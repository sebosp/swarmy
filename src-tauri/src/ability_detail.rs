//! One ability in the games of the snapshot, for the Ability view: who uses it the most, its use
//! over time, by patch and by matchup, the games with the most uses, and when it is first used.

use crate::data::*;
use crate::host::SwarmyHost;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap};
use swarmy_common::*;

/// The matchup of the games of more than two players.
const TEAM_GAMES: &str = "Team games";

/// A player game of the race of the ability, with its uses.
#[derive(Debug, Clone)]
struct PlayerGame {
    game: AbilityGameUse,
    patch: u32,
}

/// The uses of the ability per player and game, the commands counted once by their sequence
/// number as in `ability_usage`, with the first use.
fn ability_uses(ipc_path: &str, ability: &str) -> Result<LazyFrame, SwarmyError> {
    Ok(scan_snapshot_ipc(ipc_path, GAME_ABILITIES_IPC)?
        .filter(col("ability").eq(lit(ability)))
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("uses").cast(DataType::UInt64),
            col("first_seconds"),
        ]))
}

/// Every game of a player of the race of the ability, with its uses (0 when not used), map,
/// patch, date and opponent race.
fn player_games(ipc_path: &str, tracked: &TrackedAbility) -> Result<Vec<PlayerGame>, SwarmyError> {
    let players = snapshot_players(ipc_path)?;
    let opponents = players.clone().select([
        col("ext_fs_id"),
        col("player_name").alias("opponent"),
        col("race").alias("opponent_race"),
    ]);
    let matchups = players
        .clone()
        .select([col("ext_fs_id"), col("player_name")])
        .join(
            opponents,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("player_name").neq(col("opponent")))
        .group_by([col("ext_fs_id"), col("player_name")])
        .agg([
            col("opponent_race").first(),
            len().cast(DataType::UInt32).alias("opponents"),
        ]);
    let dates = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .select([
            col("ext_fs_id"),
            col("ext_datetime").dt().to_string("%Y-%m-%d").alias("date"),
        ])
        .unique(Some(cols(["ext_fs_id"])), UniqueKeepStrategy::Any);
    let keys = [col("ext_fs_id"), col("player_name")];
    let games = players
        .filter(col("race").eq(lit(tracked.race)))
        .select([
            col("ext_fs_id"),
            col("player_name"),
            col("title"),
            col("patch"),
        ])
        .join(
            ability_uses(ipc_path, tracked.ability)?,
            keys.clone(),
            keys.clone(),
            JoinArgs::new(JoinType::Left),
        )
        .join(matchups, keys.clone(), keys, JoinArgs::new(JoinType::Left))
        .join(
            dates,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Left),
        )
        .with_columns([col("uses").fill_null(lit(0u64))])
        .collect()?;
    let ids = u64_values(&games, "ext_fs_id")?;
    let names = str_values(&games, "player_name")?;
    let titles = str_values(&games, "title")?;
    let patches = u32_values(&games, "patch")?;
    let uses = u64_values(&games, "uses")?;
    let first: Vec<Option<u32>> = games.column("first_seconds")?.u32()?.iter().collect();
    let opponent_races = str_values(&games, "opponent_race")?;
    let opponents: Vec<Option<u32>> = games.column("opponents")?.u32()?.iter().collect();
    let dates = str_values(&games, "date")?;
    Ok((0..games.height())
        .map(|idx| PlayerGame {
            game: AbilityGameUse {
                ext_fs_id: ids[idx],
                player_name: names[idx].clone(),
                map_title: titles[idx].clone(),
                date: dates[idx].clone(),
                matchup: match opponents[idx] {
                    Some(1) => matchup_label(tracked.race, &opponent_races[idx]),
                    _ => TEAM_GAMES.to_string(),
                },
                uses: uses[idx] as usize,
                first_seconds: first[idx],
            },
            patch: patches[idx],
        })
        .collect())
}

/// The sums of a group of player games.
#[derive(Default)]
struct Totals {
    player_games: usize,
    games_used: usize,
    uses: usize,
    first_seconds: u64,
}

impl Totals {
    fn add(&mut self, game: &AbilityGameUse) {
        self.player_games += 1;
        self.uses += game.uses;
        if game.uses > 0 {
            self.games_used += 1;
            self.first_seconds += game.first_seconds.unwrap_or_default() as u64;
        }
    }

    fn usage(&self, label: String) -> AbilityGroupUsage {
        AbilityGroupUsage {
            label,
            player_games: self.player_games,
            games_used: self.games_used,
            uses: self.uses,
            avg_first_seconds: (self.games_used > 0)
                .then(|| (self.first_seconds / self.games_used as u64) as u32),
        }
    }
}

/// The detail of an ability from the games of the players of its race.
fn aggregate(ability: &str, games: Vec<PlayerGame>) -> AbilityDetail {
    let mut totals = Totals::default();
    let mut by_patch: BTreeMap<u32, Totals> = BTreeMap::new();
    let mut by_month: BTreeMap<String, Totals> = BTreeMap::new();
    let mut by_matchup: HashMap<String, Totals> = HashMap::new();
    let mut by_player: HashMap<String, (usize, usize)> = HashMap::new();
    for PlayerGame { game, patch } in &games {
        totals.add(game);
        by_patch.entry(*patch).or_default().add(game);
        if game.date.len() >= 7 {
            by_month
                .entry(game.date[..7].to_string())
                .or_default()
                .add(game);
        }
        by_matchup
            .entry(game.matchup.clone())
            .or_default()
            .add(game);
        let player = by_player.entry(game.player_name.clone()).or_default();
        player.0 += 1;
        player.1 += game.uses;
    }
    let mut by_matchup: Vec<AbilityGroupUsage> = by_matchup
        .into_iter()
        .map(|(label, totals)| totals.usage(label))
        .collect();
    // The most played matchup first, the team games last.
    by_matchup.sort_by(|a, b| {
        (a.label == TEAM_GAMES)
            .cmp(&(b.label == TEAM_GAMES))
            .then(b.player_games.cmp(&a.player_games))
            .then(a.label.cmp(&b.label))
    });
    let mut top_players: Vec<AbilityPlayerUsage> = by_player
        .into_iter()
        .filter(|(_, (games, uses))| *games >= ABILITY_RANK_MIN_GAMES && *uses > 0)
        .map(|(player_name, (games, uses))| AbilityPlayerUsage {
            player_name,
            games,
            uses,
        })
        .collect();
    top_players.sort_by(|a, b| {
        b.per_game()
            .total_cmp(&a.per_game())
            .then(a.player_name.cmp(&b.player_name))
    });
    top_players.truncate(ABILITY_DETAIL_TOP);
    let used: Vec<&AbilityGameUse> = games
        .iter()
        .map(|game| &game.game)
        .filter(|game| game.uses > 0)
        .collect();
    // Of the 1v1 games: the team and custom games have test maps and unusual starts.
    let fastest = used
        .iter()
        .filter(|game| game.first_seconds.is_some() && game.matchup != TEAM_GAMES)
        .min_by_key(|game| (game.first_seconds, game.ext_fs_id))
        .map(|game| (*game).clone());
    let mut top_games: Vec<AbilityGameUse> = used.into_iter().cloned().collect();
    top_games.sort_by(|a, b| b.uses.cmp(&a.uses).then(b.ext_fs_id.cmp(&a.ext_fs_id)));
    top_games.truncate(ABILITY_DETAIL_TOP);
    AbilityDetail {
        ability: ability.to_string(),
        totals: totals.usage("All".to_string()),
        fastest,
        by_patch: by_patch
            .into_iter()
            .map(|(patch, totals)| totals.usage(patch.to_string()))
            .collect(),
        by_month: by_month
            .into_iter()
            .map(|(month, totals)| totals.usage(month))
            .collect(),
        by_matchup,
        top_players,
        top_games,
    }
}

pub fn try_query_ability_detail(
    replay_path: String,
    ability: String,
) -> Result<AbilityDetail, SwarmyError> {
    let tracked = tracked_ability(&ability)
        .ok_or_else(|| SwarmyError::Other(format!("{ability} is not a tracked ability")))?;
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    Ok(aggregate(&ability, player_games(&ipc_path, tracked)?))
}

/// The detail of an ability in the games of the snapshot.
pub async fn query_ability_detail(
    host: &dyn SwarmyHost,
    query: AbilityDetailQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_ability_detail(
            app_config.replay_path,
            query.ability,
        ))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(
        id: u64,
        player: &str,
        matchup: &str,
        date: &str,
        patch: u32,
        uses: usize,
        first: Option<u32>,
    ) -> PlayerGame {
        PlayerGame {
            game: AbilityGameUse {
                ext_fs_id: id,
                player_name: player.to_string(),
                map_title: "Pylon".to_string(),
                date: date.to_string(),
                matchup: matchup.to_string(),
                uses,
                first_seconds: first,
            },
            patch,
        }
    }

    #[test]
    fn it_aggregates_the_uses_of_an_ability() {
        let games = vec![
            game(1, "A", "ZvT", "2026-09-01", 100, 10, Some(200)),
            game(2, "A", "ZvP", "2026-10-01", 101, 20, Some(100)),
            game(3, "A", "ZvT", "2026-10-02", 101, 0, None),
            game(4, "B", "ZvT", "2026-10-03", 101, 30, Some(300)),
            game(5, "B", "ZvT", "2026-10-04", 101, 0, None),
            game(6, "B", "ZvT", "2026-10-05", 101, 3, Some(400)),
        ];
        let detail = aggregate("SpawnLarva.Execute", games);
        assert_eq!(detail.totals.player_games, 6);
        assert_eq!(detail.totals.games_used, 4);
        assert_eq!(detail.totals.uses, 63);
        // (200 + 100 + 300 + 400) / 4
        assert_eq!(detail.totals.avg_first_seconds, Some(250));
        assert_eq!(detail.fastest.as_ref().map(|game| game.ext_fs_id), Some(2));
        let patches: Vec<_> = detail
            .by_patch
            .iter()
            .map(|row| (row.label.as_str(), row.uses))
            .collect();
        assert_eq!(patches, vec![("100", 10), ("101", 53)]);
        let months: Vec<_> = detail
            .by_month
            .iter()
            .map(|row| row.label.as_str())
            .collect();
        assert_eq!(months, vec!["2026-09", "2026-10"]);
        // The most played matchup first.
        assert_eq!(detail.by_matchup[0].label, "ZvT");
        assert_eq!(detail.by_matchup[0].player_games, 5);
        // B: 33 uses in 3 games, A: 30 in 3.
        let players: Vec<_> = detail
            .top_players
            .iter()
            .map(|row| row.player_name.as_str())
            .collect();
        assert_eq!(players, vec!["B", "A"]);
        assert_eq!(detail.top_games[0].ext_fs_id, 4);
        assert!(detail.top_games.iter().all(|game| game.uses > 0));
    }
}

#[cfg(test)]
mod snapshot_tests {
    /// Runs against a real snapshot: `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test -- --ignored`
    #[test]
    #[ignore]
    fn it_queries_the_detail_of_an_ability() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let start = std::time::Instant::now();
        let detail =
            super::try_query_ability_detail(replay_path, "SpawnLarva.Execute".to_string()).unwrap();
        eprintln!("{:?}", start.elapsed());
        eprintln!("{:?}", detail.totals);
        eprintln!("fastest {:?}", detail.fastest);
        eprintln!("matchups {:?}", detail.by_matchup);
        eprintln!("top {:?}", detail.top_players.first());
        assert!(detail.totals.uses > 0);
        assert!(!detail.by_patch.is_empty());
    }
}
