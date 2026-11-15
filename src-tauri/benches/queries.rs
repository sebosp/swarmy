//! The time of each query of the snapshot on a real snapshot, with criterion: the queries of the
//! query files (AGENTS.md, "A query is a file"), one group per view, each run as the view runs
//! it (`plan` then `collect`). `scripts/bench.sh` runs them and `scripts/bench-compare.py`
//! compares the medians with the baseline committed in `benches/baseline-corvid.json`: a release
//! is not made with a query 20 % slower than the baseline.
//!
//! The snapshot is the one of `SWARMY_TEST_REPLAYS` (a Replay Path); without it the benches say
//! so and end, so that `cargo bench` is harmless anywhere.
//!
//!   SWARMY_TEST_REPLAYS=~/SC2Replays/PRO cargo bench -p swarmy --bench queries
//!   cargo bench -p swarmy --bench queries -- matchups       # one query
//!   cargo bench -p swarmy --bench queries -- --save-baseline before   # criterion's own compare

use criterion::{BenchmarkId, Criterion, SamplingMode};
use polars::prelude::*;
use std::collections::HashMap;
use std::time::Duration;
use swarmy_tauri_lib::data::{IpcSnapshot, SnapshotSource};
use swarmy_tauri_lib::{map_details, player_detail};

/// The queries take 10 ms to 2 s: few samples, each one long, no linear growth of the iterations.
fn group<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(10));
    group.sampling_mode(SamplingMode::Flat);
    group
}

/// Runs the plan as the view does.
fn collect(plan: Result<LazyFrame, swarmy_common::SwarmyError>) -> DataFrame {
    plan.expect("the plan builds")
        .collect()
        .expect("the plan runs")
}

/// The most played map and player of the snapshot, the parameters of the queries: the heaviest
/// case of each.
fn parameters(source: &dyn SnapshotSource, groups: &HashMap<String, String>) -> (String, String) {
    let maps = collect(map_details::maps::plan(source));
    let map = maps
        .column("title")
        .and_then(|column| Ok(column.str()?.get(0).unwrap_or_default().to_string()))
        .expect("a map in the snapshot");
    let players = collect(player_detail::players::plan(source, groups));
    let player = players
        .column("player")
        .and_then(|column| Ok(column.str()?.get(0).unwrap_or_default().to_string()))
        .expect("a player in the snapshot");
    (map, player)
}

fn bench_map_details(c: &mut Criterion, source: &dyn SnapshotSource, map: &str) {
    let mut group = group(c, "map_details");
    group.bench_function(BenchmarkId::new("maps", "all"), |b| {
        b.iter(|| collect(map_details::maps::plan(source)))
    });
    group.bench_function(BenchmarkId::new("games_summary", map), |b| {
        b.iter(|| collect(map_details::games_summary::plan(source, map)))
    });
    group.bench_function(BenchmarkId::new("matchups", map), |b| {
        b.iter(|| collect(map_details::matchups::plan(source, map)))
    });
    group.bench_function(BenchmarkId::new("top_players", map), |b| {
        b.iter(|| collect(map_details::top_players::plan(source, map)))
    });
    group.bench_function(BenchmarkId::new("recent_games", map), |b| {
        b.iter(|| collect(map_details::recent_games::plan(source, map)))
    });
    group.finish();
}

fn bench_player_detail(
    c: &mut Criterion,
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    player: &str,
) {
    let mut group = group(c, "player_detail");
    group.bench_function(BenchmarkId::new("player_rows", "all"), |b| {
        b.iter(|| collect(player_detail::player_rows::plan(source, groups)))
    });
    group.bench_function(BenchmarkId::new("players", "all"), |b| {
        b.iter(|| collect(player_detail::players::plan(source, groups)))
    });
    group.bench_function(BenchmarkId::new("game_results", player), |b| {
        b.iter(|| collect(player_detail::game_results::plan(source, groups, player)))
    });
    group.bench_function(BenchmarkId::new("matches", player), |b| {
        b.iter(|| collect(player_detail::matches::plan(source, groups, player)))
    });
    group.bench_function(BenchmarkId::new("opponents", player), |b| {
        b.iter(|| collect(player_detail::opponents::plan(source, groups, player)))
    });
    group.bench_function(BenchmarkId::new("unit_effectiveness", player), |b| {
        b.iter(|| {
            collect(player_detail::unit_effectiveness::plan(
                source, groups, player,
            ))
        })
    });
    group.bench_function(BenchmarkId::new("accounts", player), |b| {
        b.iter(|| collect(player_detail::accounts::plan(source, groups, player)))
    });
    group.bench_function(BenchmarkId::new("similar_accounts", player), |b| {
        b.iter(|| {
            collect(player_detail::similar_accounts::plan(
                source, groups, player,
            ))
        })
    });
    group.bench_function(BenchmarkId::new("benchmarks", "all"), |b| {
        b.iter(|| collect(player_detail::benchmarks::plan(source)))
    });
    group.finish();
}

fn bench_replay_list(
    c: &mut Criterion,
    source: &dyn SnapshotSource,
    groups: &HashMap<String, String>,
    player: &str,
) {
    use swarmy_common::ReplayListQuery;
    let mut group = group(c, "replay_list");
    let newest = ReplayListQuery::newest_first();
    group.bench_function(BenchmarkId::new("games", "newest first"), |b| {
        b.iter(|| {
            collect(swarmy_tauri_lib::replay_list::games::plan(
                source, groups, &newest, None,
            ))
        })
    });
    let of_player = ReplayListQuery {
        player_name: player.to_string(),
        ..ReplayListQuery::newest_first()
    };
    group.bench_function(BenchmarkId::new("games", player), |b| {
        b.iter(|| {
            collect(swarmy_tauri_lib::replay_list::games::plan(
                source, groups, &of_player, None,
            ))
        })
    });
    group.bench_function(BenchmarkId::new("protocol_versions", "all"), |b| {
        b.iter(|| {
            collect(swarmy_tauri_lib::replay_list::protocol_versions::plan(
                source,
            ))
        })
    });
    group.bench_function(BenchmarkId::new("years", "all"), |b| {
        b.iter(|| collect(swarmy_tauri_lib::replay_list::years::plan(source)))
    });
    group.finish();
}

fn bench_progress(c: &mut Criterion, source: &dyn SnapshotSource, player: &str) {
    use swarmy_tauri_lib::progress;
    // The Main Player Name is a name without a clan tag.
    let name = progress::name_of(player);
    let mut group = group(c, "progress");
    group.bench_function(BenchmarkId::new("games", name), |b| {
        b.iter(|| collect(progress::games::plan(source, name)))
    });
    group.bench_function(BenchmarkId::new("metrics", name), |b| {
        b.iter(|| collect(progress::metrics::plan(source, name)))
    });
    group.finish();
}

fn bench_race_detail(c: &mut Criterion, source: &dyn SnapshotSource) {
    use swarmy_tauri_lib::race_detail;
    let race = "Zerg";
    let units = race_detail::units::race_units(race).expect("the balance data");
    let mut group = group(c, "race_detail");
    group.bench_function(BenchmarkId::new("player_games", race), |b| {
        b.iter(|| collect(race_detail::player_games::plan(source, race)))
    });
    group.bench_function(BenchmarkId::new("matchups", race), |b| {
        b.iter(|| collect(race_detail::matchups::plan(source, race)))
    });
    group.bench_function(BenchmarkId::new("by_patch", race), |b| {
        b.iter(|| collect(race_detail::by_patch::plan(source, race)))
    });
    group.bench_function(BenchmarkId::new("maps", race), |b| {
        b.iter(|| collect(race_detail::maps::plan(source, race)))
    });
    group.bench_function(BenchmarkId::new("top_players", race), |b| {
        b.iter(|| collect(race_detail::top_players::plan(source, race)))
    });
    group.bench_function(BenchmarkId::new("units", race), |b| {
        b.iter(|| collect(race_detail::units::plan(source, race, units.clone())))
    });
    group.finish();
}

fn bench_map_stats(c: &mut Criterion, source: &dyn SnapshotSource, player: &str) {
    use swarmy_common::MapStatsQuery;
    use swarmy_tauri_lib::map_stats;
    let mut group = group(c, "map_stats");
    group.bench_function(BenchmarkId::new("maps", "all"), |b| {
        b.iter(|| collect(map_stats::maps::plan(source, &MapStatsQuery::default())))
    });
    let by_player = MapStatsQuery {
        player_name: player.to_string(),
        ..Default::default()
    };
    group.bench_function(BenchmarkId::new("maps", player), |b| {
        b.iter(|| collect(map_stats::maps::plan(source, &by_player)))
    });
    group.finish();
}

fn bench_usage_over_time(c: &mut Criterion, source: &dyn SnapshotSource) {
    use swarmy_common::TimeGrain;
    use swarmy_tauri_lib::usage_over_time;
    let mut group = group(c, "usage_over_time");
    // The days are the most rows, the heaviest grain of each.
    group.bench_function(BenchmarkId::new("race_games", "Zerg/day"), |b| {
        b.iter(|| {
            collect(usage_over_time::race_games::plan(
                source,
                TimeGrain::Day,
                "Zerg",
            ))
        })
    });
    group.bench_function(BenchmarkId::new("ability_uses", "Transfusion/day"), |b| {
        b.iter(|| {
            collect(usage_over_time::ability_uses::plan(
                source,
                TimeGrain::Day,
                "Transfusion.Execute",
                "Zerg",
            ))
        })
    });
    group.bench_function(BenchmarkId::new("unit_uses", "Marine/day"), |b| {
        b.iter(|| {
            collect(usage_over_time::unit_uses::plan(
                source,
                TimeGrain::Day,
                "Marine",
            ))
        })
    });
    group.bench_function(BenchmarkId::new("unit_race", "Marine"), |b| {
        b.iter(|| collect(usage_over_time::unit_race::plan(source, "Marine")))
    });
    group.bench_function(BenchmarkId::new("patch_starts", "day"), |b| {
        b.iter(|| collect(usage_over_time::patch_starts::plan(source, TimeGrain::Day)))
    });
    group.finish();
}

fn bench_unit_stats(c: &mut Criterion, source: &dyn SnapshotSource) {
    use swarmy_common::EXCEPTIONAL_MIN_BORN;
    use swarmy_tauri_lib::patch_overview::unit_stats;
    let unit = "Marine";
    let compared = collect(unit_stats::kills_per_unit::plan(
        source,
        unit,
        EXCEPTIONAL_MIN_BORN,
    ));
    let threshold = unit_stats::kills_per_unit::threshold(&compared)
        .expect("the threshold")
        .threshold;
    let mut group = group(c, "unit_stats");
    // No forms: the morphs are a static table per patch, generated off-line (forms/README.md).
    group.bench_function(BenchmarkId::new("games", "all"), |b| {
        b.iter(|| collect(unit_stats::games::plan(source)))
    });
    group.bench_function(BenchmarkId::new("totals", unit), |b| {
        b.iter(|| collect(unit_stats::totals::plan(source, unit)))
    });
    group.bench_function(BenchmarkId::new("by_month", unit), |b| {
        b.iter(|| collect(unit_stats::by_month::plan(source, unit)))
    });
    group.bench_function(BenchmarkId::new("kills_per_unit", unit), |b| {
        b.iter(|| {
            collect(unit_stats::kills_per_unit::plan(
                source,
                unit,
                EXCEPTIONAL_MIN_BORN,
            ))
        })
    });
    group.bench_function(BenchmarkId::new("exceptional", unit), |b| {
        b.iter(|| {
            collect(unit_stats::exceptional::plan(
                source,
                unit,
                EXCEPTIONAL_MIN_BORN,
                threshold,
            ))
        })
    });
    group.finish();
}

fn bench_ability_detail(c: &mut Criterion, source: &dyn SnapshotSource) {
    use swarmy_common::tracked_ability;
    use swarmy_tauri_lib::ability_detail;
    let ability = "SpawnLarva.Execute";
    let tracked = tracked_ability(ability).expect("a tracked ability");
    let mut group = group(c, "ability_detail");
    group.bench_function(BenchmarkId::new("player_games", "SpawnLarva"), |b| {
        b.iter(|| collect(ability_detail::player_games::plan(source, tracked)))
    });
    group.finish();
}

fn bench_snapshot_stats(c: &mut Criterion, source: &dyn SnapshotSource) {
    use swarmy_tauri_lib::snapshot_stats;
    let mut group = group(c, "snapshot_stats");
    group.bench_function(BenchmarkId::new("summary", "all"), |b| {
        b.iter(|| collect(snapshot_stats::summary::plan(source)))
    });
    group.finish();
}

fn bench_ability_usage(c: &mut Criterion, source: &dyn SnapshotSource) {
    use swarmy_tauri_lib::ability_usage;
    let tracked = ability_usage::game_uses::tracked_abilities();
    let mut group = group(c, "ability_usage");
    group.bench_function(BenchmarkId::new("game_uses", "all"), |b| {
        b.iter(|| collect(ability_usage::game_uses::plan(source, tracked.clone())))
    });
    group.bench_function(BenchmarkId::new("totals", "all"), |b| {
        b.iter(|| collect(ability_usage::totals::plan(source, tracked.clone())))
    });
    group.bench_function(BenchmarkId::new("by_player", "all"), |b| {
        b.iter(|| collect(ability_usage::by_player::plan(source, tracked.clone())))
    });
    group.finish();
}

fn bench_replay_detail(c: &mut Criterion, source: &dyn SnapshotSource) {
    use swarmy_tauri_lib::replay_detail::{
        bases, death_density, deaths, epicenters, opening_answers, patch_timings, winner_benchmarks,
    };
    // The version with the most games.
    let versions = collect(swarmy_tauri_lib::replay_list::protocol_versions::plan(
        source,
    ));
    let version = versions
        .column("ext_protocol_version")
        .and_then(|column| Ok(column.u32()?.get(0).unwrap_or_default()))
        .expect("a protocol version in the snapshot");
    let games = patch_timings::games::shape(collect(patch_timings::games::plan(source, version)))
        .expect("the games of the version");
    let tech = opening_answers::answers::tech_structures().expect("the balance data");
    // The newest game of the snapshot for the queries of one replay, with an engagement every
    // minute of its first half hour: more than a real game has.
    let newest = collect(
        source
            .table(swarmy_common::DETAILS_IPC)
            .map(|details| details.select([col("ext_fs_id").max()])),
    );
    let ext_fs_id = newest
        .column("ext_fs_id")
        .and_then(|column| Ok(column.u64()?.get(0).unwrap_or_default()))
        .expect("a game in the snapshot");
    let engagements: Vec<swarmy_common::Engagement> = (0..30)
        .map(|minute| swarmy_common::Engagement {
            start_seconds: minute * 60,
            end_seconds: minute * 60 + 59,
            ..Default::default()
        })
        .collect();
    let windows = epicenters::engagement_windows(&engagements).expect("the windows");
    let valued =
        swarmy_tauri_lib::replay_detail::valued_units::valued_units().expect("the balance data");
    let mut group = group(c, "replay_detail");
    group.bench_function(BenchmarkId::new("bases", ext_fs_id), |b| {
        b.iter(|| collect(bases::plan(source, ext_fs_id)))
    });
    group.bench_function(BenchmarkId::new("deaths", ext_fs_id), |b| {
        b.iter(|| collect(deaths::plan(source, ext_fs_id, valued.clone())))
    });
    // A large playable area, the heaviest grid.
    let frame = death_density::Frame {
        left: 0.,
        bottom: 0.,
        right: 200.,
        top: 200.,
    };
    let offsets = death_density::kernel_offsets().expect("the offsets");
    group.bench_function(BenchmarkId::new("death_density", ext_fs_id), |b| {
        b.iter(|| {
            collect(death_density::plan(
                source,
                ext_fs_id,
                valued.clone(),
                offsets.clone(),
                frame,
            ))
        })
    });
    group.bench_function(BenchmarkId::new("epicenters", ext_fs_id), |b| {
        b.iter(|| {
            collect(epicenters::plan(
                source,
                ext_fs_id,
                windows.clone(),
                valued.clone(),
            ))
        })
    });
    group.bench_function(BenchmarkId::new("games", version), |b| {
        b.iter(|| collect(patch_timings::games::plan(source, version)))
    });
    group.bench_function(BenchmarkId::new("upgrade_averages", version), |b| {
        b.iter(|| collect(patch_timings::upgrade_averages::plan(source, &games)))
    });
    group.bench_function(BenchmarkId::new("expansion_averages", version), |b| {
        b.iter(|| collect(patch_timings::expansion_averages::plan(source, &games)))
    });
    group.bench_function(BenchmarkId::new("medians", version), |b| {
        b.iter(|| collect(winner_benchmarks::medians::plan(source, version)))
    });
    group.bench_function(BenchmarkId::new("answers", "all"), |b| {
        b.iter(|| collect(opening_answers::answers::plan(source, tech.clone())))
    });
    group.finish();
}

fn main() {
    let Ok(replays) = std::env::var("SWARMY_TEST_REPLAYS") else {
        eprintln!(
            "queries: skipped, SWARMY_TEST_REPLAYS (a Replay Path with a snapshot) is not set"
        );
        return;
    };
    let source = IpcSnapshot::open(&replays).expect("the snapshot of SWARMY_TEST_REPLAYS");
    // The benches run without a personal database: no accounts grouped.
    let groups = HashMap::new();
    let (map, player) = parameters(&source, &groups);
    eprintln!("queries: {replays}, map {map:?}, player {player:?}");
    let mut c = Criterion::default().configure_from_args();
    bench_map_details(&mut c, &source, &map);
    bench_player_detail(&mut c, &source, &groups, &player);
    bench_replay_list(&mut c, &source, &groups, &player);
    bench_progress(&mut c, &source, &player);
    bench_race_detail(&mut c, &source);
    bench_map_stats(&mut c, &source, &player);
    bench_usage_over_time(&mut c, &source);
    bench_unit_stats(&mut c, &source);
    bench_ability_detail(&mut c, &source);
    bench_snapshot_stats(&mut c, &source);
    bench_ability_usage(&mut c, &source);
    bench_replay_detail(&mut c, &source);
    c.final_summary();
}
