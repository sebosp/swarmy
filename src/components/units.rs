//! The units of a player and the enemy units they killed.

use crate::components::links::UnitLink;
use crate::format_resources;
use leptos::prelude::*;
use swarmy_common::UnitEffectiveness;

/// The units the player really builds: produced at least `min_produced` times and at least
/// [`SHARE_OF_PRODUCTION`] of everything they produced. A Zerg with a handful of Terran games
/// built 18 Siege Tanks with 215 kills: more kills per unit than their 78,000 Lurkers, and not
/// their unit.
fn usual_units(
    units: &[UnitEffectiveness],
    min_produced: usize,
) -> impl Iterator<Item = &UnitEffectiveness> {
    let total: usize = units.iter().map(|unit| unit.produced).sum();
    let min_produced = min_produced.max(total / SHARE_OF_PRODUCTION);
    units
        .iter()
        .filter(move |unit| unit.produced >= min_produced)
}

/// One in this many units produced: 0.5 % of the production.
const SHARE_OF_PRODUCTION: usize = 200;

/// The unit with the most kills per unit, among the ones the player usually builds.
pub fn most_effective_unit(units: &[UnitEffectiveness], min_produced: usize) -> Option<String> {
    usual_units(units, min_produced)
        .max_by(|a, b| a.kills_per_unit().total_cmp(&b.kills_per_unit()))
        .map(|unit| format!("{}, {:.2} kills per unit", unit.unit, unit.kills_per_unit()))
}

/// The unit that killed the most value for the resources invested in it, among the ones the
/// player usually builds.
pub fn best_trading_unit(units: &[UnitEffectiveness], min_produced: usize) -> Option<String> {
    usual_units(units, min_produced)
        .filter_map(|unit| Some((unit, unit.trade_ratio()?)))
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(unit, ratio)| format!("{}, killed {ratio:.2}x its cost", unit.unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(name: &str, produced: usize, kills: usize) -> UnitEffectiveness {
        UnitEffectiveness {
            unit: name.to_string(),
            produced,
            kills,
            ..Default::default()
        }
    }

    #[test]
    fn a_unit_of_a_few_games_of_another_race_is_not_the_most_effective() {
        let units = [
            unit("Lurker", 78_018, 632_196),
            unit("Ultralisk", 4_607, 29_926),
            // 11.9 kills per unit, but 18 of 82,643 units.
            unit("SiegeTank", 18, 215),
        ];
        assert_eq!(
            most_effective_unit(&units, 10),
            Some("Lurker, 8.10 kills per unit".to_string())
        );
    }

    #[test]
    fn a_player_with_few_units_keeps_the_minimum() {
        let units = [unit("Marine", 40, 20), unit("SiegeTank", 12, 60)];
        assert_eq!(
            most_effective_unit(&units, 10),
            Some("SiegeTank, 5.00 kills per unit".to_string())
        );
    }
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
        <p class="text-xs text-muted" title="Among the units the player usually builds: at least 0.5% of everything they produced">
            "Most kills per unit: "
            <span class="text-success">{most_effective.unwrap_or_else(|| "-".to_string())}</span>
        </p>
        <p class="text-xs text-muted" title="The minerals and vespene of the enemy units killed, divided by the minerals and vespene spent on the unit">
            "Best trade: "
            <span class="text-success">{best_trading.unwrap_or_else(|| "-".to_string())}</span>
        </p>
        <table class="min-w-full text-xs text-muted">
            <thead>
                <tr class="text-left text-muted">
                    <th class="pr-2">"Unit"</th>
                    <th class="pr-2 num">"Produced"</th>
                    <th class="pr-2 num">"Kills"</th>
                    <th class="pr-2 num">"Kills/unit"</th>
                    <th class="pr-2 num" title="The minerals and vespene of the enemy units killed">"Value killed"</th>
                    <th class="num" title="The value killed for each mineral and vespene spent on the unit">"Trade"</th>
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
                                <td class="pr-2 text-base-content"><UnitLink unit_name=unit.unit.clone() /></td>
                                <td class="pr-2 num">{unit.produced}</td>
                                <td class="pr-2 num">{unit.kills}</td>
                                <td class="pr-2 num">{format!("{:.2}", unit.kills_per_unit())}</td>
                                <td class="pr-2 num">{format_resources(unit.value_killed)}</td>
                                <td
                                    class="num"
                                    class:text-success=trade.is_some_and(|trade| trade >= 1.)
                                    class:text-error=trade.is_some_and(|trade| trade < 1.)
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
