//! The abilities that cost energy or have a cooldown, i.e. Abduct, Fungal Growth, Inject Larva or
//! Yamato Cannon, how often they are used in a replay compared to the games of the same patch.

use serde::{Deserialize, Serialize};

/// What limits how often an ability can be used.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum AbilityCost {
    #[default]
    Energy,
    Cooldown,
}

impl AbilityCost {
    pub fn label(self) -> &'static str {
        match self {
            AbilityCost::Energy => "Energy",
            AbilityCost::Cooldown => "Cooldown",
        }
    }
}

/// An ability whose uses are counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackedAbility {
    /// The ability and command of the game events, i.e. `Yoink.Execute`.
    pub ability: &'static str,
    /// The name in the game, i.e. `Abduct`.
    pub label: &'static str,
    /// The unit that casts it.
    pub unit: &'static str,
    /// Terran, Zerg or Protoss.
    pub race: &'static str,
    pub cost: AbilityCost,
}

const fn energy(
    ability: &'static str,
    label: &'static str,
    unit: &'static str,
    race: &'static str,
) -> TrackedAbility {
    TrackedAbility {
        ability,
        label,
        unit,
        race,
        cost: AbilityCost::Energy,
    }
}

const fn cooldown(
    ability: &'static str,
    label: &'static str,
    unit: &'static str,
    race: &'static str,
) -> TrackedAbility {
    TrackedAbility {
        ability,
        label,
        unit,
        race,
        cost: AbilityCost::Cooldown,
    }
}

/// The abilities with a target, the snapshot only has the commands targeting a point or a unit,
/// i.e. Guardian Shield or Stimpack are not in it.
pub const TRACKED_ABILITIES: [TrackedAbility; 42] = [
    energy("SpawnLarva.Execute", "Inject Larva", "Queen", "Zerg"),
    energy("Transfusion.Execute", "Transfuse", "Queen", "Zerg"),
    energy("Build.CreepTumorQueen", "Creep Tumor", "Queen", "Zerg"),
    energy("Yoink.Execute", "Abduct", "Viper", "Zerg"),
    energy("ViperConsumeStructure.Execute", "Consume", "Viper", "Zerg"),
    energy("BlindingCloud.Execute", "Blinding Cloud", "Viper", "Zerg"),
    energy("ParasiticBomb.Execute", "Parasitic Bomb", "Viper", "Zerg"),
    energy("FungalGrowth.Execute", "Fungal Growth", "Infestor", "Zerg"),
    energy(
        "NeuralParasite.Execute",
        "Neural Parasite",
        "Infestor",
        "Zerg",
    ),
    energy(
        "AmorphousArmorcloud.Execute",
        "Microbial Shroud",
        "Infestor",
        "Zerg",
    ),
    energy("Contaminate.Execute", "Contaminate", "Overseer", "Zerg"),
    cooldown(
        "RavagerCorrosiveBile.Execute",
        "Corrosive Bile",
        "Ravager",
        "Zerg",
    ),
    cooldown("CausticSpray.Execute", "Caustic Spray", "Corruptor", "Zerg"),
    cooldown(
        "SpawnLocustsTargeted.Execute",
        "Spawn Locusts",
        "SwarmHostMP",
        "Zerg",
    ),
    energy("CalldownMULE.Execute", "MULE", "OrbitalCommand", "Terran"),
    energy(
        "ScannerSweep.Execute",
        "Scanner Sweep",
        "OrbitalCommand",
        "Terran",
    ),
    energy(
        "SupplyDrop.Execute",
        "Extra Supplies",
        "OrbitalCommand",
        "Terran",
    ),
    energy("EMP.Execute", "EMP", "Ghost", "Terran"),
    energy(
        "ChannelSnipe.Execute",
        "Steady Targeting",
        "Ghost",
        "Terran",
    ),
    energy("BuildAutoTurret.Execute", "Auto-Turret", "Raven", "Terran"),
    energy(
        "RavenShredderMissile.Execute",
        "Anti-Armor Missile",
        "Raven",
        "Terran",
    ),
    energy(
        "RavenScramblerMissile.Execute",
        "Interference Matrix",
        "Raven",
        "Terran",
    ),
    cooldown("Yamato.Execute", "Yamato Cannon", "Battlecruiser", "Terran"),
    cooldown(
        "Hyperjump.Execute",
        "Tactical Jump",
        "Battlecruiser",
        "Terran",
    ),
    cooldown("KD8Charge.Execute", "KD8 Charge", "Reaper", "Terran"),
    cooldown("LockOn.Execute", "Lock On", "Cyclone", "Terran"),
    cooldown("TacNukeStrike.Execute", "Tactical Nuke", "Ghost", "Terran"),
    energy(
        "ChronoBoostEnergyCost.Execute",
        "Chrono Boost",
        "Nexus",
        "Protoss",
    ),
    energy(
        "EnergyRecharge.Execute",
        "Energy Recharge",
        "Nexus",
        "Protoss",
    ),
    energy(
        "PsiStorm.Execute",
        "Psionic Storm",
        "HighTemplar",
        "Protoss",
    ),
    energy("Feedback.Execute", "Feedback", "HighTemplar", "Protoss"),
    energy("ForceField.Execute", "Force Field", "Sentry", "Protoss"),
    energy(
        "OracleRevelation.Execute",
        "Revelation",
        "Oracle",
        "Protoss",
    ),
    energy("Build.OracleStasisTrap", "Stasis Ward", "Oracle", "Protoss"),
    energy(
        "GravitonBeam.Execute",
        "Graviton Beam",
        "Phoenix",
        "Protoss",
    ),
    energy(
        "TemporalField.Execute",
        "Time Warp",
        "Mothership",
        "Protoss",
    ),
    cooldown("Blink.Execute", "Blink", "Stalker", "Protoss"),
    cooldown(
        "AdeptPhaseShift.Execute",
        "Psionic Transfer",
        "Adept",
        "Protoss",
    ),
    cooldown(
        "NexusMassRecall.Execute",
        "Strategic Recall",
        "Nexus",
        "Protoss",
    ),
    cooldown("MassRecall.Execute", "Mass Recall", "Mothership", "Protoss"),
    cooldown(
        "DarkTemplarBlink.Execute",
        "Shadow Stride",
        "DarkTemplar",
        "Protoss",
    ),
    cooldown(
        "PurificationNovaTargeted.Execute",
        "Purification Nova",
        "Disruptor",
        "Protoss",
    ),
];

/// The tracked ability of a command, by its ability and command, i.e. `Yoink.Execute`.
pub fn tracked_ability(ability: &str) -> Option<&'static TrackedAbility> {
    TRACKED_ABILITIES
        .iter()
        .find(|tracked| tracked.ability == ability)
}

/// The tracked abilities of a race, i.e. `Zerg`.
pub fn race_abilities(race: &str) -> impl Iterator<Item = &'static TrackedAbility> + '_ {
    TRACKED_ABILITIES
        .iter()
        .filter(move |tracked| tracked.race == race)
}

/// A rank among others, 1 is the most uses.
#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct Rank {
    pub rank: usize,
    pub of: usize,
}

impl Rank {
    /// The share of the others ranked below or at the same place, i.e. `top 5%`.
    pub fn top_percent(&self) -> f64 {
        100. * self.rank as f64 / self.of.max(1) as f64
    }
}

/// Players in the top this many of the patch are shown as such.
pub const ABILITY_TOP_K: usize = 10;

/// The players need this many games of the race in the patch to be ranked.
pub const ABILITY_RANK_MIN_GAMES: usize = 3;

/// The use of an ability in the games of a patch, by the players of its race.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct AbilityPatchUsage {
    /// The balance data version closest to the protocol version of the games.
    pub patch: u32,
    /// [`TrackedAbility::ability`]
    pub ability: String,
    /// The games of each player of the race, one per player and game.
    pub player_games: usize,
    /// The player games in which the ability was used at least once.
    pub games_used: usize,
    pub uses: usize,
    /// The players that used it the most per game, with at least [`ABILITY_RANK_MIN_GAMES`].
    pub top_players: Vec<AbilityPlayerUsage>,
}

impl AbilityPatchUsage {
    pub fn per_game(&self) -> f64 {
        self.uses as f64 / self.player_games.max(1) as f64
    }

    /// The uses per game in the games it was used.
    pub fn per_game_used(&self) -> f64 {
        self.uses as f64 / self.games_used.max(1) as f64
    }

    pub fn used_percent(&self) -> f64 {
        100. * self.games_used as f64 / self.player_games.max(1) as f64
    }

    pub fn tracked(&self) -> Option<&'static TrackedAbility> {
        tracked_ability(&self.ability)
    }
}

/// How much a player used an ability in the games of its race of a patch.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct AbilityPlayerUsage {
    /// As stored in the replay, with the clan tag escaped.
    pub player_name: String,
    pub games: usize,
    pub uses: usize,
}

impl AbilityPlayerUsage {
    pub fn per_game(&self) -> f64 {
        self.uses as f64 / self.games.max(1) as f64
    }
}

/// The use of the tracked abilities in the games of the snapshot, by patch.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct AbilityUsageTable {
    /// The patches of the games, oldest first.
    pub patches: Vec<u32>,
    /// By patch and by the order of [`TRACKED_ABILITIES`].
    pub usage: Vec<AbilityPatchUsage>,
}

impl AbilityUsageTable {
    pub fn of_patch(&self, patch: u32) -> impl Iterator<Item = &AbilityPatchUsage> {
        self.usage.iter().filter(move |usage| usage.patch == patch)
    }

    pub fn get(&self, patch: u32, ability: &str) -> Option<&AbilityPatchUsage> {
        self.usage
            .iter()
            .find(|usage| usage.patch == patch && usage.ability == ability)
    }
}

/// The uses of an ability by a player in a replay, compared to the games of the same patch.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct AbilityUse {
    /// [`TrackedAbility::ability`]
    pub ability: String,
    pub uses: u32,
    /// The use in the games of the patch, none without games of the race in the snapshot.
    #[serde(default)]
    pub patch: Option<AbilityPatchUsage>,
    /// The rank of the uses of this game among the games of the race in the patch.
    #[serde(default)]
    pub game_rank: Option<Rank>,
    /// The rank of the player by uses per game in the patch, with at least
    /// [`ABILITY_RANK_MIN_GAMES`] games of the race.
    #[serde(default)]
    pub player_rank: Option<Rank>,
}

impl AbilityUse {
    pub fn tracked(&self) -> Option<&'static TrackedAbility> {
        tracked_ability(&self.ability)
    }

    pub fn is_top_player(&self) -> bool {
        self.player_rank
            .is_some_and(|rank| rank.rank <= ABILITY_TOP_K)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_finds_the_tracked_abilities() {
        assert_eq!(tracked_ability("Yoink.Execute").unwrap().label, "Abduct");
        assert_eq!(
            tracked_ability("Build.CreepTumorQueen").unwrap().unit,
            "Queen"
        );
        assert!(tracked_ability("Build.CreepTumor").is_none());
        assert!(tracked_ability("attack.Execute").is_none());
        assert!(race_abilities("Zerg").all(|tracked| tracked.race == "Zerg"));
        let mut abilities: Vec<&str> = TRACKED_ABILITIES.iter().map(|t| t.ability).collect();
        abilities.sort();
        abilities.dedup();
        assert_eq!(abilities.len(), TRACKED_ABILITIES.len());
    }

    #[test]
    fn it_computes_the_shares() {
        let usage = AbilityPatchUsage {
            player_games: 10,
            games_used: 4,
            uses: 20,
            ..Default::default()
        };
        assert_eq!(usage.per_game(), 2.);
        assert_eq!(usage.per_game_used(), 5.);
        assert_eq!(usage.used_percent(), 40.);
        assert_eq!(Rank { rank: 1, of: 20 }.top_percent(), 5.);
    }
}
