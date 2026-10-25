//! The units of a player and the enemy units they killed.

use crate::components::links::UnitLink;
use crate::format_resources;
use leptos::prelude::*;
use swarmy_common::UnitEffectiveness;

/// The unit with the most kills per unit, among the ones produced at least `min_produced` times.
pub fn most_effective_unit(units: &[UnitEffectiveness], min_produced: usize) -> Option<String> {
    units
        .iter()
        .filter(|unit| unit.produced >= min_produced)
        .max_by(|a, b| a.kills_per_unit().total_cmp(&b.kills_per_unit()))
        .map(|unit| format!("{}, {:.2} kills per unit", unit.unit, unit.kills_per_unit()))
}

/// The unit that killed the most value for the resources invested in it.
pub fn best_trading_unit(units: &[UnitEffectiveness], min_produced: usize) -> Option<String> {
    units
        .iter()
        .filter(|unit| unit.produced >= min_produced)
        .filter_map(|unit| Some((unit, unit.trade_ratio()?)))
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(unit, ratio)| format!("{}, killed {ratio:.2}x its cost", unit.unit))
}

/// The units sorted by kills, with the most effective one first.
#[component]
pub fn UnitsTable(
    units: Vec<UnitEffectiveness>,
    /// Units produced less than this are not considered for the most effective unit.
    min_produced: usize,
    /// The rows shown, all if None.
    #[prop(optional)]
    max_rows: Option<usize>,
) -> impl IntoView {
    let most_effective = most_effective_unit(&units, min_produced);
    let best_trading = best_trading_unit(&units, min_produced);
    view! {
        <p class="text-xs text-gray-300">
            "Most kills per unit: "
            <span class="text-green-400">{most_effective.unwrap_or_else(|| "-".to_string())}</span>
        </p>
        <p class="text-xs text-gray-300" title="The minerals and vespene of the enemy units killed, divided by the minerals and vespene spent on the unit">
            "Best trade: "
            <span class="text-green-400">{best_trading.unwrap_or_else(|| "-".to_string())}</span>
        </p>
        <table class="min-w-full text-xs text-gray-400">
            <thead>
                <tr class="text-left text-gray-300">
                    <th class="pr-2">"Unit"</th>
                    <th class="pr-2">"Produced"</th>
                    <th class="pr-2">"Kills"</th>
                    <th class="pr-2">"Kills/unit"</th>
                    <th class="pr-2" title="The minerals and vespene of the enemy units killed">"Value killed"</th>
                    <th title="The value killed for each mineral and vespene spent on the unit">"Trade"</th>
                </tr>
            </thead>
            <tbody>
                {units
                    .iter()
                    .take(max_rows.unwrap_or(usize::MAX))
                    .map(|unit| {
                        let trade = unit.trade_ratio();
                        view! {
                            <tr>
                                <td class="pr-2 text-white"><UnitLink unit_name=unit.unit.clone() /></td>
                                <td class="pr-2">{unit.produced}</td>
                                <td class="pr-2">{unit.kills}</td>
                                <td class="pr-2">{format!("{:.2}", unit.kills_per_unit())}</td>
                                <td class="pr-2">{format_resources(unit.value_killed)}</td>
                                <td
                                    class:text-green-400=trade.is_some_and(|trade| trade >= 1.)
                                    class:text-red-400=trade.is_some_and(|trade| trade < 1.)
                                >
                                    {trade.map(|trade| format!("{trade:.2}x")).unwrap_or_else(|| "-".to_string())}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}
