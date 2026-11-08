//! The StarCraft II community API of Battle.net: the profiles and ladders of the accounts.

use serde::{Deserialize, Serialize};

/// Whether the backend has Battle.net API credentials, the secret is never sent back.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct BattleNetStatus {
    pub configured: bool,
    /// Set by environment variables of the backend, not by Config.
    #[serde(default)]
    pub from_environment: bool,
    /// Battle.net accepted them just now.
    #[serde(default)]
    pub working: bool,
    /// Why they do not work, when they are set.
    #[serde(default)]
    pub error: String,
}

/// A ladder of the current season of an account.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct BattleNetLadder {
    /// i.e. `1v1 Master`.
    pub mode: String,
    /// i.e. `MASTER`.
    pub league: String,
    pub rank: u32,
    pub wins: u32,
    pub losses: u32,
    pub mmr: Option<i32>,
}

/// The Battle.net profile of an account (`region-realm-id`).
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct BattleNetProfile {
    pub account: String,
    pub display_name: String,
    pub clan_tag: String,
    /// The page of the profile on starcraft2.blizzard.com.
    pub profile_url: String,
    pub portrait_url: String,
    /// i.e. `GRANDMASTER`, achieved `best_1v1_times` times.
    pub best_1v1_league: String,
    pub best_1v1_times: u32,
    pub total_career_games: u32,
    pub games_this_season: u32,
    pub ladders: Vec<BattleNetLadder>,
}
