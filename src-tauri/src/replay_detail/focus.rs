//! What the player pays attention to, from the units selected and commanded, to find players
//! that only control the army and forget to macro, expand or upgrade.

use super::data::{EXPANSION_TOWNHALLS, TOWNHALLS};
use crate::replay_rerun::decisions::{base_unit_name, WORKERS};
use s2protocol::game_events::VersionedBalanceUnits;
use s2protocol::SC2Unit;
use swarmy_common::*;

/// The structures that research upgrades, selecting them is paying attention to upgrades.
const RESEARCH_STRUCTURES: [&str; 23] = [
    "EngineeringBay",
    "Armory",
    "BarracksTechLab",
    "FactoryTechLab",
    "StarportTechLab",
    "FusionCore",
    "GhostAcademy",
    "Forge",
    "CyberneticsCore",
    "TwilightCouncil",
    "RoboticsBay",
    "FleetBeacon",
    "TemplarArchive",
    "DarkShrine",
    "EvolutionChamber",
    "RoachWarren",
    "BanelingNest",
    "HydraliskDen",
    "LurkerDenMP",
    "InfestationPit",
    "Spire",
    "GreaterSpire",
    "UltraliskCavern",
];

/// The abilities that are macro regardless of the unit using them, i.e. a Queen injecting.
const MACRO_ABILITIES: [&str; 9] = [
    "Train",
    "WarpGateTrain",
    "LarvaTrain",
    "SpawnLarva",
    "ChronoBoost",
    "CalldownMULE",
    "SupplyDrop",
    "Rally",
    "UpgradeTo",
];

/// The time without macro actions is only measured after the opening, when the army is out.
const ARMY_ONLY_AFTER_SECONDS: u32 = 120;

fn is_structure(balance: &VersionedBalanceUnits, name: &str) -> bool {
    balance.get(name).is_some_and(|unit| {
        unit.attributes
            .inner
            .iter()
            .any(|attribute| attribute.kind.as_deref() == Some("Structure"))
    })
}

/// The focus of a single unit, the selections and commands use the most relevant of their units.
fn unit_focus(balance: &VersionedBalanceUnits, name: &str) -> FocusKind {
    let name = base_unit_name(name);
    if WORKERS.contains(&name) {
        FocusKind::Economy
    } else if RESEARCH_STRUCTURES.contains(&name) {
        FocusKind::Upgrades
    } else if TOWNHALLS.contains(&name)
        || matches!(name, "Larva" | "Egg")
        || is_structure(balance, name)
    {
        FocusKind::Macro
    } else {
        FocusKind::Army
    }
}

/// Classifies a selection or a command, `ability` is empty for selections.
/// An army unit in a selection makes it an army action, so that a selection of the army and a
/// worker that followed it is still the army.
pub fn classify_action(
    units: &[SC2Unit],
    ability: &str,
    balance: &VersionedBalanceUnits,
) -> Option<FocusKind> {
    let (ability_name, command) = ability.split_once('.').unwrap_or((ability, ""));
    if ability_name.contains("Research") {
        return Some(FocusKind::Upgrades);
    }
    if ability_name.ends_with("Build") || ability_name == "Build" {
        return Some(if EXPANSION_TOWNHALLS.contains(&command) {
            FocusKind::Expand
        } else {
            FocusKind::Macro
        });
    }
    if MACRO_ABILITIES
        .iter()
        .any(|macro_ability| ability_name.starts_with(macro_ability))
    {
        return Some(FocusKind::Macro);
    }
    let kinds: Vec<FocusKind> = units
        .iter()
        .map(|unit| unit_focus(balance, &unit.name))
        .collect();
    [
        FocusKind::Army,
        FocusKind::Upgrades,
        FocusKind::Macro,
        FocusKind::Economy,
    ]
    .into_iter()
    .find(|kind| kinds.contains(kind))
}

/// Collects the actions of a player.
#[derive(Debug, Default)]
pub struct FocusAccumulator {
    per_minute: Vec<[u32; 5]>,
    /// The start of the current time without macro actions and the last army action in it.
    army_only: Option<TimeRange>,
    longest_army_only: Option<TimeRange>,
}

impl FocusAccumulator {
    pub fn add(&mut self, seconds: u32, kind: FocusKind) {
        let minute = (seconds / 60) as usize;
        if self.per_minute.len() <= minute {
            self.per_minute.resize(minute + 1, [0; 5]);
        }
        self.per_minute[minute][kind.index()] += 1;
        match kind {
            FocusKind::Army if seconds >= ARMY_ONLY_AFTER_SECONDS => {
                let range = self.army_only.get_or_insert(TimeRange {
                    start: seconds,
                    end: seconds,
                });
                range.end = seconds;
            }
            FocusKind::Macro | FocusKind::Expand | FocusKind::Upgrades => {
                self.end_army_only(seconds);
            }
            _ => {}
        }
    }

    /// A macro action ends the time without macro, it lasted until this action.
    fn end_army_only(&mut self, seconds: u32) {
        if let Some(mut range) = self.army_only.take() {
            range.end = seconds.max(range.end);
            let longest = self
                .longest_army_only
                .is_none_or(|longest| range.end - range.start > longest.end - longest.start);
            if longest {
                self.longest_army_only = Some(range);
            }
        }
    }

    pub fn summary(mut self, game_seconds: u32) -> PlayerFocus {
        self.end_army_only(game_seconds);
        let mut totals = [0; 5];
        for minute in &self.per_minute {
            for (total, actions) in totals.iter_mut().zip(minute) {
                *total += actions;
            }
        }
        PlayerFocus {
            per_minute: self.per_minute,
            totals,
            longest_army_only: self.longest_army_only,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay_rerun::decisions::latest_balance_units;

    fn units(names: &[&str]) -> Vec<SC2Unit> {
        names
            .iter()
            .map(|name| SC2Unit {
                name: name.to_string(),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn it_classifies_actions() {
        let balance = latest_balance_units().unwrap();
        let classify =
            |names: &[&str], ability: &str| classify_action(&units(names), ability, balance);
        assert_eq!(classify(&["Marine", "SCV"], ""), Some(FocusKind::Army));
        assert_eq!(
            classify(&["CommandCenter", "SCV"], ""),
            Some(FocusKind::Macro)
        );
        assert_eq!(
            classify(&["SCV"], "SCVHarvest.Gather"),
            Some(FocusKind::Economy)
        );
        assert_eq!(classify(&["Probe"], "Build.Nexus"), Some(FocusKind::Expand));
        assert_eq!(classify(&["Probe"], "Build.Pylon"), Some(FocusKind::Macro));
        assert_eq!(
            classify(&["Queen"], "SpawnLarva.Execute"),
            Some(FocusKind::Macro)
        );
        assert_eq!(classify(&["Forge"], ""), Some(FocusKind::Upgrades));
        assert_eq!(classify(&[], ""), None);
    }

    #[test]
    fn it_measures_the_longest_time_without_macro() {
        let mut focus = FocusAccumulator::default();
        focus.add(130, FocusKind::Army);
        focus.add(150, FocusKind::Army);
        focus.add(160, FocusKind::Macro);
        focus.add(200, FocusKind::Army);
        focus.add(260, FocusKind::Economy);
        focus.add(300, FocusKind::Army);
        focus.add(330, FocusKind::Upgrades);
        let summary = focus.summary(400);
        assert_eq!(
            summary.longest_army_only,
            Some(TimeRange {
                start: 200,
                end: 330
            })
        );
        assert_eq!(summary.totals, [4, 1, 1, 0, 1]);
        assert_eq!(summary.per_minute.len(), 6);
    }
}
