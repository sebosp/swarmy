//! One ability in the games of the snapshot, for the Ability view: who uses it the most, its use
//! over time, by patch and by matchup, the games with the most uses, and when it is first used.
//! The rows come from the query file `player_games.rs` (AGENTS.md, "A query is a file"), this
//! file adds them up.

pub mod player_games;

use crate::data::{IpcSnapshot, QueryMeta};
use crate::host::SwarmyHost;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use player_games::{PlayerGame, TEAM_GAMES};
use std::collections::{BTreeMap, HashMap};
use swarmy_common::*;

/// The queries of the view, for the debug links.
pub const QUERIES: [QueryMeta; 1] = [player_games::META];

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
    replay_path: &str,
    ability: &str,
) -> Result<AbilityDetail, SwarmyError> {
    let tracked = tracked_ability(ability)
        .ok_or_else(|| SwarmyError::Other(format!("{ability} is not a tracked ability")))?;
    let source = IpcSnapshot::open(replay_path)?;
    let games = player_games::shape(player_games::plan(&source, tracked)?.collect()?)?;
    Ok(aggregate(ability, games))
}

/// The detail of an ability in the games of the snapshot.
pub async fn query_ability_detail(
    host: &dyn SwarmyHost,
    query: AbilityDetailQuery,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(try_query_ability_detail(
                &app_config.replay_path,
                &query.ability,
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
        let detail = super::try_query_ability_detail(&replay_path, "SpawnLarva.Execute").unwrap();
        eprintln!("{:?}", start.elapsed());
        eprintln!("{:?}", detail.totals);
        eprintln!("fastest {:?}", detail.fastest);
        eprintln!("matchups {:?}", detail.by_matchup);
        eprintln!("top {:?}", detail.top_players.first());
        assert!(detail.totals.uses > 0);
        assert!(!detail.by_patch.is_empty());
    }
}
