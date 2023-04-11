//! Camera Hotkeys maybe?

use crate::*;

pub mod save;
pub use save::*;
pub mod update;
pub use update::*;

pub const CAMERA_SCALE: f32 = 27_000f32;

/// A unit with state and ownership.
#[derive(Reflect, Component, Clone, Default, Debug)]
#[reflect(Component)]
struct PlayerCamera {
    /// 3 bits stored for m_which, maybe which one of the 8(?) cameras?
    pub which: Option<i64>,
    /// The current camera position in x axis
    pub x: f32,
    /// The current camera position in y axis
    pub y: f32,
    /// The player set the camera position.
    pub player: Player,
    /// The las ttime the unit was updated.
    pub last_game_loop: u64,
    /// Last updated second
    pub last_updated: u64,
}

impl PlayerCamera {
    pub fn try_from_camera_save(source: &GameEventType) -> Result<Self, String> {
        let game_step = match source {
            GameEventType::Game(val) => val,
            _ => return Err(String::from("GameEventType is not Game related.")),
        };
        match &game_step.event {
            ReplayGameEvent::CameraSave(evt) => Ok(Self {
                which: Some(evt.m_which),
                x: evt.m_target.x as f32 / CAMERA_SCALE,
                y: evt.m_target.y as f32 / CAMERA_SCALE,
                player: Player {
                    id: game_step.user_id as u8,
                    ..Default::default()
                },
                last_game_loop: 0u64,
                last_updated: 0u64,
            }),
            _ => Err(String::from("ReplayGameEvent is not CameraSave.")),
        }
    }

    pub fn try_from_camera_update(source: &GameEventType) -> Result<Self, String> {
        let game_step = match source {
            GameEventType::Game(val) => val,
            _ => return Err(String::from("GameEventType is not Game related.")),
        };
        match &game_step.event {
            ReplayGameEvent::CameraUpdate(evt) => {
                let target = match &evt.m_target {
                    Some(val) => val,
                    None => return Err(String::from("CameraUpdateEvent has no target.")),
                };
                Ok(Self {
                    which: None,
                    x: target.x as f32 / CAMERA_SCALE,
                    y: target.y as f32 / CAMERA_SCALE,
                    player: Player {
                        id: game_step.user_id as u8,
                        ..Default::default()
                    },
                    last_game_loop: 0u64,
                    last_updated: 0u64,
                })
            }
            _ => Err(String::from("ReplayGameEvent is not CameraUpdate.")),
        }
    }
}

pub fn player_camera_material(step_user_id: u8) -> StandardMaterial {
    StandardMaterial {
        emissive: match step_user_id {
            0 => bevy::prelude::Color::RED,
            1 => bevy::prelude::Color::GREEN,
            2 => bevy::prelude::Color::BLUE,
            _ => bevy::prelude::Color::YELLOW,
        },
        ..default()
    }
}
