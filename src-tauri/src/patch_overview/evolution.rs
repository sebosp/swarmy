//! How the costs of a unit, and of what it trains, builds, researches and upgrades, change over
//! the patches of the balance data, for the Patch Evolution view.

use crate::replay_rerun::decisions::{balance_units, NORMAL_SECONDS_PER_FASTER_SECOND};
use s2protocol::game_events::{Cost, VersionedBalanceUnit};
use std::collections::BTreeSet;
use swarmy_common::*;

/// The settings key of the saved queries.
const SAVED_EVOLUTIONS_KEY: &str = "patch_evolutions";

/// What a unit trains, builds, researches or upgrades in a patch: its item name, cost and
/// requirements.
fn unit_items(unit: &VersionedBalanceUnit) -> Vec<(String, Option<&Cost>, usize)> {
    let mut items = vec![(
        EVOLUTION_UNIT_ITEM.to_string(),
        unit.cost.as_ref(),
        unit.requires.inner.len(),
    )];
    for (verb, refs) in [
        ("Trains", &unit.trains.inner),
        ("Builds", &unit.builds.inner),
    ] {
        for unit_ref in refs {
            items.push((
                format!("{verb} {}", unit_ref.id),
                unit_ref.cost.as_ref(),
                unit_ref.requires.inner.len(),
            ));
        }
    }
    for research in unit
        .researches
        .iter()
        .flat_map(|researches| &researches.inner)
    {
        items.push((
            format!("Researches {}", research.id),
            Some(&research.cost),
            0,
        ));
    }
    for upgrade in &unit.upgrades.inner {
        for level in &upgrade.levels {
            let requirements = level
                .requires
                .iter()
                .map(|requires| requires.unit.len() + requires.upgrade.len())
                .sum();
            items.push((
                format!("Upgrades {}", level.id),
                Some(&level.cost),
                requirements,
            ));
        }
    }
    items
}

/// The items of the unit in any patch, the unit itself first.
pub fn try_list_evolution_items(unit_name: &str) -> Result<Vec<String>, SwarmyError> {
    let mut items = BTreeSet::new();
    for units in balance_units()?.values() {
        if let Some(unit) = units.get(unit_name) {
            items.extend(unit_items(unit).into_iter().map(|(item, _, _)| item));
        }
    }
    items.remove(EVOLUTION_UNIT_ITEM);
    Ok(std::iter::once(EVOLUTION_UNIT_ITEM.to_string())
        .chain(items)
        .collect())
}

fn metric_value(cost: Option<&Cost>, requirements: usize, metric: EvolutionMetric) -> Option<f64> {
    match metric {
        EvolutionMetric::Minerals => cost?.minerals.map(f64::from),
        EvolutionMetric::Vespene => cost?.vespene.map(f64::from),
        EvolutionMetric::Supply => cost?.supply.map(f64::from),
        EvolutionMetric::Time => cost?
            .time
            .map(|time| ((time / NORMAL_SECONDS_PER_FASTER_SECOND) as f64 * 10.).round() / 10.),
        EvolutionMetric::Exists => Some(1.),
        EvolutionMetric::Requirements => Some(requirements as f64),
    }
}

/// The values of every series in every patch.
pub fn try_query_patch_evolution(
    series: &[EvolutionSeries],
) -> Result<PatchEvolution, SwarmyError> {
    let all_units = balance_units()?;
    let mut versions: Vec<u32> = all_units.keys().copied().collect();
    versions.sort();
    let values = series
        .iter()
        .map(|series| {
            versions
                .iter()
                .map(|version| {
                    let item = all_units[version].get(&series.unit).and_then(|unit| {
                        unit_items(unit)
                            .into_iter()
                            .find(|(item, _, _)| *item == series.item)
                    });
                    match item {
                        Some((_, cost, requirements)) => {
                            metric_value(cost, requirements, series.metric)
                        }
                        // Not in the patch.
                        None if series.metric == EvolutionMetric::Exists => Some(0.),
                        None => None,
                    }
                })
                .collect()
        })
        .collect();
    Ok(PatchEvolution { versions, values })
}

fn settings_store(
    app_handle: &tauri::AppHandle,
) -> Result<std::sync::Arc<tauri_plugin_store::Store<tauri::Wry>>, SwarmyError> {
    let store = tauri_plugin_store::StoreBuilder::new(app_handle, "settings.json").build()?;
    let _ = store.reload();
    Ok(store)
}

/// The queries saved in the settings.
pub fn load_saved_evolutions(
    app_handle: &tauri::AppHandle,
) -> Result<Vec<SavedEvolution>, SwarmyError> {
    Ok(settings_store(app_handle)?
        .get(SAVED_EVOLUTIONS_KEY)
        .and_then(|saved| serde_json::from_value(saved).ok())
        .unwrap_or_default())
}

/// Saves a query in the settings, replacing the one with the same name, an empty series removes
/// it.
pub fn save_evolution(
    app_handle: &tauri::AppHandle,
    saved: SavedEvolution,
) -> Result<Vec<SavedEvolution>, SwarmyError> {
    let mut all = load_saved_evolutions(app_handle)?;
    all.retain(|other| other.name != saved.name);
    if !saved.series.is_empty() {
        all.push(saved);
    }
    all.sort_by(|a, b| a.name.cmp(&b.name));
    let store = settings_store(app_handle)?;
    store.set(
        SAVED_EVOLUTIONS_KEY,
        serde_json::to_value(&all).map_err(|err| SwarmyError::Other(err.to_string()))?,
    );
    store.save()?;
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_follows_the_costs_over_the_patches() {
        let items = try_list_evolution_items("Barracks").unwrap();
        assert_eq!(items[0], EVOLUTION_UNIT_ITEM);
        assert!(items.contains(&"Trains Marine".to_string()), "{items:?}");
        let series = [
            EvolutionSeries {
                unit: "Barracks".to_string(),
                item: "Trains Marine".to_string(),
                metric: EvolutionMetric::Minerals,
            },
            EvolutionSeries {
                unit: "Marine".to_string(),
                item: EVOLUTION_UNIT_ITEM.to_string(),
                metric: EvolutionMetric::Exists,
            },
        ];
        let evolution = try_query_patch_evolution(&series).unwrap();
        assert_eq!(evolution.values.len(), 2);
        assert_eq!(evolution.values[0].len(), evolution.versions.len());
        assert!(evolution.values[0]
            .iter()
            .flatten()
            .all(|minerals| *minerals == 50.));
        assert!(evolution.values[1].iter().all(|exists| *exists == Some(1.)));
    }
}
