//! The units of each patch from the s2protocol multi versioned balance data.

use crate::replay_rerun::decisions::{balance_units, has_cost, latest_balance_units};
use s2protocol::game_events::{Cost, UnitRef, UnitShortRef, UpgradeLevel, VersionedBalanceUnit};
use std::collections::{BTreeMap, BTreeSet};
use swarmy_common::*;

/// A list of a unit in the balance data, i.e. what it is strong against or trains.
type Listed<T> = fn(&VersionedBalanceUnit) -> &[T];

/// Shown when the unit or the property is not in a patch.
const MISSING: &str = "-";

/// Shown when a unit is in a list of a patch, i.e. a unit is strong against it.
const PRESENT: &str = "✓";

/// The co-op and campaign variants that share the names of the melee units.
fn is_melee_unit(name: &str) -> bool {
    !name.contains("Gluescreen") && !name.ends_with("Dummy")
}

/// Terran, Zerg, Protoss or Neutral, the units without a race: mineral fields, rocks, critters,
/// Xel'Naga towers.
pub fn race_name(unit: &VersionedBalanceUnit) -> String {
    match unit.meta.race.as_deref() {
        Some("Terr") => "Terran",
        Some("Zerg") => "Zerg",
        Some("Prot") => "Protoss",
        _ => "Neutral",
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

/// A cost as the game shows it: `100m 100g, 2 supply, 114 s` (the time in game seconds at Faster
/// speed); the parts that are not set are left out.
fn format_cost(cost: &Cost) -> String {
    let mut parts = vec![];
    let mut resources = vec![];
    if let Some(minerals) = cost.minerals {
        resources.push(format!("{minerals}m"));
    }
    if let Some(vespene) = cost.vespene {
        resources.push(format!("{vespene}g"));
    }
    if !resources.is_empty() {
        parts.push(resources.join(" "));
    }
    if let Some(supply) = cost.supply.filter(|supply| *supply > 0.) {
        parts.push(format!("{} supply", format_number(supply)));
    }
    if let Some(energy) = cost.energy {
        parts.push(format!("{energy} energy"));
    }
    if let Some(time) = cost.time {
        let seconds = time / crate::replay_rerun::decisions::NORMAL_SECONDS_PER_FASTER_SECOND;
        parts.push(format!(
            "{} s",
            format_number((seconds * 10.).round() / 10.)
        ));
    }
    if let Some(cooldown) = cost.cooldown {
        parts.push(format!("{} s cooldown", format_number(cooldown)));
    }
    if parts.is_empty() {
        PRESENT.to_string()
    } else {
        parts.join(", ")
    }
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
    // The costs (`100m 100g, 114 s`) and the other texts with numbers in the same places: all
    // the numbers up or the same is an increase, all down a decrease, mixed a change.
    if let Some(change) = compare_numbers_in(previous, value) {
        return change;
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

/// The numbers of a text and the text without them, `100m 100g, 114 s` is `[100, 100, 114]` and
/// `#m #g, # s`.
fn numbers_and_skeleton(text: &str) -> (Vec<f32>, String) {
    let mut numbers = vec![];
    let mut skeleton = String::new();
    let mut number = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || (c == '.' && !number.is_empty()) {
            number.push(c);
        } else {
            if !number.is_empty() {
                if let Ok(parsed) = number.parse::<f32>() {
                    numbers.push(parsed);
                    skeleton.push('#');
                }
                number.clear();
            }
            skeleton.push(c);
        }
    }
    (numbers, skeleton)
}

/// How two texts with numbers in the same places compare, None when they have none or differ
/// elsewhere.
fn compare_numbers_in(previous: &str, value: &str) -> Option<PatchChange> {
    let (before, skeleton_before) = numbers_and_skeleton(previous);
    let (after, skeleton_after) = numbers_and_skeleton(value);
    if before.is_empty() || before.len() != after.len() || skeleton_before != skeleton_after {
        return None;
    }
    let up = before.iter().zip(&after).any(|(b, a)| a > b);
    let down = before.iter().zip(&after).any(|(b, a)| a < b);
    Some(match (up, down) {
        (true, false) => PatchChange::Increased,
        (false, true) => PatchChange::Decreased,
        (false, false) => PatchChange::Same,
        (true, true) => PatchChange::Changed,
    })
}

/// The id of a level of an upgrade: the upgrade's own when the level has none (`Stimpack`,
/// `ShieldWall`, the one-level upgrades).
pub fn level_id(upgrade: &s2protocol::game_events::Upgrade, level: &UpgradeLevel) -> String {
    if level.id.is_empty() {
        upgrade.id.clone()
    } else {
        level.id.clone()
    }
}

/// The upgrades of a unit as trees: the levels of each upgrade, a level under the level it
/// requires (`TerranInfantryWeaponsLevel2` needs `Level1`), with the structures it requires.
pub fn upgrade_trees(unit: &VersionedBalanceUnit) -> Vec<UpgradeNode> {
    let mut trees = vec![];
    for upgrade in &unit.upgrades.inner {
        let node_of = |level: &UpgradeLevel| UpgradeNode {
            id: level_id(upgrade, level),
            cost: format_cost(&level.cost),
            requires_units: level
                .requires
                .iter()
                .flat_map(|requirement| requirement.unit.iter().map(|unit| unit.id.clone()))
                .collect(),
            children: vec![],
        };
        let parent_of = |level: &UpgradeLevel| -> Option<String> {
            level
                .requires
                .iter()
                .flat_map(|requirement| requirement.upgrade.iter())
                .map(|required| required.id.clone())
                .find(|required| {
                    upgrade
                        .levels
                        .iter()
                        .any(|other| level_id(upgrade, other) == *required)
                })
        };
        // The roots first, then each level under its parent, in the order of the levels.
        let mut roots: Vec<UpgradeNode> = vec![];
        for level in &upgrade.levels {
            let node = node_of(level);
            match parent_of(level) {
                Some(parent) => {
                    if !attach(&mut roots, &parent, node.clone()) {
                        roots.push(node);
                    }
                }
                None => roots.push(node),
            }
        }
        trees.extend(roots);
    }
    trees
}

/// Puts the node under the node of `parent_id`, wherever it is in the trees.
fn attach(nodes: &mut [UpgradeNode], parent_id: &str, node: UpgradeNode) -> bool {
    for candidate in nodes.iter_mut() {
        if candidate.id == parent_id {
            candidate.children.push(node);
            return true;
        }
        if attach(&mut candidate.children, parent_id, node.clone()) {
            return true;
        }
    }
    false
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
    // The derived balance data are not patches.
    let versions = crate::replay_rerun::decisions::patch_balance_versions()?;
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
    // The derived balance data are not patches.
    let versions = crate::replay_rerun::decisions::patch_balance_versions()?;
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
    rows.extend([
        row("Supply provided", Benefit, &|unit| {
            // The townhalls, the Overlords, the Pylons and the Supply Depots: the units that use
            // supply have a negative number here.
            let supply = unit.misc.supply?;
            (supply > 0.).then(|| format_number(supply))
        }),
        row("Energy", Benefit, &|unit| {
            Some(unit.cost.as_ref()?.energy?.to_string())
        }),
        row("Life regeneration", Benefit, &|unit| {
            Some(format_number(unit.life.regen_rate?))
        }),
        row("Shield regeneration", Benefit, &|unit| {
            Some(format_number(unit.shields.as_ref()?.regen_rate))
        }),
        row("Shield armor", Benefit, &|unit| {
            unit.shields.as_ref()?;
            Some(unit.shield_armor.start.to_string())
        }),
        row("Acceleration", Benefit, &|unit| {
            Some(format_number(unit.movement.as_ref()?.acceleration))
        }),
        row("Movement", Neutral, &|unit| {
            Some(unit.movement.as_ref()?.kind.clone())
        }),
        row("Radius", Neutral, &|unit| {
            Some(format_number(unit.misc.radius?))
        }),
        row("Footprint", Neutral, &|unit| unit.misc.footprint.clone()),
        row("Cargo size", Cost, &|unit| {
            let size = unit.misc.cargo_size?;
            (size > 0).then(|| size.to_string())
        }),
        row("Cargo capacity", Benefit, &|unit| {
            Some(unit.misc.cargo_capacity?.to_string())
        }),
        row("Build score", Neutral, &|unit| {
            Some(unit.score.build?.to_string())
        }),
        row("Kill score", Neutral, &|unit| {
            Some(unit.score.kill?.to_string())
        }),
        row("Abilities", Neutral, &|unit| {
            let abilities: Vec<&str> = unit
                .abilities
                .inner
                .iter()
                .filter(|ability| {
                    ability
                        .commands
                        .iter()
                        .any(|command| !command.meta.name.is_empty())
                })
                .map(|ability| ability.id.as_str())
                .collect();
            (!abilities.is_empty()).then(|| abilities.join(", "))
        }),
    ]);
    // What the unit trains and builds, one row per unit with its cost in that patch.
    let productions: [(&str, Listed<UnitRef>); 2] = [
        ("Trains", |unit| &unit.trains.inner),
        ("Builds", |unit| &unit.builds.inner),
    ];
    for (label, list) in productions {
        let names: BTreeSet<String> = units
            .iter()
            .flatten()
            .flat_map(|unit| list(unit).iter().map(|produced| produced.id.clone()))
            .collect();
        for name in names {
            let mut production_row = row(label, Cost, &|unit| {
                let produced = list(unit).iter().find(|produced| produced.id == name)?;
                Some(
                    produced
                        .cost
                        .as_ref()
                        .map(format_cost)
                        .unwrap_or_else(|| PRESENT.to_string()),
                )
            });
            production_row.unit = Some(name);
            rows.push(production_row);
        }
    }
    // The upgrades of the unit (its weapons and armor levels) and the researches of a structure,
    // one row per level with its cost.
    let upgrade_levels: BTreeSet<String> = units
        .iter()
        .flatten()
        .flat_map(|unit| {
            unit.upgrades.inner.iter().flat_map(|upgrade| {
                upgrade
                    .levels
                    .iter()
                    .map(move |level| level_id(upgrade, level))
            })
        })
        .collect();
    for id in upgrade_levels {
        rows.push(row(&format!("Upgrade {id}"), Cost, &|unit| {
            let level = unit
                .upgrades
                .inner
                .iter()
                .flat_map(|upgrade| {
                    upgrade
                        .levels
                        .iter()
                        .map(move |level| (level_id(upgrade, level), level))
                })
                .find(|(level_id, _)| *level_id == id)
                .map(|(_, level)| level)?;
            Some(format_cost(&level.cost))
        }));
    }
    let researches: BTreeSet<String> = units
        .iter()
        .flatten()
        .flat_map(|unit| unit.researches.iter())
        .flat_map(|researches| researches.inner.iter().map(|research| research.id.clone()))
        .collect();
    for research_id in researches {
        rows.push(row(&format!("Research {research_id}"), Cost, &|unit| {
            let research = unit
                .researches
                .as_ref()?
                .inner
                .iter()
                .find(|research| research.id == research_id)?;
            Some(format_cost(&research.cost))
        }));
    }
    // One row per unit countered or countering in any patch, a list of names is too packed.
    let counters: [(&str, PatchRowKind, Listed<UnitShortRef>); 2] = [
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
    // The dependency tree of the upgrades, of the latest patch that has the unit.
    let upgrade_trees = units
        .iter()
        .rev()
        .flatten()
        .next()
        .map(|unit| upgrade_trees(unit))
        .unwrap_or_default();
    Ok(PatchUnit {
        name: unit_name,
        race,
        versions,
        rows,
        upgrade_trees,
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
    fn it_compares_the_numbers_inside_texts() {
        assert_eq!(
            compare_values("100m 100g, 114 s", "150m 150g, 114 s"),
            PatchChange::Increased
        );
        assert_eq!(
            compare_values("175m, 2 supply, 35.7 s", "150m, 2 supply, 35.7 s"),
            PatchChange::Decreased
        );
        assert_eq!(
            compare_values("100m 100g, 114 s", "150m 100g, 100 s"),
            PatchChange::Changed
        );
        assert_eq!(
            compare_values("100m 100g, 114 s", "100m, 114 s"),
            PatchChange::Changed,
            "another skeleton"
        );
    }

    #[test]
    fn the_upgrade_levels_hang_from_the_level_they_need() {
        let balance = latest_balance_units().unwrap();
        let trees = upgrade_trees(&balance["Marine"]);
        let weapons = trees
            .iter()
            .find(|node| node.id == "TerranInfantryWeaponsLevel1")
            .expect("the first level is a root");
        assert_eq!(weapons.children.len(), 1);
        let level2 = &weapons.children[0];
        assert_eq!(level2.id, "TerranInfantryWeaponsLevel2");
        assert_eq!(level2.requires_units, ["Armory"]);
        assert_eq!(level2.children[0].id, "TerranInfantryWeaponsLevel3");
        assert!(weapons.requires_units.is_empty());
        // A one-level upgrade without a level id takes the upgrade's.
        assert!(trees.iter().any(|node| node.id == "Stimpack"), "{trees:?}");
    }

    #[test]
    fn it_formats_the_costs() {
        let cost = Cost {
            minerals: Some(100),
            vespene: Some(100),
            supply: None,
            time: Some(160.),
            energy: None,
            cooldown: None,
            life: None,
        };
        assert_eq!(format_cost(&cost), "100m 100g, 114.3 s");
        let queen = Cost {
            minerals: Some(175),
            vespene: None,
            supply: Some(2.),
            time: Some(50.),
            ..cost.clone()
        };
        assert_eq!(format_cost(&queen), "175m, 2 supply, 35.7 s");
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
        assert!(
            list.units
                .iter()
                .any(|unit| unit.name == "Marine" && unit.race == "Terran")
        );
        assert!(
            !list
                .units
                .iter()
                .any(|unit| unit.name.contains("Gluescreen"))
        );
        let marine = try_query_unit_overview("Marine".to_string()).unwrap();
        let minerals = marine
            .rows
            .iter()
            .find(|row| row.label == "Minerals")
            .unwrap();
        assert!(minerals.values.iter().all(|value| value.value == "50"));
        assert!(
            minerals
                .values
                .iter()
                .all(|value| value.change == PatchChange::Same)
        );
        assert!(
            marine
                .rows
                .iter()
                .any(|row| row.label == "GuassRifle range")
        );
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
