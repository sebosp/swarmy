//! Scores the unit production choices of a player against the army of its opponents, using the
//! strengths and weaknesses from the s2protocol balance data.

use s2protocol::game_events::{
    read_balance_data_from_included_assets, MultiVersionedBalanceUnits, UnitShortRef,
    VersionedBalanceUnits,
};
use s2protocol::SC2Unit;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use swarmy_common::SwarmyError;

/// Workers cost supply but are not part of the army.
pub const WORKERS: [&str; 4] = ["SCV", "Probe", "Drone", "MULE"];

/// A score above this is considered a good choice, below its negative a bad choice.
pub const EVEN_SCORE_THRESHOLD: f64 = 0.05;

/// The balance data is expensive to read, it is shared across requests.
pub fn balance_units() -> Result<&'static MultiVersionedBalanceUnits, SwarmyError> {
    static BALANCE_UNITS: OnceLock<MultiVersionedBalanceUnits> = OnceLock::new();
    if let Some(units) = BALANCE_UNITS.get() {
        return Ok(units);
    }
    let units = read_balance_data_from_included_assets()?;
    Ok(BALANCE_UNITS.get_or_init(|| units))
}

/// Returns the balance data for the protocol version, or the closest one available.
/// The unit strengths, weaknesses and costs rarely change between versions, the ability ids do,
/// so this is only used for the unit choices and not for the event iterator.
pub fn closest_balance_units(version: u32) -> Result<&'static VersionedBalanceUnits, SwarmyError> {
    let closest = closest_balance_version(version)?;
    if closest != version {
        tracing::info!("No balance data for protocol {version}, using {closest} instead");
    }
    Ok(&balance_units()?[&closest])
}

/// The versions of the balance data exported from the Editor, the patches. The derived ones, i.e.
/// 75689 (4.10) with the units of 94137, only name the commands of their replays, they are not
/// compared as patches.
pub fn patch_balance_versions() -> Result<Vec<u32>, SwarmyError> {
    let mut versions: Vec<u32> = balance_units()?
        .keys()
        .copied()
        .filter(|version| !s2protocol::game_events::DERIVED_BALANCE_DATA_VERSIONS.contains(version))
        .collect();
    versions.sort_unstable();
    Ok(versions)
}

/// The version of the balance data closest to the protocol version, the patch of a game.
pub fn closest_balance_version(version: u32) -> Result<u32, SwarmyError> {
    patch_balance_versions()?
        .into_iter()
        .min_by_key(|available| available.abs_diff(version))
        .ok_or_else(|| SwarmyError::Other("No balance data available".to_string()))
}

/// The balance data of the newest protocol version, the costs rarely change between versions.
pub fn latest_balance_units() -> Result<&'static VersionedBalanceUnits, SwarmyError> {
    balance_units()?
        .iter()
        .max_by_key(|(version, _)| **version)
        .map(|(_, units)| units)
        .ok_or_else(|| SwarmyError::Other("No balance data available".to_string()))
}

/// Unit name variants, such as burrowed or sieged, are mapped to the base unit name used by the
/// balance data strengths and weaknesses.
pub fn base_unit_name(name: &str) -> &str {
    match name {
        "SiegeTankSieged" => "SiegeTank",
        "VikingAssault" => "VikingFighter",
        "LiberatorAG" => "Liberator",
        "ThorAP" => "Thor",
        "WarpPrismPhasing" => "WarpPrism",
        "ObserverSiegeMode" => "Observer",
        "OverseerSiegeMode" => "Overseer",
        _ => name.strip_suffix("Burrowed").unwrap_or(name),
    }
}

/// The base unit name of a column of unit names, as [`base_unit_name`] does.
pub fn base_unit_name_expr(column: &str) -> polars::prelude::Expr {
    use polars::prelude::*;
    let renames = [
        ("SiegeTankSieged", "SiegeTank"),
        ("VikingAssault", "VikingFighter"),
        ("LiberatorAG", "Liberator"),
        ("ThorAP", "Thor"),
        ("WarpPrismPhasing", "WarpPrism"),
        ("ObserverSiegeMode", "Observer"),
        ("OverseerSiegeMode", "Overseer"),
    ];
    let unburrowed = col(column).str().strip_suffix(lit("Burrowed"));
    renames.iter().fold(unburrowed, |renamed, (name, base)| {
        when(col(column).eq(lit(*name)))
            .then(lit(*base))
            .otherwise(renamed)
    })
}

/// The value, minerals and vespene, of the units and structures that cost resources, as
/// [`unit_cost`] and [`has_cost`]: columns `unit` and `value`.
pub fn unit_values_frame(
    balance_units: &VersionedBalanceUnits,
) -> Result<polars::prelude::DataFrame, SwarmyError> {
    use polars::prelude::*;
    let (units, values): (Vec<&str>, Vec<u32>) = balance_units
        .keys()
        .filter(|name| has_cost(balance_units, name))
        .map(|name| {
            let cost = unit_cost(balance_units, name);
            (name.as_str(), cost.minerals + cost.vespene)
        })
        .unzip();
    Ok(df!("unit" => units, "value" => values)?)
}

/// The balance data has the build times at Normal speed, the game seconds are at Faster speed,
/// i.e. a Nexus takes 100 seconds at Normal speed and 71 at Faster.
pub const NORMAL_SECONDS_PER_FASTER_SECOND: f32 = 1.4;

/// The cost of one unit, structure or morph.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct UnitCost {
    pub minerals: u32,
    pub vespene: u32,
    pub supply: f32,
    /// The build time in game seconds, at Faster speed.
    pub time: f32,
}

/// Returns the cost of a single unit. Morphs, i.e. Baneling, only cost the morph, not the
/// unit they morphed from.
pub fn unit_cost(balance_units: &VersionedBalanceUnits, name: &str) -> UnitCost {
    let Some(cost) = balance_units.get(name).and_then(|unit| unit.cost.as_ref()) else {
        return UnitCost::default();
    };
    // Zerglings are produced, and costed, in pairs.
    let units_per_cost = if name == "Zergling" { 2 } else { 1 };
    UnitCost {
        minerals: cost.minerals.unwrap_or_default() / units_per_cost,
        vespene: cost.vespene.unwrap_or_default() / units_per_cost,
        supply: cost.supply.unwrap_or_default().max(0.) / units_per_cost as f32,
        time: cost.time.unwrap_or_default() / NORMAL_SECONDS_PER_FASTER_SECOND,
    }
}

/// Returns true if the unit or structure costs resources, larva, eggs, etc do not.
pub fn has_cost(balance_units: &VersionedBalanceUnits, name: &str) -> bool {
    balance_units
        .get(name)
        .and_then(|unit| unit.cost.as_ref())
        .is_some_and(|cost| cost.minerals.unwrap_or(0) > 0 || cost.vespene.unwrap_or(0) > 0)
}

/// The units that "die" without a killer when they turn into another unit: a Drone morphing into
/// a structure, two templars merging into an Archon. In combat these always have a killer.
pub const MORPH_SOURCES: [&str; 3] = ["Drone", "HighTemplar", "DarkTemplar"];

/// Returns true if the death without a killer is the unit turning into another unit, not a loss.
pub fn is_morph_death(name: &str, has_killer: bool) -> bool {
    !has_killer && MORPH_SOURCES.contains(&name)
}

/// Returns the supply cost of a unit, structures and units such as larva or eggs have none.
pub fn unit_supply(balance_units: &VersionedBalanceUnits, name: &str) -> Option<f32> {
    balance_units
        .get(name)?
        .cost
        .as_ref()?
        .supply
        .filter(|supply| *supply > 0.)
}

/// Returns the supply cost of a unit if it is part of the army.
pub fn army_supply(balance_units: &VersionedBalanceUnits, name: &str) -> Option<f32> {
    if WORKERS.contains(&name) {
        return None;
    }
    unit_supply(balance_units, name)
}

fn refers_to(units: &[UnitShortRef], name: &str) -> bool {
    units.iter().any(|unit| unit.id == name)
}

/// How a produced unit fares against the enemy army at the time it was produced.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitChoice {
    /// From -1 (counters every enemy unit) to 1 (every enemy unit is countered), weighted by
    /// the enemy units supply.
    pub score: f64,
    /// The enemy units this unit is good against, by name and count.
    pub strong_vs: BTreeMap<String, usize>,
    /// The enemy units this unit is bad against, by name and count.
    pub weak_vs: BTreeMap<String, usize>,
}

impl UnitChoice {
    pub fn is_good(&self) -> bool {
        self.score > EVEN_SCORE_THRESHOLD
    }

    pub fn is_bad(&self) -> bool {
        self.score < -EVEN_SCORE_THRESHOLD
    }
}

/// Scores producing `unit_name` against the enemy units.
/// Returns None if the unit is not part of the army or the enemy has no army yet.
pub fn score_unit_choice<'a>(
    balance_units: &VersionedBalanceUnits,
    unit_name: &str,
    enemy_units: impl IntoIterator<Item = &'a SC2Unit>,
) -> Option<UnitChoice> {
    army_supply(balance_units, unit_name)?;
    let unit = balance_units.get(unit_name)?;
    // Morphing cocoons and eggs cost supply but are not a choice, the hatched unit is.
    if unit.strengths.inner.is_empty() && unit.weaknesses.inner.is_empty() {
        return None;
    }
    let mut total_supply = 0f64;
    let mut weighted_score = 0f64;
    let mut strong_vs = BTreeMap::new();
    let mut weak_vs = BTreeMap::new();
    for enemy in enemy_units {
        let enemy_name = base_unit_name(&enemy.name);
        let (Some(enemy_supply), Some(enemy_unit)) = (
            army_supply(balance_units, enemy_name),
            balance_units.get(enemy_name),
        ) else {
            continue;
        };
        // The balance data is not symmetric, i.e. the Roach is strong vs the Zergling but the
        // Zergling does not list the Roach as a weakness, so both sides are checked.
        let favorable = refers_to(&unit.strengths.inner, enemy_name)
            || refers_to(&enemy_unit.weaknesses.inner, unit_name);
        let unfavorable = refers_to(&unit.weaknesses.inner, enemy_name)
            || refers_to(&enemy_unit.strengths.inner, unit_name);
        let advantage = favorable as i32 - unfavorable as i32;
        total_supply += enemy_supply as f64;
        weighted_score += advantage as f64 * enemy_supply as f64;
        match advantage {
            1 => *strong_vs.entry(enemy_name.to_string()).or_default() += 1,
            -1 => *weak_vs.entry(enemy_name.to_string()).or_default() += 1,
            _ => {}
        }
    }
    if total_supply == 0. {
        return None;
    }
    Some(UnitChoice {
        score: weighted_score / total_supply,
        strong_vs,
        weak_vs,
    })
}

/// Formats unit counts as `12xZergling,2xQueen`.
pub fn format_unit_counts(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(name, count)| format!("{count}x{name}"))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use s2protocol::game_events::read_balance_data_from_included_assets;

    fn unit(name: &str) -> SC2Unit {
        SC2Unit {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn latest_balance_units() -> VersionedBalanceUnits {
        let all = read_balance_data_from_included_assets().unwrap();
        let latest = all.keys().max().unwrap();
        all[latest].clone()
    }

    #[test]
    fn it_scores_unit_choices() {
        let balance_units = latest_balance_units();
        let zerglings = [unit("Zergling"), unit("Zergling"), unit("ZerglingBurrowed")];
        let roach = score_unit_choice(&balance_units, "Roach", &zerglings).unwrap();
        assert!(roach.is_good());
        assert_eq!(roach.strong_vs["Zergling"], 3);
        let marauders = [unit("Marauder")];
        let roach = score_unit_choice(&balance_units, "Roach", &marauders).unwrap();
        assert!(roach.is_bad());
        // Workers and structures are not choices, an enemy without army can not be countered.
        assert_eq!(score_unit_choice(&balance_units, "Drone", &zerglings), None);
        assert_eq!(
            score_unit_choice(&balance_units, "Roach", &[unit("SCV")]),
            None
        );
        assert_eq!(
            score_unit_choice(&balance_units, "LurkerMPEgg", &zerglings),
            None
        );
    }

    #[test]
    fn it_maps_unit_variants() {
        assert_eq!(base_unit_name("RoachBurrowed"), "Roach");
        assert_eq!(base_unit_name("SiegeTankSieged"), "SiegeTank");
        assert_eq!(base_unit_name("Zergling"), "Zergling");
    }
}
