//! Swarmy Replay

use bevy::prelude::*;
use s2protocol::game_events::GameEvent;
use s2protocol::game_events::ReplayGameEvent;
use s2protocol::tracker_events::TrackerEvent;
use std::time::Duration;

pub mod assets;
pub mod camera;
pub mod unit;

pub use assets::*;
pub use camera::*;
pub use unit::*;

/// Current game scale
pub const GAME_SCALE: f32 = 100.;

/// It seems the gamespeed is 22.4 loops per second.
pub const GAME_LOOP_SPEED: f32 = 22.40f32;

/// The Players from the replay.
#[derive(Reflect, Component, Clone, Debug)]
#[reflect(Component)]
pub struct Player {
    pub id: u8,
    pub name: String,
    pub color: bevy::prelude::Color,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            id: 99,
            name: String::from("Unknown"),
            color: bevy::prelude::Color::WHITE,
        }
    }
}

#[derive(Resource, Debug)]
pub struct ReplayTimer {
    pub last_updated: i64,
    pub current: Timer,
}

#[derive(Component)]
pub struct Upkeep {
    pub current: usize,
    pub max: usize,
}

pub struct SC2ReplayPlugin;
impl Plugin for SC2ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplayTimer {
            last_updated: 0i64,
            current: Timer::new(Duration::from_millis(1_000), TimerMode::Repeating),
        })
        .add_startup_system(assets::asset_loading)
        .add_startup_system(assets::mpq_events::add_events)
        .add_system(unit::born::unit_born)
        .add_system(unit::death::unit_dead)
        .add_system(unit::movement::unit_move)
        .add_system(camera::save::camera_save)
        .add_system(camera::update::camera_update);
    }
}

#[derive(Resource, Component, PartialEq, Clone)]
pub enum GameEventType {
    Tracker(TrackerEvent),
    Game(GameEvent),
}

#[derive(Resource, Component)]
pub struct GameLoopEvent {
    processed: bool,
    game_loop: u64,
    evt: GameEventType,
}
