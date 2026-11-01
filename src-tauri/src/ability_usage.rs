//! How often the abilities with energy or a cooldown are used in the games of each patch, i.e.
//! Abduct, Fungal Growth or Yamato Cannon. A replay is compared with the games of its patch, the
//! Patch Overview, Patch Evolution and Race Detail views show the averages.
//!
//! The uses are the commands of the players, a command is not always cast, i.e. a Psionic Storm
//! can be cancelled before the High Templar is in range.

use crate::data::*;
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
        let more = self
            .game_uses
            .get(&(patch, ability.to_string()))
            .map(|uses_by_game| {
                uses_by_game
                    .iter()
                    .take_while(|other| **other > uses)
                    .count()
            })
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

/// The tracked abilities and their race, to keep the uses of the abilities of the race of the
/// player: a Zerg that took a Ghost with a Neural Parasite does not use EMP.
fn abilities_frame() -> Result<LazyFrame, SwarmyError> {
    let abilities: Vec<&str> = TRACKED_ABILITIES.iter().map(|t| t.ability).collect();
    let races: Vec<&str> = TRACKED_ABILITIES.iter().map(|t| t.race).collect();
    Ok(DataFrame::new(
        abilities.len(),
        vec![
            Column::new("ability".into(), abilities),
            Column::new("race".into(), races),
        ],
    )?
    .lazy())
}

/// The commands of the tracked abilities of both cmd files, the uses of each player in each game.
///
/// The cmd files have one row per unit selected when the command was given, a Psionic Storm
/// commanded with four High Templar selected is four rows, the commands are counted once by their
/// sequence number.
fn player_game_uses(ipc_path: &str) -> Result<LazyFrame, SwarmyError> {
    let abilities: Vec<&str> = TRACKED_ABILITIES.iter().map(|t| t.ability).collect();
    let command = ["ext_fs_id", "user_id", "sequence", "ext_replay_loop"];
    let commands = [CMD_TARGET_POINT_IPC, CMD_TARGET_UNIT_IPC]
        .iter()
        .map(|file_name| {
            Ok(scan_snapshot_ipc(ipc_path, file_name)?
                .filter(is_one_of("ability", &abilities).and(col("player_name").is_not_null()))
                .select(
                    command
                        .iter()
                        .chain(["player_name", "ability"].iter())
                        .map(|name| col(*name))
                        .collect::<Vec<_>>(),
                ))
        })
        .collect::<Result<Vec<_>, SwarmyError>>()?;
    Ok(concat(commands, UnionArgs::default())?
        .unique(Some(cols(command)), UniqueKeepStrategy::Any)
        .group_by([col("ext_fs_id"), col("player_name"), col("ability")])
        .agg([len().cast(DataType::UInt64).alias("uses")]))
}

/// Computes the usage of the tracked abilities by patch.
pub fn compute_ability_usage(ipc_path: &str) -> Result<AbilityUsage, SwarmyError> {
    let players = snapshot_players(ipc_path)?.select([
        col("ext_fs_id"),
        col("player_name"),
        col("race"),
        col("patch"),
    ]);
    // The games of every patch, and once more as the games of all the patches.
    let players = concat(
        [
            players.clone(),
            players.with_column(lit(ALL_PATCHES).cast(DataType::UInt32).alias("patch")),
        ],
        UnionArgs::default(),
    )?;
    let game_uses = player_game_uses(ipc_path)?
        .join(
            players.clone(),
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .join(
            abilities_frame()?,
            [col("ability"), col("race")],
            [col("ability"), col("race")],
            JoinArgs::new(JoinType::Inner),
        );
    let race_games = players
        .clone()
        .group_by([col("patch"), col("race")])
        .agg([len().cast(DataType::UInt64).alias("player_games")]);
    let player_games = players
        .group_by([col("patch"), col("race"), col("player_name")])
        .agg([len().cast(DataType::UInt64).alias("games")]);

    let totals = game_uses
        .clone()
        .group_by([col("patch"), col("race"), col("ability")])
        .agg([
            len().cast(DataType::UInt64).alias("games_used"),
            col("uses").sum(),
        ])
        .join(
            race_games,
            [col("patch"), col("race")],
            [col("patch"), col("race")],
            JoinArgs::new(JoinType::Inner),
        );
    let by_game = game_uses
        .clone()
        .select([
            col("patch"),
            col("ability"),
            col("uses"),
            col("ext_fs_id"),
            col("player_name"),
        ])
        .sort(
            ["patch", "ability", "uses"],
            SortMultipleOptions::default().with_order_descending_multi([false, false, true]),
        );
    let by_player = game_uses
        .group_by([
            col("patch"),
            col("race"),
            col("ability"),
            col("player_name"),
        ])
        .agg([col("uses").sum()])
        .join(
            player_games,
            [col("patch"), col("race"), col("player_name")],
            [col("patch"), col("race"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .filter(col("games").gt_eq(lit(ABILITY_RANK_MIN_GAMES as u64)))
        .with_column(
            (col("uses").cast(DataType::Float64) / col("games").cast(DataType::Float64))
                .alias("per_game"),
        )
        .sort(
            ["patch", "ability", "per_game", "player_name"],
            SortMultipleOptions::default().with_order_descending_multi([false, false, true, false]),
        );
    // One plan, the scans of the cmd files and the players are shared by the three results.
    let [totals, by_game, by_player]: [DataFrame; 3] =
        collect_all(vec![totals, by_game, by_player])?
            .try_into()
            .map_err(|_| SwarmyError::Other("The ability usage returned no results".into()))?;

    let mut usage = AbilityUsage::default();
    for (((patch, ability), uses), (player_name, games)) in u32_values(&by_player, "patch")?
        .into_iter()
        .zip(str_values(&by_player, "ability")?)
        .zip(u64_values(&by_player, "uses")?)
        .zip(
            str_values(&by_player, "player_name")?
                .into_iter()
                .zip(u64_values(&by_player, "games")?),
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
    for (((patch, ability), uses), (ext_fs_id, player_name)) in u32_values(&by_game, "patch")?
        .into_iter()
        .zip(str_values(&by_game, "ability")?)
        .zip(u64_values(&by_game, "uses")?)
        .zip(
            u64_values(&by_game, "ext_fs_id")?
                .into_iter()
                .zip(str_values(&by_game, "player_name")?),
        )
    {
        if patch != ALL_PATCHES {
            usage
                .player_game
                .entry((ext_fs_id, player_name))
                .or_default()
                .insert(ability.clone(), uses as u32);
        }
        usage
            .game_uses
            .entry((patch, ability))
            .or_default()
            .push(uses as u32);
    }
    let mut rows: Vec<AbilityPatchUsage> = u32_values(&totals, "patch")?
        .into_iter()
        .zip(str_values(&totals, "ability")?)
        .zip(u64_values(&totals, "player_games")?)
        .zip(
            u64_values(&totals, "games_used")?
                .into_iter()
                .zip(u64_values(&totals, "uses")?),
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
                    player_rank: usage.player_rank(patch, tracked.ability, &summary.player.name),
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
#[tauri::command(rename_all = "snake_case")]
pub async fn query_ability_usage(app_handle: tauri::AppHandle) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(&app_handle)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_ability_usage(app_config.replay_path))
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
