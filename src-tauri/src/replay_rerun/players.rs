//! The players of a replay and the ownership of their units.

use s2protocol::{SC2ReplayState, SC2Unit};
use swarmy_common::clean_player_name_clan_escapes;

/// The player name used for units without an owner, i.e. minerals, rocks.
pub const NEUTRAL_PLAYER: &str = "neutral";

/// The players from the replay details, indexed by tracker player_id - 1.
#[derive(Debug, Default, Clone)]
pub struct ReplayPlayers {
    /// The names without the clan tag escapes.
    pub names: Vec<String>,
    /// The names as stored in the replay, used to link to other views.
    pub raw_names: Vec<String>,
    /// RGBA
    pub colors: Vec<[u8; 4]>,
    pub teams: Vec<u8>,
}

impl ReplayPlayers {
    pub fn from_state(state: &SC2ReplayState) -> Self {
        let players = &state.details.player_list;
        Self {
            names: players
                .iter()
                .map(|player| clean_player_name_clan_escapes(&player.name))
                .collect(),
            raw_names: players.iter().map(|player| player.name.clone()).collect(),
            colors: players
                .iter()
                .map(|player| {
                    let color = &player.color;
                    [color.r, color.g, color.b, color.a]
                })
                .collect(),
            teams: players.iter().map(|player| player.team_id).collect(),
        }
    }

    fn index(&self, player: &str) -> Option<usize> {
        self.names.iter().position(|name| name == player)
    }

    /// Returns the player that owns the unit.
    pub fn unit_player(&self, unit: &SC2Unit) -> String {
        match (&unit.player_name, unit.user_id) {
            (Some(name), _) => clean_player_name_clan_escapes(name),
            // Units of computer players have no player name, only their control player id.
            (None, Some(player_id)) if (1..=self.names.len()).contains(&(player_id as usize)) => {
                self.tracker_player_name(player_id)
            }
            _ => NEUTRAL_PLAYER.to_string(),
        }
    }

    /// Returns the color the player had in the game, if it is a player.
    pub fn color(&self, player: &str) -> Option<[u8; 4]> {
        self.colors.get(self.index(player)?).copied()
    }

    /// Returns the name stored in the replay for a player, if it is a player.
    pub fn raw_name(&self, player: &str) -> Option<&str> {
        self.raw_names.get(self.index(player)?).map(String::as_str)
    }

    pub fn tracker_player_name(&self, player_id: u8) -> String {
        self.names
            .get((player_id as usize).wrapping_sub(1))
            .cloned()
            .unwrap_or_else(|| format!("P{player_id}"))
    }

    /// Returns true if the players are in different teams, neutral units are nobody's enemy.
    pub fn is_enemy(&self, player: &str, other: &str) -> bool {
        if player == NEUTRAL_PLAYER || other == NEUTRAL_PLAYER || player == other {
            return false;
        }
        let team = |name: &str| self.teams.get(self.index(name)?).copied();
        match (team(player), team(other)) {
            (Some(team), Some(other_team)) => team != other_team,
            _ => true,
        }
    }
}
