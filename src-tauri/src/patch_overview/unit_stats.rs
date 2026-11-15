//! How a unit was used in the games of the snapshot: the units made, died and their kills, by
//! month, and the games in which the unit was exceptionally effective. Each statistic is a query
//! file of this directory (AGENTS.md, "A query is a file"); this file runs them.

pub mod by_month;
pub mod exceptional;
pub mod forms;
pub mod games;
pub mod kills_per_unit;
pub mod totals;

use crate::data::{IpcSnapshot, QueryMeta};
use crate::replay_rerun::decisions::{base_unit_name, latest_balance_units, unit_cost};
use swarmy_common::*;

/// The queries of the panel, for the debug links. The morphs (`forms`) are not one: they come
/// from the static table of each patch, `forms/unit_forms.json`.
pub const QUERIES: [QueryMeta; 5] = [
    games::META,
    totals::META,
    by_month::META,
    kills_per_unit::META,
    exceptional::META,
];

/// The stats of a form in the latest balance data, for the comparison of the forms; a form the
/// balance data does not know has its name only.
pub fn unit_form(name: &str) -> UnitForm {
    let Ok(balance) = latest_balance_units() else {
        return UnitForm {
            name: name.to_string(),
            ..Default::default()
        };
    };
    let Some(unit) = balance.get(name) else {
        return UnitForm {
            name: name.to_string(),
            ..Default::default()
        };
    };
    let number = |value: f32| {
        let text = format!("{value:.3}");
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    UnitForm {
        name: name.to_string(),
        life: unit.life.max.to_string(),
        armor: unit.armor.start.to_string(),
        speed: unit
            .movement
            .as_ref()
            .map(|movement| number(movement.speed))
            .unwrap_or_default(),
        weapons: unit
            .weapons
            .inner
            .iter()
            .map(
                |weapon| match weapon.misc.as_ref().and_then(|misc| misc.range) {
                    Some(range) => format!("{} ({})", weapon.id, number(range)),
                    None => weapon.id.clone(),
                },
            )
            .collect::<Vec<_>>()
            .join(", "),
        abilities: unit
            .abilities
            .inner
            .iter()
            .filter(|ability| {
                ability
                    .commands
                    .iter()
                    .any(|command| !command.meta.name.is_empty())
            })
            .map(|ability| ability.id.clone())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// The unit names that are the same unit, i.e. `RoachBurrowed` is a Roach.
pub fn unit_variants(unit_name: &str) -> Vec<String> {
    const VARIANTS: [&str; 7] = [
        "SiegeTankSieged",
        "VikingAssault",
        "LiberatorAG",
        "ThorAP",
        "WarpPrismPhasing",
        "ObserverSiegeMode",
        "OverseerSiegeMode",
    ];
    let mut names = vec![unit_name.to_string(), format!("{unit_name}Burrowed")];
    names.extend(
        VARIANTS
            .iter()
            .filter(|variant| base_unit_name(variant) == unit_name)
            .map(|variant| variant.to_string()),
    );
    names
}

pub fn try_query_unit_snapshot_stats(
    replay_path: &str,
    unit_name: &str,
) -> Result<UnitSnapshotStats, SwarmyError> {
    let source = IpcSnapshot::open(replay_path)?;
    let cost = unit_cost(latest_balance_units()?, unit_name);
    let unit = base_unit_name(unit_name);
    let totals = totals::shape(totals::plan(&source, unit)?.collect()?)?;
    let by_month = by_month::shape(by_month::plan(&source, unit)?.collect()?)?;
    let compared = kills_per_unit::plan(&source, unit, EXCEPTIONAL_MIN_BORN)?.collect()?;
    let threshold = kills_per_unit::threshold(&compared)?;
    let exceptional = exceptional::shape(
        exceptional::plan(&source, unit, EXCEPTIONAL_MIN_BORN, threshold.threshold)?.collect()?,
    )?;
    // The forms of the unit in the newest patch (the static table of the morphs), the unit itself
    // first then by the changes.
    let form_changes = forms::forms_of(unit_name, forms::latest_forms()?);
    let mut form_names: Vec<String> = vec![unit_name.to_string()];
    for change in &form_changes {
        for name in [&change.from, &change.to] {
            if !form_names.contains(name) {
                form_names.push(name.clone());
            }
        }
    }
    let forms = form_names.iter().map(|name| unit_form(name)).collect();
    Ok(UnitSnapshotStats {
        unit_name: unit_name.to_string(),
        games: totals.games,
        games_with_unit: totals.games_with_unit,
        born: totals.born,
        died: totals.died,
        kills: totals.kills,
        value_killed: totals.value_killed,
        unit_cost: cost.minerals + cost.vespene,
        by_month,
        kills_per_unit_mean: threshold.mean,
        kills_per_unit_threshold: threshold.threshold,
        exceptional,
        form_changes,
        forms,
    })
}

/// The games the tests of the queries of this panel share.
#[cfg(test)]
pub mod test_games {
    use crate::data::fixtures::{Born, Game, Player, Snapshot, Unit};

    /// Two games for the forms of the units: in game 1 Bob's Roach (tag 50) burrows at 150 s and
    /// unburrows at 200 s while Alice's Siege Tank (tag 60) sieges at 160 s; in game 2 Carl's
    /// Roach (tag 50) burrows at 150 s.
    pub fn forms() -> Snapshot {
        Snapshot::new()
            .game(Game {
                id: 1,
                players: vec![
                    Player {
                        born: vec![
                            Born::new(100, "SiegeTank", 60),
                            Born::new(160, "SiegeTankSieged", 60),
                        ],
                        ..Player::won("Alice", "Terran")
                    },
                    Player {
                        born: vec![
                            Born::new(100, "Roach", 50),
                            Born::new(150, "RoachBurrowed", 50),
                            Born::new(200, "Roach", 50),
                        ],
                        ..Player::lost("Bob", "Zerg")
                    },
                ],
                ..Default::default()
            })
            .game(Game {
                id: 2,
                players: vec![
                    Player::won("Alice", "Terran"),
                    Player {
                        born: vec![
                            Born::new(100, "Roach", 50),
                            Born::new(150, "RoachBurrowed", 50),
                        ],
                        ..Player::lost("Carl", "Zerg")
                    },
                ],
                ..Default::default()
            })
    }

    /// Eight 1v1 games on Pylon LE, each Terran player against Zed (Zerg). Six Terran players
    /// made 10 Marines each in January: Alice, Bea, Cleo, Dora and Eve killed 5 with them (0.5
    /// per Marine; Alice lost 3 of hers), Fiona killed 50 (5.0 per Marine, the exceptional game).
    /// In February Gina made 2 Marines and killed 20 with them (too few to compare) and game 8
    /// is Zerg against Protoss, without Marines.
    ///
    /// | Game | Day | Terran | Marines: born, kills, value killed |
    /// |---|---|---|---|
    /// | 1 | 2026-01-10 | Alice | 10, 5, 250 (3 died) |
    /// | 2 | 2026-01-11 | Bea | 10, 5, 250 |
    /// | 3 | 2026-01-12 | Cleo | 10, 5, 250 |
    /// | 4 | 2026-01-13 | Dora | 10, 5, 250 |
    /// | 5 | 2026-01-14 | Eve | 10, 5, 250 |
    /// | 6 | 2026-01-20 | Fiona | 10, 50, 2500 |
    /// | 7 | 2026-02-05 | Gina | 2, 20, 1000 |
    /// | 8 | 2026-02-06 | Zed (Zerg) beats Pat (Protoss) | none |
    pub fn marine_makers() -> Snapshot {
        let marines = |name: &'static str, day: &'static str, id: u64, unit: Unit| Game {
            id,
            played_at: day,
            players: vec![
                Player {
                    units: vec![unit],
                    ..Player::won(name, "Terran")
                },
                Player::lost("Zed", "Zerg"),
            ],
            ..Default::default()
        };
        let marine = |born: u32, kills: u32, value_killed: u64| {
            Unit::new("Marine", born, kills, value_killed)
        };
        Snapshot::new()
            .game(marines(
                "Alice",
                "2026-01-10 20:30:00",
                1,
                Unit {
                    died: 3,
                    ..marine(10, 5, 250)
                },
            ))
            .game(marines("Bea", "2026-01-11 20:30:00", 2, marine(10, 5, 250)))
            .game(marines(
                "Cleo",
                "2026-01-12 20:30:00",
                3,
                marine(10, 5, 250),
            ))
            .game(marines(
                "Dora",
                "2026-01-13 20:30:00",
                4,
                marine(10, 5, 250),
            ))
            .game(marines("Eve", "2026-01-14 20:30:00", 5, marine(10, 5, 250)))
            .game(marines(
                "Fiona",
                "2026-01-20 20:30:00",
                6,
                marine(10, 50, 2_500),
            ))
            .game(marines(
                "Gina",
                "2026-02-05 20:30:00",
                7,
                marine(2, 20, 1_000),
            ))
            .game(Game {
                id: 8,
                played_at: "2026-02-06 20:30:00",
                players: vec![Player::won("Zed", "Zerg"), Player::lost("Pat", "Protoss")],
                ..Default::default()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_lists_the_unit_variants() {
        assert_eq!(unit_variants("Roach"), vec!["Roach", "RoachBurrowed"]);
        assert!(unit_variants("SiegeTank").contains(&"SiegeTankSieged".to_string()));
        assert!(unit_variants("VikingFighter").contains(&"VikingAssault".to_string()));
    }
}
