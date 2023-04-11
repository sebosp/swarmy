//! Initial unit management

use crate::*;
use colored::*;
use s2protocol::tracker_events::UnitBornEvent;

pub mod born;
pub mod death;
pub mod movement;

pub use born::*;
pub use death::*;
pub use movement::*;

pub mod props;
pub use props::*;

/// A unit that can spawn other units, the units
#[derive(Reflect, Component, Default, Clone)]
#[reflect(Component)]
pub struct UnitCreator {
    /// The unit tag index.
    idx: u32,
    /// The unit tag recycle
    recycle: u32,
}

/// A unit state, TODO: add burrowed
#[derive(Reflect, Component, Default, PartialEq, Clone)]
#[reflect(Component)]
pub enum UnitState {
    /// The unit is alive
    #[default]
    Alive,
    /// The unit has died.
    Dead,
}

/// A unit with state and ownership.
#[derive(Reflect, Component, Clone)]
#[reflect(Component)]
pub struct Unit {
    /// The unit tag index
    pub idx: u32,
    /// The unit tag recycle
    pub recycle: u32,
    /// The name of the unit
    pub name: String,
    /// The current unit position in x axis
    pub x: f32,
    /// The current unit position in y axis
    pub y: f32,
    /// The player in control of the unit.
    /// This can change ownership but the event is not handled yet.
    pub player: Player,
    /// The unit creator, for example a hatchery can spawn larva.
    /// TODO: If no creator of unit, we have a custom "System" unit with ID 9999
    pub creator: UnitCreator,
    /// The las ttime the unit was updated.
    pub last_game_loop: u64,
    /// The unit may have an activity counter where its size/color/etc may change to visualize
    /// there are changes on it.
    pub activity_counter: u32,
    /// The color of the Unit
    pub size: f32,
    /// The unit state
    pub state: UnitState,
    /// Last updated second
    pub last_updated: u64,
}

impl Unit {
    pub fn colored_term(&self, time: &Time) {
        let player = match self.player.id {
            1 => "1".blue(),
            2 => "2".yellow(),
            3 => "3".red(),
            4 => "3".purple(),
            _ => "_".white(),
        };
        tracing::error!(
            "{}[{:>3}] idx: {:>4}, pos: ({:>4},{:>4}), creat_idx:{} Name [{:>32}]",
            player,
            time.elapsed_seconds().floor(),
            self.idx.to_string().green(),
            self.x,
            self.y,
            self.creator.idx.to_string().green(),
            self.name,
        );
    }
}

impl From<UnitBornEvent> for Unit {
    fn from(evt: UnitBornEvent) -> Self {
        Unit {
            idx: evt.unit_tag_index,
            recycle: evt.unit_tag_recycle,
            name: evt.unit_type_name,
            x: evt.x as f32 / GAME_SCALE,
            y: evt.y as f32 / GAME_SCALE,
            player: Player {
                id: evt.control_player_id,
                ..Default::default()
            },
            creator: UnitCreator {
                // A unit that has no creator would default to this ID.
                // Probably a bad idea. sounds like an soon-to-be footgun.
                idx: evt.creator_unit_tag_index.unwrap_or(9999),
                recycle: evt.creator_unit_tag_recycle.unwrap_or(9999),
            },
            last_game_loop: 0u64,
            activity_counter: 0u32,
            size: 1f32,
            state: UnitState::Alive,
            last_updated: 0u64,
        }
    }
}

impl Default for Unit {
    fn default() -> Self {
        Self {
            idx: 0,
            name: String::from("Unknown"),
            ..default()
        }
    }
}
