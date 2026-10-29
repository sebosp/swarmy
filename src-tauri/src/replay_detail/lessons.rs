//! What cost each player the most in a game, and how much of their casters' energy they used.
//!
//! Each lesson gets an estimate of the minerals and vespene it cost so that they can be ranked,
//! the estimates are rough on purpose, they compare a supply block with idle production or a lost
//! fight, they are not exact:
//!
//! - Supply blocked: the income of the seconds blocked, it could not be spent on units.
//! - Idle production: [`IDLE_PRODUCTION_COST_PER_SECOND`] for each second a production
//!   structure waited, about what a Barracks or a Gateway spends.
//! - Floating: the average unspent resources above [`FLOATING_THRESHOLD`].
//! - Missed injects, chrono boosts and MULEs: the uses the energy allowed and were not made, by
//!   what each use is worth ([`INJECT_VALUE`], [`CHRONO_VALUE`], [`MULE_VALUE`]).
//! - The worst fight: what the player lost more than the other side in the engagement.

use super::data::FLOATING_THRESHOLD;
use std::collections::BTreeMap;
use swarmy_common::*;

/// The energy regenerated per game second at Faster speed.
pub const ENERGY_PER_SECOND: f32 = 0.7875;

/// A hatchery can be injected again once the previous inject spawned, after 29 seconds.
pub const INJECT_SECONDS: f32 = 29.;

pub const IDLE_PRODUCTION_COST_PER_SECOND: f32 = 2.;

/// Three larva, often not all spent: about a drone.
pub const INJECT_VALUE: f32 = 50.;

/// 20 seconds of production at 50% faster, about 10 seconds of a structure.
pub const CHRONO_VALUE: f32 = 20.;

/// The minerals a MULE mines in its 64 seconds.
pub const MULE_VALUE: f32 = 225.;

/// The lessons that cost less than this are not shown.
pub const MIN_LESSON_COST: u32 = 100;

/// The uses of the abilities with energy compared to what the energy of the casters allowed.
pub fn ability_efficiency(
    caster_seconds: &BTreeMap<&'static str, f32>,
    macro_summary: &MacroSummary,
) -> Vec<AbilityEfficiency> {
    let seconds = |names: &[&str]| -> f32 {
        names
            .iter()
            .filter_map(|name| caster_seconds.get(name))
            .sum()
    };
    let mut efficiency = vec![];
    let queen_seconds = seconds(&["Queen"]);
    if queen_seconds > 0. {
        // The queens' energy, or the hatcheries that can be injected, whichever is less.
        let possible = (queen_seconds * ENERGY_PER_SECOND / 25.)
            .min(seconds(&["Hatchery", "Lair", "Hive"]) / INJECT_SECONDS);
        efficiency.push(AbilityEfficiency {
            ability: "Inject Larva".to_string(),
            caster: "Queen".to_string(),
            uses: macro_summary.injects,
            caster_minutes: queen_seconds / 60.,
            possible,
        });
    }
    for (ability, caster, uses) in [
        ("Chrono Boost", "Nexus", macro_summary.chrono_boosts),
        ("MULE", "OrbitalCommand", macro_summary.mules),
    ] {
        let caster_seconds = seconds(&[caster]);
        if caster_seconds > 0. {
            efficiency.push(AbilityEfficiency {
                ability: ability.to_string(),
                caster: caster.to_string(),
                uses,
                caster_minutes: caster_seconds / 60.,
                possible: caster_seconds * ENERGY_PER_SECOND / 50.,
            });
        }
    }
    efficiency
}

/// The most costly lessons of the player of the summary, at most [`MAX_LESSONS`].
pub fn lessons(summary: &ReplayPlayerSummary, engagements: &[Engagement]) -> Vec<Lesson> {
    let macro_summary = &summary.macro_summary;
    let income_per_second = macro_summary.avg_income as f32 / 60.;
    let mut lessons = vec![];
    if macro_summary.supply_blocked_seconds > 0 {
        lessons.push(Lesson {
            title: "Supply blocked".to_string(),
            detail: format!(
                "{} seconds unable to make units, build supply before you need it",
                macro_summary.supply_blocked_seconds
            ),
            cost: (macro_summary.supply_blocked_seconds as f32 * income_per_second) as u32,
            seconds: None,
        });
    }
    let idle_seconds: u32 = macro_summary
        .production_idle
        .iter()
        .map(|idle| idle.idle_seconds)
        .sum();
    if let Some(worst) = macro_summary
        .production_idle
        .iter()
        .max_by_key(|idle| idle.idle_seconds)
    {
        lessons.push(Lesson {
            title: "Idle production".to_string(),
            detail: format!(
                "Production waited {idle_seconds} seconds in all, the {} the most ({} seconds)",
                worst.structure, worst.idle_seconds
            ),
            cost: (idle_seconds as f32 * IDLE_PRODUCTION_COST_PER_SECOND) as u32,
            seconds: None,
        });
    }
    if macro_summary.avg_unspent > FLOATING_THRESHOLD as u32 {
        lessons.push(Lesson {
            title: "Floating resources".to_string(),
            detail: format!(
                "{} unspent on average, {} seconds above {FLOATING_THRESHOLD}: add production or expand",
                macro_summary.avg_unspent, macro_summary.floating_seconds
            ),
            cost: macro_summary.avg_unspent - FLOATING_THRESHOLD as u32,
            seconds: None,
        });
    }
    for efficiency in &summary.ability_efficiency {
        let missed = (efficiency.possible - efficiency.uses as f32).max(0.);
        let value = match efficiency.ability.as_str() {
            "Inject Larva" => INJECT_VALUE,
            "Chrono Boost" => CHRONO_VALUE,
            _ => MULE_VALUE,
        };
        if missed >= 1. {
            lessons.push(Lesson {
                title: format!("Missed {}", efficiency.ability),
                detail: format!(
                    "{} of about {:.0} possible with the energy of the {} ({:.0}%)",
                    efficiency.uses,
                    efficiency.possible,
                    efficiency.caster,
                    100. * efficiency.uses as f32 / efficiency.possible.max(1.)
                ),
                cost: (missed * value) as u32,
                seconds: None,
            });
        }
    }
    if let Some((engagement, lost, traded)) = worst_engagement(&summary.player.name, engagements) {
        lessons.push(Lesson {
            title: "Lost fight".to_string(),
            detail: format!(
                "Lost {lost} resources and killed {} at {}, fight where your units counter theirs or wait",
                lost - traded,
                game_time(engagement.start_seconds)
            ),
            cost: traded,
            seconds: Some(engagement.start_seconds),
        });
    }
    lessons.retain(|lesson| lesson.cost >= MIN_LESSON_COST);
    lessons.sort_by_key(|lesson| std::cmp::Reverse(lesson.cost));
    lessons.truncate(MAX_LESSONS);
    lessons
}

/// `mm:ss`, as the views show the times.
fn game_time(seconds: u32) -> String {
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// The engagement in which the player lost the most more than the enemies: the engagement, what
/// the player lost and how much more than the enemies.
fn worst_engagement<'a>(
    player: &str,
    engagements: &'a [Engagement],
) -> Option<(&'a Engagement, u32, u32)> {
    engagements
        .iter()
        .filter_map(|engagement| {
            let lost = |own: bool| -> u32 {
                engagement
                    .losses
                    .iter()
                    .filter(|loss| (loss.player == player) == own)
                    .map(|loss| loss.resources.minerals + loss.resources.vespene)
                    .sum()
            };
            let (own, enemies) = (lost(true), lost(false));
            (own > enemies).then(|| (engagement, own, own - enemies))
        })
        .max_by_key(|(_, _, traded)| *traded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_ranks_the_lessons_by_cost() {
        let mut summary = ReplayPlayerSummary::default();
        summary.player.name = "Sazed".to_string();
        summary.macro_summary = MacroSummary {
            supply_blocked_seconds: 60,
            avg_income: 1200,
            avg_unspent: 1500,
            production_idle: vec![ProductionIdle {
                structure: "Barracks".to_string(),
                structures: 2,
                idle_seconds: 30,
            }],
            ..Default::default()
        };
        let lessons = lessons(&summary, &[]);
        let titles: Vec<&str> = lessons.iter().map(|lesson| lesson.title.as_str()).collect();
        // 60 s of 20 per second, 500 above the threshold, 30 s of idle production is too little.
        assert_eq!(titles, vec!["Supply blocked", "Floating resources"]);
        assert_eq!(lessons[0].cost, 1200);
    }

    #[test]
    fn it_compares_the_injects_with_the_queens_energy() {
        let casters = BTreeMap::from([("Queen", 600.), ("Hatchery", 1200.)]);
        let summary = MacroSummary {
            injects: 10,
            ..Default::default()
        };
        let efficiency = ability_efficiency(&casters, &summary);
        // 600 s of queen energy allow 18.9 injects, the hatcheries 41.
        assert!((efficiency[0].possible - 18.9).abs() < 0.01);
        assert_eq!(efficiency[0].uses, 10);
    }
}

/// The lessons, ability efficiency and control of the replays of a directory:
/// SWARMY_TEST_REPLAYS=~/SC2Replays/PRO cargo test --release lessons_of_replays -- --ignored --nocapture
#[test]
#[ignore]
fn lessons_of_replays() {
    let dir = std::env::var("SWARMY_TEST_REPLAYS").unwrap();
    let files = s2protocol::get_matching_files(std::path::PathBuf::from(dir), 8, 8).unwrap();
    for file in files {
        let Ok(detail) = super::data::analyze_replay_file(&file.display().to_string()) else {
            continue;
        };
        println!(
            "== {} {} {}s",
            detail.protocol_version, detail.map_title, detail.duration_seconds
        );
        for player in &detail.players {
            println!(
                "  {} {} {:?}",
                player.player.name, player.player.race, player.control
            );
            for efficiency in &player.ability_efficiency {
                println!(
                    "    {} {} of {:.1} ({:.1} caster minutes)",
                    efficiency.ability,
                    efficiency.uses,
                    efficiency.possible,
                    efficiency.caster_minutes
                );
            }
            for lesson in &player.lessons {
                println!(
                    "    lesson {} ({}): {}",
                    lesson.title, lesson.cost, lesson.detail
                );
            }
        }
    }
}
