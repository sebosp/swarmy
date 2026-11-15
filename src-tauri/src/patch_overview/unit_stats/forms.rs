#![doc = include_str!("forms/README.md")]

use crate::data::*;
use crate::replay_rerun::decisions::{closest_balance_units, patch_balance_versions};
use polars::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The morphs of every patch, `forms/unit_forms.json`: written by [`generate`] over a large
/// snapshot when a patch comes out, reviewed, committed. The Unit Overview reads it, no query of
/// the snapshot at request time.
const UNIT_FORMS_JSON: &str = include_str!("forms/unit_forms.json");

/// The patches and their morphs, `patch -> pairs`.
pub type PatchForms = BTreeMap<u32, Vec<UnitFormChange>>;

/// The forms the units of the games changed into and back, by patch: a unit that burrows,
/// sieges, lands, morphs or upgrades its building is the same unit (tag) born again with another
/// type in `unit_born` (the tracker's unit type change). For every pair of consecutive types of a
/// unit, `patch` (u32, of the game), `from`, `to` and `changes` (u32, how many times it happened);
/// by patch, the most frequent first, then by names. Run on one day partition at a time by
/// [`generate`]: the sort of a whole snapshot's `unit_born` took 13 GB.
pub fn plan(source: &dyn SnapshotSource) -> Result<LazyFrame, SwarmyError> {
    let patches = source
        .table(DETAILS_IPC)?
        .select([col("ext_fs_id"), col("ext_protocol_version")])
        .unique(None, UniqueKeepStrategy::Any)
        .join(
            patch_versions_of(source)?,
            [col("ext_protocol_version")],
            [col("ext_protocol_version")],
            JoinArgs::new(JoinType::Inner),
        )
        .select([col("ext_fs_id"), col("patch")]);
    // Sorted by unit then time, the previous row is the previous type of the same unit when its
    // keys match: a shift of the whole frame, no window per unit (millions of units).
    let same_unit = col("ext_fs_id")
        .eq(col("previous_game"))
        .and(col("unit_tag_index").eq(col("previous_tag")))
        .and(col("unit_tag_recycle").eq(col("previous_recycle")));
    Ok(source
        .table(UNIT_BORN_IPC)?
        .select([
            col("ext_fs_id"),
            col("unit_tag_index"),
            col("unit_tag_recycle"),
            col("unit_type_name"),
            col("ext_replay_loop"),
        ])
        .sort_by_exprs(
            [
                col("ext_fs_id"),
                col("unit_tag_index"),
                col("unit_tag_recycle"),
                col("ext_replay_loop"),
            ],
            Default::default(),
        )
        .with_columns([
            col("ext_fs_id").shift(lit(1)).alias("previous_game"),
            col("unit_tag_index").shift(lit(1)).alias("previous_tag"),
            col("unit_tag_recycle")
                .shift(lit(1))
                .alias("previous_recycle"),
            col("unit_type_name").shift(lit(1)).alias("from"),
        ])
        .filter(same_unit.and(col("from").neq(col("unit_type_name"))))
        .join(
            patches,
            [col("ext_fs_id")],
            [col("ext_fs_id")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("patch"), col("from"), col("unit_type_name").alias("to")])
        .agg([len().alias("changes")])
        .sort_by_exprs(
            [col("patch"), col("changes"), col("from"), col("to")],
            SortMultipleOptions::default().with_order_descending_multi([false, true, false, false]),
        ))
}

/// A change rarer than this share of the most frequent change of the unit is noise: a cancelled
/// morph of a few games that the balance data can't tell apart.
pub const MIN_SHARE: f64 = 0.01;

/// The rows of [`plan`], by patch.
pub fn shape(frame: DataFrame) -> Result<PatchForms, SwarmyError> {
    let mut forms = PatchForms::new();
    for (((patch, from), to), changes) in u32_values(&frame, "patch")?
        .into_iter()
        .zip(str_values(&frame, "from")?)
        .zip(str_values(&frame, "to")?)
        .zip(u32_values(&frame, "changes")?)
    {
        forms
            .entry(patch)
            .or_default()
            .push(UnitFormChange { from, to, changes });
    }
    Ok(forms)
}

/// Only the pairs whose two types are units of the patch's balance data: a unit of a custom map
/// (`Lair2`), a wreckage or a decoration is not a form of a melee unit.
pub fn validate(forms: PatchForms) -> Result<PatchForms, SwarmyError> {
    let mut valid = PatchForms::new();
    for (patch, changes) in forms {
        let balance = closest_balance_units(patch)?;
        let kept: Vec<UnitFormChange> = changes
            .into_iter()
            .filter(|change| balance.contains_key(&change.from) && balance.contains_key(&change.to))
            .collect();
        if !kept.is_empty() {
            valid.insert(patch, kept);
        }
    }
    Ok(valid)
}

/// The order of the pairs of a patch: the most frequent first, then by names.
fn sort_pairs(changes: &mut [UnitFormChange]) {
    changes.sort_by(|a, b| {
        b.changes
            .cmp(&a.changes)
            .then_with(|| a.from.cmp(&b.from))
            .then_with(|| a.to.cmp(&b.to))
    });
}

/// The morphs of every patch of the snapshot at `ipc_path`, validated: [`plan`] on each day
/// partition (a day at a time: 2.6 GB at the peak over the pro snapshot), the counts added up.
/// Writes nothing: the
/// ignored test `write_unit_forms` turns the result into `unit_forms.json`.
pub fn generate(ipc_path: &str) -> Result<PatchForms, SwarmyError> {
    let mut counts: HashMap<(u32, String, String), u32> = HashMap::new();
    let days =
        s2protocol::arrow_store::partitioned::list_partitions(std::path::Path::new(ipc_path))?;
    for day in days {
        let Some(date) = partition_date(&day) else {
            continue;
        };
        let source = IpcSnapshot::at(ipc_path).on_day(date);
        for (patch, changes) in shape(plan(&source)?.collect()?)? {
            for change in changes {
                *counts.entry((patch, change.from, change.to)).or_default() += change.changes;
            }
        }
    }
    let mut forms = PatchForms::new();
    for ((patch, from, to), changes) in counts {
        forms
            .entry(patch)
            .or_default()
            .push(UnitFormChange { from, to, changes });
    }
    forms.values_mut().for_each(|changes| sort_pairs(changes));
    validate(forms)
}

/// The date of a day partition, `.../year=2026/month=11/day=12`.
fn partition_date(day: &std::path::Path) -> Option<chrono::NaiveDate> {
    let value = |key: &str| -> Option<u32> {
        day.components()
            .filter_map(|part| part.as_os_str().to_str())
            .find_map(|part| part.strip_prefix(key)?.parse().ok())
    };
    chrono::NaiveDate::from_ymd_opt(value("year=")? as i32, value("month=")?, value("day=")?)
}

/// The morphs of every patch, from `unit_forms.json` (parsed once).
pub fn static_forms() -> Result<&'static PatchForms, SwarmyError> {
    static FORMS: OnceLock<PatchForms> = OnceLock::new();
    if let Some(forms) = FORMS.get() {
        return Ok(forms);
    }
    let parsed: BTreeMap<u32, Vec<UnitFormChange>> = serde_json::from_str(UNIT_FORMS_JSON)
        .map_err(|err| SwarmyError::Other(format!("unit_forms.json: {err}")))?;
    Ok(FORMS.get_or_init(|| parsed))
}

/// The morphs of a patch: its own table, else the one of the newest patch before it (the morphs
/// rarely change), else the oldest one.
pub fn forms_of_patch(patch: u32) -> Result<&'static [UnitFormChange], SwarmyError> {
    Ok(table_of(static_forms()?, patch))
}

/// [`forms_of_patch`] in a given set of tables.
pub fn table_of(forms: &PatchForms, patch: u32) -> &[UnitFormChange] {
    forms
        .range(..=patch)
        .next_back()
        .or_else(|| forms.iter().next())
        .map(|(_, changes)| changes.as_slice())
        .unwrap_or_default()
}

/// The morphs of the newest patch of the balance data, the one the Unit Overview compares with.
pub fn latest_forms() -> Result<&'static [UnitFormChange], SwarmyError> {
    let latest = patch_balance_versions()?
        .last()
        .copied()
        .unwrap_or_default();
    forms_of_patch(latest)
}

/// The morphs of a unit: the types it changes into, directly or through another form (a Viking
/// lands as VikingAssault, a Hatchery becomes a Lair then a Hive, a Roach a Ravager), and the
/// changes among them; the pairs under [`MIN_SHARE`] of the most frequent are left out. What
/// changes into the unit is not a form of it: the Hydralisk that morphs into a Lurker is another
/// unit, with its own page.
pub fn forms_of(unit: &str, changes: &[UnitFormChange]) -> Vec<UnitFormChange> {
    let most = changes
        .iter()
        .filter(|change| change.from == unit || change.to == unit)
        .map(|change| change.changes)
        .max()
        .unwrap_or_default();
    let changes: Vec<UnitFormChange> = changes
        .iter()
        .filter(|change| f64::from(change.changes) >= f64::from(most) * MIN_SHARE)
        .cloned()
        .collect();
    let mut names: Vec<String> = vec![unit.to_string()];
    loop {
        let before = names.len();
        for change in &changes {
            if names.contains(&change.from) && !names.contains(&change.to) {
                names.push(change.to.clone());
            }
        }
        if names.len() == before {
            break;
        }
    }
    changes
        .iter()
        .filter(|change| names.contains(&change.from) && names.contains(&change.to))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests;
