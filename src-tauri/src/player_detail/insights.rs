//! Statistics of a player from the analysis of each of its games, compared to the snapshot.

use super::data::add_game;
use crate::data::*;
use crate::replay_detail::data::BENCHMARK_MINUTES;
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use swarmy_common::*;

/// The games shown in the recent form.
pub const RECENT_FORM_GAMES: usize = 10;

/// The stats are sampled every 160 game loops (~7 seconds), the first sample after the minute.

/// The average economy at a minute, i.e. of a matchup in the snapshot.
#[derive(Debug, Default, Clone)]
struct BenchmarkAverage {
    games: usize,
    workers: f32,
    supply: f32,
    income: f32,
    army_value: f32,
}

impl BenchmarkAverage {
    fn add(&mut self, workers: i32, supply: i32, income: i32, army_value: i32) {
        // Running averages, to avoid keeping the samples around.
        self.games += 1;
        let weight = 1. / self.games as f32;
        self.workers += (workers as f32 - self.workers) * weight;
        self.supply += (supply as f32 - self.supply) * weight;
        self.income += (income as f32 - self.income) * weight;
        self.army_value += (army_value as f32 - self.army_value) * weight;
    }
}

type BenchmarkTable = HashMap<(String, u32), BenchmarkAverage>;

/// The average economy per matchup and minute of every game in the snapshot, it does not change
/// until the snapshot is optimized again.
fn snapshot_benchmarks(ipc_path: &str) -> Result<Arc<BenchmarkTable>, SwarmyError> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<BenchmarkTable>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(table) = cache
        .lock()
        .ok()
        .and_then(|cache| cache.get(ipc_path).cloned())
    {
        return Ok(table);
    }
    let table = compute_benchmarks(ipc_path)?;
    let table = Arc::new(table);
    if let Ok(mut cache) = cache.lock() {
        cache.insert(ipc_path.to_string(), table.clone());
    }
    Ok(table)
}

/// The average economy per matchup and minute of the 1v1 games of the snapshot: the first stats
/// sample after each minute of each player, averaged per matchup of the player.
fn compute_benchmarks(ipc_path: &str) -> Result<BenchmarkTable, SwarmyError> {
    let game = [col("ext_fs_id")];
    // The matchup of each player of the 1v1 games, its race against the race of the other one.
    let matchups = scan_snapshot_ipc(ipc_path, DETAILS_IPC)?
        .select([
            col("ext_fs_id"),
            col("player_name"),
            race_letter_expr("player_race").alias("race"),
        ])
        .filter(len().over(game.clone())?.eq(lit(2)))
        .with_column(
            concat_str(
                [
                    col("race"),
                    lit("v"),
                    col("race").reverse().over(game.clone())?,
                ],
                "",
                false,
            )
            .alias("matchup"),
        );
    // The stats are by tracker player id.
    let player_matchups = scan_snapshot_ipc(ipc_path, GAME_PLAYERS_IPC)?
        .select([col("ext_fs_id"), col("player_name"), col("player_id")])
        .join(
            matchups,
            [col("ext_fs_id"), col("player_name")],
            [col("ext_fs_id"), col("player_name")],
            JoinArgs::new(JoinType::Inner),
        )
        .select([col("ext_fs_id"), col("player_id"), col("matchup")])
        .unique(None, UniqueKeepStrategy::First);
    let measures = ["workers", "supply", "income", "army_value"];
    let averages = scan_snapshot_ipc(ipc_path, GAME_MINUTES_IPC)?
        .filter(col("minute").is_in(
            lit(Series::new("minutes".into(), BENCHMARK_MINUTES.as_slice())).implode(false),
            false,
        ))
        .with_column((col("minerals_rate") + col("vespene_rate")).alias("income"))
        .join(
            player_matchups,
            [col("ext_fs_id"), col("player_id")],
            [col("ext_fs_id"), col("player_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("matchup"), col("minute")])
        .agg(
            std::iter::once(len().alias("games"))
                .chain(measures.map(|measure| col(measure).cast(DataType::Float32).mean()))
                .collect::<Vec<_>>(),
        )
        .collect()?;
    let matchups = str_values(&averages, "matchup")?;
    let minutes = u32_values(&averages, "minute")?;
    let games = averages.column("games")?.cast(&DataType::UInt64)?;
    let games = games.u64()?;
    let [workers, supply, income, army_value] =
        measures.map(|measure| f32_values(&averages, measure).unwrap_or_default());
    Ok(matchups
        .into_iter()
        .zip(minutes)
        .enumerate()
        .map(|(idx, key)| {
            let average = BenchmarkAverage {
                games: games.get(idx).unwrap_or_default() as usize,
                workers: workers[idx],
                supply: supply[idx],
                income: income[idx],
                army_value: army_value[idx],
            };
            (key, average)
        })
        .collect())
}
/// The share of each choice in each context, most chosen first.
fn choice_shares(choices: Vec<(String, String, bool)>) -> Vec<ChoiceShare> {
    let mut context_games: HashMap<String, usize> = HashMap::new();
    let mut shares: BTreeMap<(String, String), ChoiceShare> = BTreeMap::new();
    for (context, choice, won) in choices {
        *context_games.entry(context.clone()).or_default() += 1;
        let share = shares
            .entry((context.clone(), choice.clone()))
            .or_insert_with(|| ChoiceShare {
                context,
                choice,
                ..Default::default()
            });
        share.games += 1;
        share.wins += won as usize;
    }
    let mut shares: Vec<ChoiceShare> = shares
        .into_values()
        .map(|mut share| {
            share.context_games = context_games[&share.context];
            share
        })
        .collect();
    shares.sort_by(|a, b| a.context.cmp(&b.context).then(b.games.cmp(&a.games)));
    shares
}

fn sorted_counts(counts: BTreeMap<String, usize>) -> Vec<UnitCount> {
    let mut counts: Vec<UnitCount> = counts
        .into_iter()
        .map(|(unit, count)| UnitCount { unit, count })
        .collect();
    counts.sort_by(|a, b| b.count.cmp(&a.count).then(a.unit.cmp(&b.unit)));
    counts
}

fn average(values: impl IntoIterator<Item = f32>) -> Option<f32> {
    let (sum, count) = values
        .into_iter()
        .fold((0., 0), |(sum, count), value| (sum + value, count + 1));
    (count > 0).then(|| sum / count as f32)
}

fn length_bucket(seconds: u32) -> &'static str {
    match seconds / 60 {
        0..10 => "1. Under 10 minutes",
        10..15 => "2. 10 to 15 minutes",
        15..20 => "3. 15 to 20 minutes",
        _ => "4. Over 20 minutes",
    }
}

/// Adds up the insights of the games of the player, computed from the snapshot by Optimize.
pub fn player_insights(
    ipc_path: &str,
    player_name: &str,
    matches: &[PlayerMatch],
) -> Result<PlayerInsights, SwarmyError> {
    // A game against several opponents is a single game.
    let mut seen = std::collections::HashSet::new();
    let games: Vec<&PlayerMatch> = matches
        .iter()
        .filter(|game| seen.insert(game.ext_fs_id))
        .collect();
    // The games computed by Optimize from the snapshot; the games added since the last Optimize
    // are not counted until the next one, the replays are not read.
    let stored = super::game_insights::read_game_insights(ipc_path, &games)?;
    let name_of = |game: &PlayerMatch| {
        if game.played_as.is_empty() {
            player_name.to_string()
        } else {
            game.played_as.clone()
        }
    };
    let analyzed: Vec<(&PlayerMatch, PlayerGameInsight)> = games
        .iter()
        .filter_map(|game| {
            let insight = stored.get(&(game.ext_fs_id, name_of(game)))?;
            Some((*game, insight.clone()))
        })
        .collect();
    let snapshot = snapshot_benchmarks(ipc_path)?;
    let mut insights = PlayerInsights {
        analyzed_games: analyzed.len(),
        recent_form: games
            .iter()
            .take(RECENT_FORM_GAMES)
            .map(|game| game.won)
            .collect(),
        ..Default::default()
    };
    let mut by_matchup = BTreeMap::new();
    let mut by_map = BTreeMap::new();
    let mut by_length = BTreeMap::new();
    let mut openings_by_matchup = vec![];
    let mut openings_by_map = vec![];
    let mut first_army_units = vec![];
    let mut favourite_units = vec![];
    let mut benchmarks: BTreeMap<(String, u32), BenchmarkAverage> = BTreeMap::new();
    let mut production_idle: BTreeMap<String, u32> = BTreeMap::new();
    let mut first_scout_units = BTreeMap::new();
    let mut upgrade_priorities = BTreeMap::new();
    let mut killed_by = BTreeMap::new();
    let mut army_at_10: BTreeMap<String, usize> = BTreeMap::new();
    let mut summaries = vec![];
    for (game, insight) in &analyzed {
        let won = insight.won;
        let matchup = insight.matchup.clone();
        add_game(&mut by_matchup, matchup.clone(), won);
        add_game(&mut by_map, insight.map_title.clone(), won);
        add_game(
            &mut by_length,
            length_bucket(insight.duration_seconds).to_string(),
            won,
        );
        if !insight.opening.is_empty() {
            let opening = insight.opening.join(" → ");
            openings_by_matchup.push((matchup.clone(), opening.clone(), won));
            openings_by_map.push((insight.map_title.clone(), opening, won));
        }
        if let Some(unit) = &insight.first_army_unit {
            first_army_units.push((matchup.clone(), unit.clone(), won));
        }
        if let Some(unit) = &insight.favourite_army_unit {
            favourite_units.push((insight.map_title.clone(), unit.clone(), won));
        }
        for benchmark in &insight.benchmarks {
            benchmarks
                .entry((matchup.clone(), benchmark.minute))
                .or_default()
                .add(
                    benchmark.workers,
                    benchmark.supply,
                    benchmark.minerals_rate + benchmark.vespene_rate,
                    benchmark.army_value,
                );
        }
        for idle in &insight.production_idle {
            *production_idle.entry(idle.structure.clone()).or_default() += idle.idle_seconds;
        }
        if let Some(unit) = &insight.first_scout_unit {
            *first_scout_units.entry(unit.clone()).or_default() += 1;
        }
        if let Some(upgrade) = &insight.first_upgrade {
            *upgrade_priorities.entry(upgrade.clone()).or_default() += 1;
        }
        for unit in &insight.killed_by {
            *killed_by.entry(unit.unit.clone()).or_default() += unit.count;
        }
        for unit in &insight.army_at_10 {
            *army_at_10.entry(unit.unit.clone()).or_default() += unit.count;
        }
        if insight.was_behind {
            insights.behind_games += 1;
            insights.comeback_wins += won as usize;
        }
        insights.proxy_games += insight.proxy as usize;
        insights.base_trade_games += insight.base_trade as usize;
        if let Some(rating) = insight.rating {
            insights.ratings.push(RatingPoint {
                date: game.date.clone(),
                rating,
            });
        }
        summaries.push(insight);
    }
    let games_analyzed = summaries.len().max(1) as f32;
    let avg = |value: &dyn Fn(&PlayerGameInsight) -> f32| {
        average(summaries.iter().map(|summary| value(summary))).unwrap_or_default()
    };
    insights.avg_supply_blocked_seconds = avg(&|s| s.supply_blocked_seconds);
    insights.avg_floating_seconds = avg(&|s| s.floating_seconds);
    insights.avg_unspent = avg(&|s| s.avg_unspent);
    insights.avg_injects = avg(&|s| s.injects);
    insights.avg_chrono_boosts = avg(&|s| s.chrono_boosts);
    insights.avg_mules = avg(&|s| s.mules);
    insights.avg_commands_per_minute = avg(&|s| s.commands_per_minute);
    insights.avg_resources_lost = avg(&|s| s.resources_lost);
    insights.avg_resources_killed = avg(&|s| s.resources_killed);
    insights.avg_mining_seconds_lost = avg(&|s| s.mining_seconds_lost);
    insights.avg_bases_at_10 = avg(&|s| s.bases_at_10);
    insights.avg_first_kill_seconds = average(
        summaries
            .iter()
            .filter_map(|summary| summary.first_kill_seconds)
            .map(|seconds| seconds as f32),
    );
    insights.avg_first_scout_seconds = average(
        summaries
            .iter()
            .filter_map(|summary| summary.first_scout_seconds)
            .map(|seconds| seconds as f32),
    );
    insights.production_idle = production_idle
        .into_iter()
        .map(|(structure, idle_seconds)| ProductionIdle {
            structure,
            structures: 0,
            idle_seconds: (idle_seconds as f32 / games_analyzed) as u32,
        })
        .collect();
    let mut army_at_10: Vec<UnitAverage> = army_at_10
        .into_iter()
        .map(|(unit, count)| UnitAverage {
            unit,
            average: count as f32 / games_analyzed,
        })
        .collect();
    army_at_10.sort_by(|a, b| b.average.total_cmp(&a.average));
    insights.army_at_10 = army_at_10;
    insights.benchmarks = benchmarks
        .into_iter()
        .map(|((matchup, minute), player)| {
            let snapshot = snapshot
                .get(&(matchup.clone(), minute))
                .cloned()
                .unwrap_or_default();
            BenchmarkComparison {
                matchup,
                minute,
                games: player.games,
                workers: player.workers,
                supply: player.supply,
                income: player.income,
                army_value: player.army_value,
                snapshot_games: snapshot.games,
                snapshot_workers: snapshot.workers,
                snapshot_supply: snapshot.supply,
                snapshot_income: snapshot.income,
                snapshot_army_value: snapshot.army_value,
            }
        })
        .collect();
    insights.by_matchup = by_matchup.into_values().collect();
    insights.by_map = by_map.into_values().collect();
    insights.by_length = by_length.into_values().collect();
    insights.openings_by_matchup = choice_shares(openings_by_matchup);
    insights.openings_by_map = choice_shares(openings_by_map);
    insights.first_army_units = choice_shares(first_army_units);
    insights.favourite_units_by_map = choice_shares(favourite_units);
    insights.first_scout_units = sorted_counts(first_scout_units);
    insights.upgrade_priorities = sorted_counts(upgrade_priorities);
    insights.killed_by = sorted_counts(killed_by);
    insights.ratings.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(insights)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_shares_choices_by_context() {
        let shares = choice_shares(vec![
            ("Killswitch".into(), "Tanks".into(), true),
            ("Killswitch".into(), "Tanks".into(), false),
            ("Killswitch".into(), "Bio".into(), true),
        ]);
        assert_eq!(shares[0].choice, "Tanks");
        assert_eq!(
            (shares[0].games, shares[0].context_games, shares[0].wins),
            (2, 3, 1)
        );
    }

    #[test]
    fn it_averages_benchmarks() {
        let mut average = BenchmarkAverage::default();
        average.add(10, 20, 100, 0);
        average.add(20, 40, 300, 0);
        assert_eq!(
            (average.games, average.workers, average.income),
            (2, 15., 200.)
        );
    }
}
