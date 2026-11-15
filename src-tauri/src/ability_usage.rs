//! How often the abilities with energy or a cooldown are used in the games of each patch, i.e.
//! Abduct, Fungal Growth or Yamato Cannon. A replay is compared with the games of its patch, the
//! Patch Overview, Patch Evolution and Race Detail views show the averages.
//!
//! The uses are the commands of the players, a command is not always cast, i.e. a Psionic Storm
//! can be cancelled before the High Templar is in range.
//!
//! The three frames are query files of this directory (AGENTS.md, "A query is a file"), run
//! together once per snapshot and kept in memory.

pub mod by_player;
pub mod game_uses;
pub mod totals;

use crate::data::*;
use crate::host::SwarmyHost;
use crate::replay_rerun::decisions::closest_balance_version;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use polars::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

/// The patch of the games of all the patches, for the Race Detail.
pub const ALL_PATCHES: u32 = 0;

/// The abilities of the race the player did not use are shown when the players of the patch use
/// them at least this often per game.
const COMMON_USES_PER_GAME: f64 = 0.5;

/// The use of the tracked abilities in the games of the snapshot.
#[derive(Debug, Default, Clone)]
pub struct AbilityUsage {
    /// Without the games of [`ALL_PATCHES`].
    pub table: AbilityUsageTable,
    /// Of the games of [`ALL_PATCHES`], by ability.
    pub all_patches: HashMap<String, AbilityPatchUsage>,
    /// The uses of each player game in which the ability was used, the most first, by patch and
    /// ability.
    game_uses: HashMap<(u32, String), Vec<u32>>,
    /// The ranked players, the most uses per game first, by patch and ability.
    player_ranks: HashMap<(u32, String), Vec<AbilityPlayerUsage>>,
    /// The uses of each ability by each player of each game, by `ext_fs_id` and player name.
    player_game: HashMap<(u64, String), HashMap<String, u32>>,
}

impl AbilityUsage {
    /// The use of the ability in the games of the patch, or of [`ALL_PATCHES`].
    /// The uses of the abilities by the player in the game, none when the game is not in the
    /// snapshot or the player used none.
    pub fn player_game_uses(
        &self,
        ext_fs_id: u64,
        player_name: &str,
    ) -> Option<&HashMap<String, u32>> {
        self.player_game.get(&(ext_fs_id, player_name.to_string()))
    }

    pub fn patch_usage(&self, patch: u32, ability: &str) -> Option<&AbilityPatchUsage> {
        if patch == ALL_PATCHES {
            self.all_patches.get(ability)
        } else {
            self.table.get(patch, ability)
        }
    }

    /// The rank of the uses in a player game among the games of the race in the patch.
    pub fn game_rank(&self, patch: u32, ability: &str, uses: u32) -> Option<Rank> {
        let usage = self.patch_usage(patch, ability)?;
        // The uses are sorted, the most first.
        let more = self
            .game_uses
            .get(&(patch, ability.to_string()))
            .map(|uses_by_game| uses_by_game.partition_point(|other| *other > uses))
            .unwrap_or_default();
        Some(Rank {
            rank: more + 1,
            of: usage.player_games,
        })
    }

    /// The rank of the player by uses per game in the patch.
    pub fn player_rank(&self, patch: u32, ability: &str, player_name: &str) -> Option<Rank> {
        let ranked = self.player_ranks.get(&(patch, ability.to_string()))?;
        let position = ranked
            .iter()
            .position(|player| player.player_name == player_name)?;
        Some(Rank {
            rank: position + 1,
            of: ranked.len(),
        })
    }
}

/// The queries of the usage, for the debug links of the views that show it.
pub const QUERIES: [QueryMeta; 3] = [game_uses::META, totals::META, by_player::META];

pub fn compute_ability_usage(ipc_path: &str) -> Result<AbilityUsage, SwarmyError> {
    let source = IpcSnapshot::at(ipc_path);
    let tracked = game_uses::tracked_abilities();
    // One plan, the scans of the abilities and the players are shared by the three results.
    let [totals, by_game, by_player]: [DataFrame; 3] = collect_all(vec![
        totals::plan(&source, tracked.clone())?,
        game_uses::plan(&source, tracked.clone())?,
        by_player::plan(&source, tracked)?,
    ])?
    .try_into()
    .map_err(|_| SwarmyError::Other("The ability usage returned no results".into()))?;

    let mut usage = AbilityUsage::default();
    for (((patch, ability), uses), (player_name, games)) in u32_values(&by_player, "patch")?
        .into_iter()
        .zip(str_values(&by_player, "ability")?)
        .zip(u32_values(&by_player, "uses")?)
        .zip(
            str_values(&by_player, "player_name")?
                .into_iter()
                .zip(u32_values(&by_player, "games")?),
        )
    {
        usage
            .player_ranks
            .entry((patch, ability))
            .or_default()
            .push(AbilityPlayerUsage {
                player_name,
                games: games as usize,
                uses: uses as usize,
            });
    }
    // The uses of each game, the most first (the game rows come by game).
    let mut game_rows: Vec<(u32, String, u32, u64, String)> = u32_values(&by_game, "patch")?
        .into_iter()
        .zip(str_values(&by_game, "ability")?)
        .zip(u32_values(&by_game, "uses")?)
        .zip(
            u64_values(&by_game, "ext_fs_id")?
                .into_iter()
                .zip(str_values(&by_game, "player_name")?),
        )
        .map(|(((patch, ability), uses), (ext_fs_id, player_name))| {
            (patch, ability, uses, ext_fs_id, player_name)
        })
        .collect();
    game_rows.sort_by(|a, b| (a.0, &a.1, b.2, a.3, &a.4).cmp(&(b.0, &b.1, a.2, b.3, &b.4)));
    for (patch, ability, uses, ext_fs_id, player_name) in game_rows {
        if patch != ALL_PATCHES {
            usage
                .player_game
                .entry((ext_fs_id, player_name))
                .or_default()
                .insert(ability.clone(), uses);
        }
        usage
            .game_uses
            .entry((patch, ability))
            .or_default()
            .push(uses);
    }
    let mut rows: Vec<AbilityPatchUsage> = u32_values(&totals, "patch")?
        .into_iter()
        .zip(str_values(&totals, "ability")?)
        .zip(u32_values(&totals, "player_games")?)
        .zip(
            u32_values(&totals, "games_used")?
                .into_iter()
                .zip(u32_values(&totals, "uses")?),
        )
        .map(
            |(((patch, ability), player_games), (games_used, uses))| AbilityPatchUsage {
                top_players: usage
                    .player_ranks
                    .get(&(patch, ability.clone()))
                    .map(|ranked| ranked.iter().take(3).cloned().collect())
                    .unwrap_or_default(),
                patch,
                ability,
                player_games: player_games as usize,
                games_used: games_used as usize,
                uses: uses as usize,
            },
        )
        .collect();
    let order = |ability: &str| {
        TRACKED_ABILITIES
            .iter()
            .position(|tracked| tracked.ability == ability)
    };
    rows.sort_by_key(|row| (row.patch, order(&row.ability)));
    for row in rows {
        if row.patch == ALL_PATCHES {
            usage.all_patches.insert(row.ability.clone(), row);
        } else {
            if usage.table.patches.last() != Some(&row.patch) {
                usage.table.patches.push(row.patch);
            }
            usage.table.usage.push(row);
        }
    }
    Ok(usage)
}

/// The snapshot directory and the time it was written.
type CacheKey = (String, Option<std::time::SystemTime>);

/// The usage of the snapshot, computed once per snapshot generation.
pub fn ability_usage_cached(ipc_path: &str) -> Result<Arc<AbilityUsage>, SwarmyError> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Arc<AbilityUsage>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (ipc_path.to_string(), snapshot_generation(ipc_path));
    if let Some(usage) = cache.lock().ok().and_then(|cache| cache.get(&key).cloned()) {
        return Ok(usage);
    }
    let usage = Arc::new(compute_ability_usage(ipc_path)?);
    if let Ok(mut cache) = cache.lock() {
        cache.insert(key, usage.clone());
    }
    Ok(usage)
}

/// Compares the abilities used by the players of the replay with the games of its patch, and
/// adds the common abilities of their race they did not use.
pub fn annotate(detail: &mut ReplayDetail, usage: &AbilityUsage) {
    let Ok(patch) = closest_balance_version(detail.protocol_version) else {
        return;
    };
    for summary in &mut detail.players {
        // The uses of the snapshot, so that the game and the patch are counted in the same way,
        // the ones of the replay analysis when the game is not in the snapshot.
        let uses: HashMap<String, u32> =
            match usage.player_game_uses(detail.ext_fs_id, &summary.player.name) {
                Some(uses) => uses.clone(),
                None => summary
                    .abilities
                    .iter()
                    .map(|ability| (ability.ability.clone(), ability.uses))
                    .collect(),
            };
        summary.abilities = race_abilities(&summary.player.race)
            .filter_map(|tracked| {
                let uses = uses.get(tracked.ability).copied().unwrap_or_default();
                let patch_usage = usage.table.get(patch, tracked.ability).cloned();
                let is_common = patch_usage
                    .as_ref()
                    .is_some_and(|usage| usage.per_game() >= COMMON_USES_PER_GAME);
                if uses == 0 && !is_common {
                    return None;
                }
                Some(AbilityUse {
                    ability: tracked.ability.to_string(),
                    uses,
                    game_rank: usage.game_rank(patch, tracked.ability, uses),
                    player_rank: usage.player_rank(
                        patch,
                        tracked.ability,
                        crate::player_detail::game_insights::name_without_clan(
                            &summary.player.name,
                        ),
                    ),
                    patch: patch_usage,
                })
            })
            .collect();
    }
}

pub fn try_query_ability_usage(replay_path: String) -> Result<AbilityUsageTable, SwarmyError> {
    let replay_path = sanitize_replay_path(&replay_path)?;
    let ipc_path = build_ipc_path(&replay_path)?;
    Ok(ability_usage_cached(&ipc_path)?.table.clone())
}

/// The use of the tracked abilities in the games of each patch of the snapshot.
pub async fn query_ability_usage(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new()
            .with_sources(QUERIES.map(QuerySource::from))
            .process_result(try_query_ability_usage(app_config.replay_path))
    })
    .await
}

#[cfg(test)]
mod snapshot_tests {
    /// Runs against a real snapshot: `SWARMY_TEST_REPLAY_PATH=~/SC2Replays/PRO cargo test -- --ignored`
    #[test]
    #[ignore]
    fn it_computes_the_ability_usage_of_a_snapshot() {
        let replay_path = std::env::var("SWARMY_TEST_REPLAY_PATH").unwrap();
        let ipc_path = format!("{}/{}", replay_path, swarmy_common::IPC_DIR);
        let start = std::time::Instant::now();
        let usage = super::compute_ability_usage(&ipc_path).unwrap();
        eprintln!("{:?}", start.elapsed());
        eprintln!("patches {:?}", usage.table.patches);
        let mut all: Vec<_> = usage.all_patches.values().collect();
        all.sort_by(|a, b| b.per_game().total_cmp(&a.per_game()));
        for row in all.iter().take(12) {
            eprintln!(
                "{:<32} games {:>5} used {:>5.1}% per game {:>5.2} top {:?}",
                row.ability,
                row.player_games,
                row.used_percent(),
                row.per_game(),
                row.top_players.first()
            );
        }
        let inject = &usage.all_patches["SpawnLarva.Execute"];
        assert!(inject.uses > 0 && inject.games_used <= inject.player_games);
        let rank = usage
            .game_rank(super::ALL_PATCHES, "SpawnLarva.Execute", u32::MAX)
            .unwrap();
        assert_eq!(rank.rank, 1);
        assert!(!usage.table.patches.contains(&super::ALL_PATCHES));
    }
}

/// Adds to each game of the Replay List its best game rank: of its players and abilities, the
/// highest among the games of the race on the patch.
pub fn annotate_replay_list(
    ipc_path: &str,
    replays: &mut [ReplayListEntry],
    usage: &AbilityUsage,
) -> Result<(), SwarmyError> {
    let ids: Vec<u64> = replays.iter().map(|replay| replay.ext_fs_id).collect();
    let versions = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .filter(crate::data::is_in_replays(&ids))
        .select([col("ext_fs_id"), col("ext_protocol_version")])
        .unique(Some(cols(["ext_fs_id"])), UniqueKeepStrategy::Any)
        .collect()?;
    let versions: HashMap<u64, u32> = u64_values(&versions, "ext_fs_id")?
        .into_iter()
        .zip(u32_values(&versions, "ext_protocol_version")?)
        .collect();
    for replay in replays.iter_mut() {
        let Some(patch) = versions
            .get(&replay.ext_fs_id)
            .and_then(|version| closest_balance_version(*version).ok())
        else {
            continue;
        };
        let mut best: Option<GameRankHighlight> = None;
        for (idx, player_name) in replay.player_list.iter().enumerate() {
            let Some(uses) = usage.player_game_uses(replay.ext_fs_id, player_name) else {
                continue;
            };
            for (ability, uses) in uses {
                let Some(rank) = usage.game_rank(patch, ability, *uses) else {
                    continue;
                };
                // The same rank twice: the most used ability, then the first by name, so that
                // the list shows the same highlight each time (the uses come from a map).
                let better = best.as_ref().is_none_or(|best| {
                    (
                        rank.rank,
                        rank.top_percent() as u64,
                        std::cmp::Reverse(*uses),
                        ability.as_str(),
                    ) < (
                        best.rank.rank,
                        best.rank.top_percent() as u64,
                        std::cmp::Reverse(best.uses),
                        best.ability.as_str(),
                    )
                });
                if *uses > 0 && better {
                    best = Some(GameRankHighlight {
                        player_name: player_name.clone(),
                        race: replay.player_races.get(idx).cloned().unwrap_or_default(),
                        ability: ability.clone(),
                        uses: *uses,
                        rank,
                    });
                }
            }
        }
        replay.best_rank = best;
    }
    Ok(())
}
