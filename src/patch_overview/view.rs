//! The changes of a patch compared to the previous one, from the s2protocol balance data.

use crate::components::abilities::{fetch_ability_usage, AbilityUsageRows};
use crate::components::links::{RaceLink, UnitLink};
use crate::components::patch::ChangeMarker;
use crate::navigation::use_navigation;
use crate::*;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

/// The patch of the balance data closest to the replay patch, replays from versions without
/// balance data use the closest one.
fn closest_version(versions: &[u32], version: u32) -> Option<u32> {
    versions
        .iter()
        .copied()
        .min_by_key(|available| available.abs_diff(version))
}

const RACES: [&str; 4] = ["Terran", "Zerg", "Protoss", "Other"];

#[component]
pub fn PatchOverviewView() -> impl IntoView {
    let navigation = use_navigation();
    let versions = RwSignal::new(Vec::<u32>::new());
    let selected = RwSignal::new(None::<u32>);
    let changes = RwSignal::new(None::<PatchChanges>);
    let error = RwSignal::new(None::<String>);
    // The abilities need a snapshot, without it only the balance data changes are shown.
    let usage = RwSignal::new(None::<Result<AbilityUsageTable, String>>);
    spawn_local(async move {
        usage.set(Some(fetch_ability_usage().await));
    });
    spawn_local(async move {
        match invoke_json::<PatchUnitList>("list_patch_units", &()).await {
            Ok(list) => {
                // The newest patch first.
                let mut list_versions = list.versions;
                list_versions.reverse();
                versions.set(list_versions);
            }
            Err(err) => error.set(Some(err)),
        }
    });
    // The patch requested from another view, i.e. the patch version of a replay.
    Effect::new(move |_| {
        let available = versions.get();
        let version = match navigation.patch_version.get() {
            Some(version) => closest_version(&available, version),
            None => selected.get_untracked().or(available.first().copied()),
        };
        if version.is_some() && version != selected.get_untracked() {
            selected.set(version);
        }
    });
    Effect::new(move |_| {
        let Some(version) = selected.get() else {
            return;
        };
        spawn_local(async move {
            match invoke_json::<PatchChanges>("query_patch_changes", &PatchChangesQuery { version })
                .await
            {
                Ok(response) => {
                    error.set(None);
                    changes.set(Some(response));
                }
                Err(err) => error.set(Some(err)),
            }
        });
    });
    view! {
        <div class="p-2 space-y-2">
            <h1 class="text-xl font-semibold text-white">"Patch overview"</h1>
            <p class="text-xs text-gray-500">
                "The units that changed in a patch, from the s2protocol balance data. Click on a unit to see it in every patch."
            </p>
            <div class="w-64">
                <label class="block text-sm/6 font-medium text-white">"Patch"</label>
                <select
                    class="select select-sm w-full bg-gray-800 text-gray-200 border border-white/20"
                    on:change=move |ev| {
                        navigation.patch_version.set(None);
                        selected.set(event_target_value(&ev).parse().ok());
                    }
                >
                    {move || {
                        versions
                            .get()
                            .into_iter()
                            .map(|version| {
                                view! {
                                    <option
                                        value=version.to_string()
                                        selected=move || selected.get() == Some(version)
                                    >
                                        {version}
                                    </option>
                                }
                            })
                            .collect_view()
                    }}
                </select>
            </div>
            <div class="w-64">
                <label class="block text-sm/6 font-medium text-white">"Race"</label>
                <select
                    class="select select-sm w-full bg-gray-800 text-gray-200 border border-white/20"
                    on:change=move |ev| navigation.race.set(event_target_value(&ev))
                >
                    <option value="" selected=move || navigation.race.get().is_empty()>"All races"</option>
                    {RACES
                        .iter()
                        .map(|race| view! {
                            <option value=*race selected=move || navigation.race.get() == *race>{*race}</option>
                        })
                        .collect_view()}
                </select>
            </div>
            {move || {
                navigation
                    .patch_version
                    .get()
                    .zip(selected.get())
                    .filter(|(requested, selected)| requested != selected)
                    .map(|(requested, selected)| view! {
                        <p class="text-xs text-gray-400">
                            {format!("Patch {requested} is not in the balance data, showing the closest patch {selected}.")}
                        </p>
                    })
            }}
            <ErrorMessage error />
            {move || {
                let race = navigation.race.get();
                changes.get().map(|mut changes| {
                    if !race.is_empty() {
                        changes.changes.retain(|change| change.race == race);
                    }
                    view! { <PatchChangesTable changes /> }
                })
            }}
            {move || {
                let race = navigation.race.get();
                let version = selected.get()?;
                let content = match usage.get()? {
                    Ok(usage) => {
                        let rows: Vec<AbilityPatchUsage> = usage
                            .of_patch(version)
                            .filter(|row| {
                                race.is_empty()
                                    || row.tracked().is_some_and(|tracked| tracked.race == race)
                            })
                            .cloned()
                            .collect();
                        view! { <AbilityUsageRows rows /> }.into_any()
                    }
                    Err(err) => view! {
                        <p class="text-xs text-gray-400">{format!("No snapshot to count the abilities: {err}")}</p>
                    }
                    .into_any(),
                };
                Some(view! {
                    <h2 class="pt-2 text-base font-semibold text-white">"Abilities in the games of the patch"</h2>
                    <p class="text-xs text-gray-500">
                        "The abilities with energy or a cooldown commanded by the players in the games of the snapshot of this patch, the games of a protocol version without balance data count for the closest patch."
                    </p>
                    {content}
                })
            }}
        </div>
    }
}

#[component]
fn PatchChangesTable(changes: PatchChanges) -> impl IntoView {
    let Some(previous_version) = changes.previous_version else {
        return view! {
            <p class="text-sm text-gray-400">
                {format!("{} is the oldest patch in the balance data, there is nothing to compare it with.", changes.version)}
            </p>
        }
        .into_any();
    };
    if changes.changes.is_empty() {
        return view! {
            <p class="text-sm text-gray-400">
                {format!("No unit changed from {previous_version} to {}.", changes.version)}
            </p>
        }
        .into_any();
    }
    view! {
        <p class="text-sm text-gray-300">
            {format!(
                "{} changes from {previous_version} to {}.",
                changes.changes.len(),
                changes.version,
            )}
        </p>
        <table class="min-w-full text-xs text-gray-400">
            <thead>
                <tr class="text-left text-gray-300">
                    <th class="pr-3">"Race"</th>
                    <th class="pr-3">"Unit"</th>
                    <th class="pr-3">"Property"</th>
                    <th class="pr-3">{previous_version}</th>
                    <th>{changes.version}</th>
                </tr>
            </thead>
            <tbody class="divide-y divide-white/5">
                {changes
                    .changes
                    .into_iter()
                    .map(|change| view! {
                        <tr>
                            <td class="pr-3 py-1"><RaceLink race=change.race /></td>
                            <td class="pr-3 py-1 text-white"><UnitLink unit_name=change.unit /></td>
                            <td class="pr-3 py-1">{change.label}</td>
                            <td class="pr-3 py-1">{change.previous}</td>
                            <td class="py-1 text-white">
                                <ChangeMarker kind=change.kind change=change.change />
                                {change.value}
                            </td>
                        </tr>
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
    .into_any()
}
