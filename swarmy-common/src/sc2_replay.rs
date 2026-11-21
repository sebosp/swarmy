//! A replay played in StarCraft II and driven by swarmy (`src-tauri/src/sc2_game.rs`): its state
//! for the overlay, and what the controls of the overlay ask.

use serde::{Deserialize, Serialize};

/// The game loops of a second of game time (the "Faster" speed of the ladder).
pub const LOOPS_PER_SECOND: f32 = 22.4;

/// The speeds the overlay offers, 1 is the speed of a game on the ladder.
pub const SC2_REPLAY_SPEEDS: [f32; 6] = [0.5, 1., 2., 4., 8., 16.];

/// The replay StarCraft II plays for swarmy, `active` while the game is open.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct Sc2ReplayState {
    pub active: bool,
    pub replay_file_name: String,
    /// The replay in the snapshot, for the overlay's statistics; None for a file outside of it.
    pub ext_fs_id: Option<u64>,
    pub game_loop: u32,
    pub duration_loops: u32,
    pub playing: bool,
    pub speed: f32,
    /// The second being jumped to: the game computes the way there, it takes a while.
    pub seeking: Option<u32>,
    pub ended: bool,
    /// What went wrong last, empty otherwise.
    pub message: String,
}

impl Sc2ReplayState {
    pub fn seconds(&self) -> u32 {
        (self.game_loop as f32 / LOOPS_PER_SECOND) as u32
    }

    pub fn duration_seconds(&self) -> u32 {
        (self.duration_loops as f32 / LOOPS_PER_SECOND) as u32
    }
}

/// What the controls of the overlay ask of the replay.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Sc2ReplayControl {
    Play,
    Pause,
    Speed {
        speed: f32,
    },
    /// Jumps to a second: forward the game computes the way there, backward it starts again.
    Seek {
        seconds: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_counts_the_seconds_of_the_game() {
        let state = Sc2ReplayState {
            game_loop: 224,
            duration_loops: 2240,
            ..Default::default()
        };
        assert_eq!((state.seconds(), state.duration_seconds()), (10, 100));
        let control: Sc2ReplayControl =
            serde_json::from_str(r#"{"action":"seek","seconds":90}"#).unwrap();
        assert_eq!(control, Sc2ReplayControl::Seek { seconds: 90 });
    }
}
