//! The chance of each team to win over the game, from its army, income, workers and trades.
//!
//! A logistic model fitted on 27,441 1v1 games of the snapshot of corvid-zt (patches 4.x to 5.0),
//! every 30 seconds from 02:00, with the winner as the outcome. Each feature compares the two
//! sides, so swapping them swaps the chances:
//!
//! - army: `ln((army 1 + 200) / (army 2 + 200))`, the minerals and vespene of the alive army.
//! - income: `ln((income 1 + 100) / (income 2 + 100))`, the collection rates per minute.
//! - workers: `ln((workers 1 + 5) / (workers 2 + 5))`, the workers mining.
//! - trades: `(killed 1 - killed 2) / (killed 1 + killed 2 + 1000)`, the resources killed.
//!
//! The chance is `1 / (1 + e^-(sum of coefficient * feature))`, the coefficients change with the
//! time of the game, i.e. the army counts little at 03:00 and the most at 15:00. On a fifth of the
//! games left out of the fit it predicts the winner at 69% of the moments, and the chances are
//! calibrated: of the moments given 80% to 90%, 88% were won.
//!
//! How the alive units counter each other (the strengths and weaknesses of the balance data, as
//! in [`crate::replay_rerun::decisions::score_unit_choice`]) was tried as a fifth feature and did
//! not change the predictions, it is not part of the model. The model sees the economy and the
//! armies, not the blunders: a player ahead loses the game by fighting badly.
//!
//! The team games use the sums of the players of each team, the model was fitted on 1v1.

use swarmy_common::{Engagement, TurningPoint, WinChancePoint};

/// The first point of the chart, before it the sides are even.
pub const WIN_CHANCE_START_SECONDS: u32 = 120;

/// The time between two points of the chart.
pub const WIN_CHANCE_INTERVAL_SECONDS: u32 = 30;

/// The coefficients of army, income, workers and trades, fitted for two minutes of game around
/// the minute, interpolated between them.
const COEFFICIENTS: [(f32, [f32; 4]); 12] = [
    (3., [0.231, 0.761, 0.454, 3.786]),
    (5., [0.224, 1.215, 0.738, 2.724]),
    (7., [0.501, 1.024, 1.202, 2.429]),
    (9., [1.017, 0.91, 1.704, 2.616]),
    (11., [1.546, 1.29, 1.829, 2.969]),
    (13., [1.738, 1.502, 1.684, 3.238]),
    (15., [1.878, 1.54, 1.59, 3.732]),
    (17., [1.855, 1.633, 1.443, 4.344]),
    (19., [1.692, 1.194, 1.461, 4.017]),
    (21., [1.892, 1.489, 1.104, 3.615]),
    (23., [2.047, 1.415, 1.198, 3.651]),
    // From 24:00 on.
    (25., [1.154, 0.731, 0.914, 2.923]),
];

/// The coefficients at the minute of the game.
fn coefficients(minute: f32) -> [f32; 4] {
    let first = COEFFICIENTS[0];
    let last = COEFFICIENTS[COEFFICIENTS.len() - 1];
    if minute <= first.0 {
        return first.1;
    }
    if minute >= last.0 {
        return last.1;
    }
    let after = COEFFICIENTS
        .iter()
        .position(|(at, _)| *at >= minute)
        .unwrap_or(COEFFICIENTS.len() - 1);
    let (from, low) = COEFFICIENTS[after - 1];
    let (to, high) = COEFFICIENTS[after];
    let weight = (minute - from) / (to - from);
    std::array::from_fn(|idx| low[idx] + (high[idx] - low[idx]) * weight)
}

/// The economy and army of a side at a moment, the sum of its players.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SideStats {
    /// The minerals and vespene of the alive army.
    pub army: i32,
    /// The minerals and vespene collected per minute.
    pub income: i32,
    pub workers: i32,
    /// The minerals and vespene of the enemy units and structures killed.
    pub killed: i32,
}

impl std::ops::Add for SideStats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            army: self.army + other.army,
            income: self.income + other.income,
            workers: self.workers + other.workers,
            killed: self.killed + other.killed,
        }
    }
}

fn log_ratio(first: i32, second: i32, offset: f32) -> f32 {
    ((first.max(0) as f32 + offset) / (second.max(0) as f32 + offset)).ln()
}

/// The chance of the first side to win at the game second.
pub fn win_chance(seconds: u32, first: &SideStats, second: &SideStats) -> WinChancePoint {
    // The fit labelled the stats of each 30 seconds with their start.
    let minute = seconds.saturating_sub(WIN_CHANCE_INTERVAL_SECONDS) as f32 / 60.;
    let [army, income, workers, trades] = coefficients(minute);
    let killed = (first.killed.max(0) + second.killed.max(0) + 1000) as f32;
    let point = WinChancePoint {
        seconds,
        chance: 0.,
        army: army * log_ratio(first.army, second.army, 200.),
        income: income * log_ratio(first.income, second.income, 100.),
        workers: workers * log_ratio(first.workers, second.workers, 5.),
        trades: trades * (first.killed.max(0) - second.killed.max(0)) as f32 / killed,
    };
    let log_odds = point.army + point.income + point.workers + point.trades;
    WinChancePoint {
        chance: 1. / (1. + (-log_odds).exp()),
        ..point
    }
}

/// The points compared for a turning point, a minute.
const TURNING_POINT_STEPS: usize = 2;

/// A change of the chance smaller than this is not a turning point.
pub const MIN_TURNING_POINT_CHANGE: f32 = 0.15;

pub const MAX_TURNING_POINTS: usize = 3;

/// The moments of a minute in which the chance of the first team changed the most, at most
/// [`MAX_TURNING_POINTS`] that do not overlap, with the biggest engagement of each.
pub fn turning_points(points: &[WinChancePoint], engagements: &[Engagement]) -> Vec<TurningPoint> {
    let mut changes: Vec<(usize, f32)> = (TURNING_POINT_STEPS..points.len())
        .map(|idx| {
            (
                idx,
                points[idx].chance - points[idx - TURNING_POINT_STEPS].chance,
            )
        })
        .filter(|(_, change)| change.abs() >= MIN_TURNING_POINT_CHANGE)
        .collect();
    changes.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    let mut chosen: Vec<(usize, f32)> = vec![];
    for (idx, change) in changes {
        if chosen.len() == MAX_TURNING_POINTS {
            break;
        }
        if chosen
            .iter()
            .all(|(other, _)| other.abs_diff(idx) > TURNING_POINT_STEPS)
        {
            chosen.push((idx, change));
        }
    }
    chosen.sort_by_key(|(idx, _)| *idx);
    chosen
        .into_iter()
        .map(|(idx, change)| {
            let start_seconds = points[idx - TURNING_POINT_STEPS].seconds;
            let end_seconds = points[idx].seconds;
            let engagement = engagements
                .iter()
                .filter(|engagement| {
                    engagement.end_seconds + WIN_CHANCE_INTERVAL_SECONDS >= start_seconds
                        && engagement.start_seconds <= end_seconds
                })
                .max_by_key(|engagement| {
                    engagement
                        .losses
                        .iter()
                        .map(|loss| loss.resources.minerals + loss.resources.vespene)
                        .sum::<u32>()
                })
                .map(|engagement| {
                    engagement
                        .losses
                        .iter()
                        .map(|loss| {
                            format!(
                                "{} lost {} ({})",
                                swarmy_common::clean_player_name_clan_escapes(&loss.player),
                                loss.resources.minerals + loss.resources.vespene,
                                loss.units
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                });
            TurningPoint {
                start_seconds,
                end_seconds,
                change,
                engagement,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(army: i32, income: i32, workers: i32, killed: i32) -> SideStats {
        SideStats {
            army,
            income,
            workers,
            killed,
        }
    }

    #[test]
    fn it_is_even_between_equal_sides() {
        let stats = side(3000, 2000, 50, 1500);
        let point = win_chance(600, &stats, &stats);
        assert!((point.chance - 0.5).abs() < 1e-6);
    }

    #[test]
    fn it_swaps_the_chances_with_the_sides() {
        let ahead = side(6000, 2500, 60, 4000);
        let behind = side(2500, 1800, 45, 1000);
        let first = win_chance(720, &ahead, &behind);
        let second = win_chance(720, &behind, &ahead);
        assert!(first.chance > 0.9, "{first:?}");
        assert!((first.chance + second.chance - 1.).abs() < 1e-5);
        assert!(first.army > 0. && first.trades > 0.);
    }

    #[test]
    fn it_weighs_the_army_more_later() {
        let ahead = side(2000, 1000, 30, 0);
        let behind = side(1000, 1000, 30, 0);
        assert!(win_chance(240, &ahead, &behind).army < win_chance(900, &ahead, &behind).army);
    }

    #[test]
    fn it_finds_the_turning_points() {
        let chances = [0.5, 0.5, 0.52, 0.5, 0.3, 0.28, 0.3, 0.32, 0.6, 0.62];
        let points: Vec<WinChancePoint> = chances
            .iter()
            .enumerate()
            .map(|(idx, chance)| WinChancePoint {
                seconds: 120 + 30 * idx as u32,
                chance: *chance,
                ..Default::default()
            })
            .collect();
        let turning = turning_points(&points, &[]);
        // The rise of 08:00 to 08:30 (+0.32) and the fall of 03:30 to 04:00 (-0.22).
        assert_eq!(turning.len(), 2);
        assert!((180..=210).contains(&turning[0].start_seconds));
        assert!(turning[0].change < -0.2);
        assert!(turning[1].change > 0.29);
    }

    #[test]
    fn it_interpolates_the_coefficients() {
        assert_eq!(coefficients(0.), COEFFICIENTS[0].1);
        assert_eq!(coefficients(40.), COEFFICIENTS[11].1);
        let between = coefficients(4.);
        assert!((between[0] - (0.231 + 0.224) / 2.).abs() < 1e-6);
    }

    /// The chances of the replays of a directory favor their winners at the end:
    /// SWARMY_TEST_REPLAYS=~/SC2Replays/PRO cargo test --release win_chance_of_replays -- --ignored --nocapture
    #[test]
    #[ignore]
    fn win_chance_of_replays() {
        let dir = std::env::var("SWARMY_TEST_REPLAYS").unwrap();
        let files = s2protocol::get_matching_files(std::path::PathBuf::from(dir), 60, 8).unwrap();
        let (mut games, mut favored, mut favored_at_half) = (0, 0, 0);
        for file in files {
            let Ok(detail) =
                crate::replay_detail::data::analyze_replay_file(&file.display().to_string())
            else {
                continue;
            };
            let Some(chances) = detail.win_chances else {
                continue;
            };
            let Some(last) = chances.points.last() else {
                continue;
            };
            let first_won = chances
                .first_team
                .iter()
                .any(|name| detail.winners.contains(name));
            let half = &chances.points[chances.points.len() / 2];
            games += 1;
            favored += ((last.chance > 0.5) == first_won) as usize;
            favored_at_half += ((half.chance > 0.5) == first_won) as usize;
            println!(
                "{:>5}s first won {first_won:<5} chance at half {:.2} at the end {:.2} {:?}",
                detail.duration_seconds, half.chance, last.chance, chances.first_team
            );
        }
        println!(
            "{games} games, the winner favored at the end {favored}, at half {favored_at_half}"
        );
        assert!(favored as f32 > games as f32 * 0.7);
    }
}
