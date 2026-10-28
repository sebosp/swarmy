//! The units of each patch from the s2protocol multi versioned balance data.

use crate::replay_rerun::decisions::{balance_units, has_cost};
use s2protocol::game_events::{UnitShortRef, VersionedBalanceUnit};
use std::collections::{BTreeMap, BTreeSet};
use swarmy_common::*;

/// Shown when the unit or the property is not in a patch.
const MISSING: &str = "-";

/// Shown when a unit is in a list of a patch, i.e. a unit is strong against it.
const PRESENT: &str = "✓";

/// The co-op and campaign variants that share the names of the melee units.
fn is_melee_unit(name: &str) -> bool {
    !name.contains("Gluescreen") && !name.ends_with("Dummy")
}

/// Terran, Zerg, Protoss or Other.
pub fn race_name(unit: &VersionedBalanceUnit) -> String {
    match unit.meta.race.as_deref() {
        Some("Terr") => "Terran",
        Some("Zerg") => "Zerg",
        Some("Prot") => "Protoss",
        _ => "Other",
    }
    .to_string()
}

/// Formats numbers without the trailing zeros, i.e. 25 instead of 25.0.
fn format_number(value: f32) -> String {
    let formatted = format!("{value:.3}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn unit_ids(units: &[UnitShortRef]) -> String {
    units
        .iter()
        .map(|unit| unit.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// How the value changed from the previous patch, numbers go up or down, lists of names gain or
/// lose items.
pub fn compare_values(previous: &str, value: &str) -> PatchChange {
    if previous == value {
        return PatchChange::Same;
    }
    match (previous, value) {
        (MISSING, _) => return PatchChange::Added,
        (_, MISSING) => return PatchChange::Removed,
        _ => {}
    }
    if let (Ok(previous), Ok(value)) = (previous.parse::<f32>(), value.parse::<f32>()) {
        return if value > previous {
            PatchChange::Increased
        } else {
            PatchChange::Decreased
        };
    }
    let items = |list: &str| -> BTreeSet<String> {
        list.split(", ")
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect()
    };
    let (previous, value) = (items(previous), items(value));
    match (value.is_superset(&previous), previous.is_superset(&value)) {
        (true, false) => PatchChange::Increased,
        (false, true) => PatchChange::Decreased,
        _ => PatchChange::Changed,
    }
}

/// Adds the change from the previous patch to each value.
fn with_changes(values: Vec<String>) -> Vec<PatchValue> {
    let mut previous: Option<String> = None;
    values
        .into_iter()
        .map(|value| {
            let change = match &previous {
                Some(previous) => compare_values(previous, &value),
                None => PatchChange::Same,
            };
            previous = Some(value.clone());
            PatchValue { value, change }
        })
        .collect()
}

pub fn try_list_patch_units() -> Result<PatchUnitList, SwarmyError> {
    let all_units = balance_units()?;
    let mut versions: Vec<u32> = all_units.keys().copied().collect();
    versions.sort();
    let mut units: BTreeSet<(String, String)> = BTreeSet::new();
    for version_units in all_units.values() {
        for (name, unit) in version_units {
            if is_melee_unit(name) && has_cost(version_units, name) {
                units.insert((race_name(unit), name.clone()));
            }
        }
    }
    Ok(PatchUnitList {
        versions,
        units: units
            .into_iter()
            .map(|(race, name)| PatchUnitName { name, race })
            .collect(),
    })
}

/// A unit and its properties in every patch, with the changes from the previous patch.
pub fn try_query_unit_overview(unit_name: String) -> Result<PatchUnit, SwarmyError> {
    use PatchRowKind::{Benefit, Cost, Neutral};
    let all_units = balance_units()?;
    let mut versions: Vec<u32> = all_units.keys().copied().collect();
    versions.sort();
    let units: Vec<Option<&VersionedBalanceUnit>> = versions
        .iter()
        .map(|version| all_units[version].get(&unit_name))
        .collect();
    let Some(any_unit) = units.iter().flatten().next() else {
        return Err(SwarmyError::Other(format!(
            "{unit_name} is not in the balance data"
        )));
    };
    let race = race_name(any_unit);
    let row =
        |label: &str,
         kind: PatchRowKind,
         value: &dyn Fn(&VersionedBalanceUnit) -> Option<String>| PatchUnitRow {
            label: label.to_string(),
            unit: None,
            kind,
            values: with_changes(
                units
                    .iter()
                    .map(|unit| unit.and_then(value).unwrap_or_else(|| MISSING.to_string()))
                    .collect(),
            ),
        };
    let mut rows = vec![
        row("Minerals", Cost, &|unit| {
            Some(unit.cost.as_ref()?.minerals?.to_string())
        }),
        row("Vespene", Cost, &|unit| {
            Some(unit.cost.as_ref()?.vespene?.to_string())
        }),
        row("Supply", Cost, &|unit| {
            Some(format_number(unit.cost.as_ref()?.supply?))
        }),
        row("Build time", Cost, &|unit| {
            // In game seconds at Faster speed, as shown in the game.
            let seconds = unit.cost.as_ref()?.time?
                / crate::replay_rerun::decisions::NORMAL_SECONDS_PER_FASTER_SECOND;
            Some(format_number((seconds * 10.).round() / 10.))
        }),
        row("Requires", Cost, &|unit| {
            Some(unit_ids(&unit.requires.inner))
        }),
        row("Life", Benefit, &|unit| Some(unit.life.max.to_string())),
        row("Shields", Benefit, &|unit| {
            Some(unit.shields.as_ref()?.max.to_string())
        }),
        row("Armor", Benefit, &|unit| Some(unit.armor.start.to_string())),
        row("Speed", Benefit, &|unit| {
            Some(format_number(unit.movement.as_ref()?.speed))
        }),
        row("Sight", Benefit, &|unit| {
            Some(format_number(unit.misc.sight_radius?))
        }),
        row("Attributes", Neutral, &|unit| {
            Some(
                unit.attributes
                    .inner
                    .iter()
                    .filter_map(|attribute| attribute.kind.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        }),
        row("Produced by", Neutral, &|unit| {
            Some(unit.producer.as_ref()?.id.clone())
        }),
    ];
    // One row per unit countered or countering in any patch, a list of names is too packed.
    let counters: [(
        &str,
        PatchRowKind,
        fn(&VersionedBalanceUnit) -> &[UnitShortRef],
    ); 2] = [
        ("Strong vs", Benefit, |unit| &unit.strengths.inner),
        ("Weak vs", Cost, |unit| &unit.weaknesses.inner),
    ];
    for (label, kind, list) in counters {
        let names: BTreeSet<String> = units
            .iter()
            .flatten()
            .flat_map(|unit| list(unit).iter().map(|counter| counter.id.clone()))
            .collect();
        for name in names {
            let mut counter_row = row(label, kind, &|unit| {
                Some(
                    if list(unit).iter().any(|counter| counter.id == name) {
                        PRESENT
                    } else {
                        MISSING
                    }
                    .to_string(),
                )
            });
            counter_row.unit = Some(name);
            rows.push(counter_row);
        }
    }
    // The weapons of every patch, a weapon may be added or removed by a patch.
    let weapons: BTreeMap<String, ()> = units
        .iter()
        .flatten()
        .flat_map(|unit| {
            unit.weapons
                .inner
                .iter()
                .map(|weapon| (weapon.id.clone(), ()))
        })
        .collect();
    for weapon_id in weapons.keys() {
        let weapon_misc = |unit: &VersionedBalanceUnit| {
            unit.weapons
                .inner
                .iter()
                .find(|weapon| &weapon.id == weapon_id)
                .and_then(|weapon| weapon.misc.clone())
        };
        rows.push(row(&format!("{weapon_id} range"), Benefit, &|unit| {
            Some(format_number(weapon_misc(unit)?.range?))
        }));
        // The time between attacks, more is slower.
        rows.push(row(&format!("{weapon_id} cooldown"), Cost, &|unit| {
            Some(format_number(weapon_misc(unit)?.speed?))
        }));
        rows.push(row(&format!("{weapon_id} targets"), Neutral, &|unit| {
            weapon_misc(unit)?.targets
        }));
    }
    Ok(PatchUnit {
        name: unit_name,
        race,
        versions,
        rows,
    })
}

/// The changes of every unit in a patch, compared to the previous patch.
pub fn try_query_patch_changes(version: u32) -> Result<PatchChanges, SwarmyError> {
    let list = try_list_patch_units()?;
    let Some(idx) = list
        .versions
        .iter()
        .position(|available| *available == version)
    else {
        return Err(SwarmyError::Other(format!(
            "Patch {version} is not in the balance data"
        )));
    };
    let previous_version = idx.checked_sub(1).map(|prev| list.versions[prev]);
    let mut changes = vec![];
    if previous_version.is_some() {
        for unit in &list.units {
            let overview = try_query_unit_overview(unit.name.clone())?;
            for row in overview.rows {
                let value = &row.values[idx];
                if value.change == PatchChange::Same {
                    continue;
                }
                let label = match &row.unit {
                    Some(counter) => format!("{} {counter}", row.label),
                    None => row.label,
                };
                changes.push(PatchUnitChange {
                    unit: unit.name.clone(),
                    race: unit.race.clone(),
                    label,
                    kind: row.kind,
                    previous: row.values[idx - 1].value.clone(),
                    value: value.value.clone(),
                    change: value.change,
                });
            }
        }
    }
    Ok(PatchChanges {
        version,
        previous_version,
        changes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_compares_values_between_patches() {
        assert_eq!(compare_values("50", "75"), PatchChange::Increased);
        assert_eq!(compare_values("2.25", "2"), PatchChange::Decreased);
        assert_eq!(compare_values("-", "50"), PatchChange::Added);
        assert_eq!(compare_values("50", "-"), PatchChange::Removed);
        assert_eq!(
            compare_values("Armory", "Armory, FactoryTechLab"),
            PatchChange::Increased
        );
        assert_eq!(
            compare_values("Armory, FactoryTechLab", "Armory"),
            PatchChange::Decreased
        );
        assert_eq!(compare_values("ground", "air"), PatchChange::Changed);
    }

    #[test]
    fn it_formats_numbers_without_trailing_zeros() {
        assert_eq!(format_number(25.), "25");
        assert_eq!(format_number(2.25), "2.25");
        assert_eq!(format_number(0.8608), "0.861");
    }

    #[test]
    fn it_lists_melee_units_with_their_patches() {
        let list = try_list_patch_units().unwrap();
        assert!(list.versions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(list
            .units
            .iter()
            .any(|unit| unit.name == "Marine" && unit.race == "Terran"));
        assert!(!list
            .units
            .iter()
            .any(|unit| unit.name.contains("Gluescreen")));
        let marine = try_query_unit_overview("Marine".to_string()).unwrap();
        let minerals = marine
            .rows
            .iter()
            .find(|row| row.label == "Minerals")
            .unwrap();
        assert!(minerals.values.iter().all(|value| value.value == "50"));
        assert!(minerals
            .values
            .iter()
            .all(|value| value.change == PatchChange::Same));
        assert!(marine
            .rows
            .iter()
            .any(|row| row.label == "GuassRifle range"));
        // One row per countered unit.
        let strong_vs: Vec<&PatchUnitRow> = marine
            .rows
            .iter()
            .filter(|row| row.label == "Strong vs")
            .collect();
        assert!(strong_vs.len() > 1);
        assert!(strong_vs.iter().all(|row| row.unit.is_some()));
    }

    #[test]
    fn it_lists_the_changes_of_a_patch() {
        let list = try_list_patch_units().unwrap();
        let oldest = try_query_patch_changes(list.versions[0]).unwrap();
        assert_eq!(oldest.previous_version, None);
        assert!(oldest.changes.is_empty());
        let total: usize = list.versions[1..]
            .iter()
            .map(|version| try_query_patch_changes(*version).unwrap().changes.len())
            .sum();
        println!("PATCH CHANGES {total}");
    }
}
