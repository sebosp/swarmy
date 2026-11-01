//! The time of the queries of the views on a snapshot, to find the slow ones:
//! SWARMY_TEST_REPLAYS=~/SC2Replays cargo test --release bench_queries -- --ignored --nocapture

use swarmy_common::*;

fn timed<T>(name: &str, query: impl FnOnce() -> Result<T, SwarmyError>) -> Option<T> {
    let start = std::time::Instant::now();
    let result = query();
    println!(
        "{name:<28} {:>10.1?} {}",
        start.elapsed(),
        if result.is_ok() { "" } else { "FAILED" }
    );
    result.ok()
}

#[test]
#[ignore]
fn bench_queries() {
    let replays = std::env::var("SWARMY_TEST_REPLAYS").unwrap();
    let caches = std::env::var("SWARMY_TEST_CACHES").unwrap_or_default();
    let players = timed("list_players", || {
        crate::player_detail::head_to_head::try_list_players(replays.clone())
    })
    .unwrap_or_default();
    let maps = timed("list_maps", || {
        crate::map_details::data::try_list_maps(replays.clone())
    })
    .unwrap_or_default();
    let player = players.first().map(|p| p.label.clone()).unwrap_or_default();
    let opponent = players.get(1).map(|p| p.label.clone()).unwrap_or_default();
    let map = maps.first().map(|m| m.label.clone()).unwrap_or_default();
    println!("player {player:?} opponent {opponent:?} map {map:?}");
    timed("snapshot_metadata", || {
        crate::try_get_snapshot_metadata(replays.clone(), caches.clone())
    });
    let list = timed("replay_list", || {
        crate::replay_list::data::try_query_replay_list(
            replays.clone(),
            ReplayListQuery::default(),
            None,
        )
    });
    timed("replay_list player", || {
        crate::replay_list::data::try_query_replay_list(
            replays.clone(),
            ReplayListQuery {
                player_name: player.clone(),
                ..Default::default()
            },
            None,
        )
    });
    // SWARMY_TEST_EXT_FS_ID picks the replay, i.e. one with abilities, the newest one by default.
    let ext_fs_id = std::env::var("SWARMY_TEST_EXT_FS_ID")
        .ok()
        .and_then(|id| id.parse().ok())
        .or_else(|| list.and_then(|list| list.replays.first().map(|replay| replay.ext_fs_id)))
        .unwrap_or_default();
    let detail = timed("replay_detail", || {
        crate::replay_detail::data::try_query_replay_detail(replays.clone(), ext_fs_id)
    });
    for summary in detail.iter().flat_map(|detail| detail.players.iter()) {
        let abilities: Vec<String> = summary
            .abilities
            .iter()
            .map(|ability| {
                format!(
                    "{} {} (avg {:.1}, game {:?}, player {:?})",
                    ability.ability,
                    ability.uses,
                    ability
                        .patch
                        .as_ref()
                        .map(|patch| patch.per_game())
                        .unwrap_or_default(),
                    ability.game_rank.map(|rank| (rank.rank, rank.of)),
                    ability.player_rank.map(|rank| (rank.rank, rank.of)),
                )
            })
            .collect();
        println!(
            "  {} {} commands {} chrono {}: {abilities:?}",
            summary.player.name,
            summary.player.race,
            summary
                .macro_summary
                .commands_per_minute
                .iter()
                .sum::<u32>(),
            summary.macro_summary.chrono_boosts
        );
    }
    timed("replay_detail cached", || {
        crate::replay_detail::data::try_query_replay_detail(replays.clone(), ext_fs_id)
    });
    timed("adjacent_replay", || {
        crate::replay_detail::navigation::try_adjacent_replay(
            replays.clone(),
            AdjacentReplayQuery {
                ext_fs_id: Some(ext_fs_id),
                step: ReplayStep::Previous,
                ..Default::default()
            },
        )
    });
    timed("map_stats", || {
        crate::map_stats::data::try_query_map_stats(replays.clone(), MapStatsQuery::default())
    });
    timed("map_details", || {
        crate::map_details::data::try_query_map_details(
            replays.clone(),
            caches.clone(),
            map.clone(),
        )
    });
    timed("map_scout", || {
        crate::map_scout::data::try_query_map_scout(
            replays.clone(),
            caches.clone(),
            MapScoutQuery {
                map_title: map.clone(),
                max_seconds: 180,
                ..Default::default()
            },
        )
    });
    timed("player_detail", || {
        crate::player_detail::data::try_query_player_detail(replays.clone(), player.clone())
    });
    timed("player_detail cached", || {
        crate::player_detail::data::try_query_player_detail(replays.clone(), player.clone())
    });
    timed("head_to_head", || {
        crate::player_detail::head_to_head::try_query_head_to_head(
            replays.clone(),
            HeadToHeadQuery {
                player_name: player.clone(),
                opponent_name: opponent.clone(),
            },
        )
    });
    timed("unit_snapshot_stats", || {
        crate::patch_overview::unit_stats::try_query_unit_snapshot_stats(
            replays.clone(),
            "Marine".to_string(),
        )
    });
}
