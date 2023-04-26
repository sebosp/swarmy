//! Starcraft 2 - Replay visualizer.
//!

use nom_mpq::{parser, MPQParserError, MPQ};
use rerun::components::{ColorRGBA, Vec3D};
use rerun::external::re_log_types::DataTableError;
use rerun::external::re_viewer::external::eframe::Error as eframe_Error;
use rerun::time::Timeline;
use rerun::Session;
use rerun::{time, MsgSenderError};
use s2protocol::game_events::ReplayGameEvent;
use s2protocol::tracker_events::ReplayTrackerEvent;
use s2protocol::versions::read_game_events;
use s2protocol::versions::read_tracker_events;
use std::collections::HashMap;
pub use tracker_events::*;
pub mod unit_colors;
pub use unit_colors::*;
pub mod game_events;
pub use game_events::*;
pub mod tracker_events;

/// The game events seem to be at this ratio when compared to Tracker Events.
pub const GAME_EVENT_POS_RATIO: f32 = 4096f32;

// Some colors I really liked from a Freya Holmer presentation:
// https://www.youtube.com/watch?v=kfM-yu0iQBk
pub const FREYA_ORANGE: ColorRGBA = ColorRGBA(0xeb790700);
pub const FREYA_GOLD: ColorRGBA = ColorRGBA(0xea9e3600);
pub const FREYA_RED: ColorRGBA = ColorRGBA(0xf8105300);
pub const FREYA_BLUE: ColorRGBA = ColorRGBA(0x30b5f700);
pub const FREYA_GREEN: ColorRGBA = ColorRGBA(0x0aeb9f00);
pub const FREYA_LIGHT_BLUE: ColorRGBA = ColorRGBA(0x72c5dd00);
pub const FREYA_GRAY: ColorRGBA = ColorRGBA(0xb2c5c500);
pub const FREYA_PINK: ColorRGBA = ColorRGBA(0xeaa48300);
pub const FREYA_LIGHT_GRAY: ColorRGBA = ColorRGBA(0xf4f5f800);
pub const FREYA_DARK_BLUE: ColorRGBA = ColorRGBA(0x4da7c200);
pub const FREYA_DARK_GREEN: ColorRGBA = ColorRGBA(0x37bda900);
pub const FREYA_DARK_RED: ColorRGBA = ColorRGBA(0xae204400);
pub const FREYA_VIOLET: ColorRGBA = ColorRGBA(0xa401ed00);
pub const FREYA_WHITE: ColorRGBA = ColorRGBA(0xfaf8fb00);
pub const FREYA_YELLOW: ColorRGBA = ColorRGBA(0xf7d45400);
pub const FREYA_LIGHT_YELLOW: ColorRGBA = ColorRGBA(0xead8ad00);
pub const FREYA_LIGHT_GREEN: ColorRGBA = ColorRGBA(0x6ec29c00);

// This was observed in a game with max game_loop = 13735 and a duration of 15:42 = 942 seconds.
// nanos 942000000000 / 13735 game_loops = 68583909 nanoseconds per game_loop
pub const GAME_LOOP_SPEED_NANOS: i64 = 68_583_909;

pub const TRACKER_SPEED_RATIO: f32 = 0.70996;

// priarity of events, to sort them when they are at the same game loop.
// In this version, the game_loop will be multiplied by 10 and added the priority.
// This means 10 max events are supported.
pub const TRACKER_PRIORITY: i64 = 1;
pub const GAME_PRIORITY: i64 = 2;

// These many event types (replays, game, attributes, etc) are supported.
// This should be the real number, but for it's just 10 to help debugging.
pub const MAX_EVENT_TYPES: i64 = 10;

/// Supported event types.
#[derive(Debug, Clone)]
pub enum SC2EventType {
    Tracker {
        tracker_loop: i64,
        event: ReplayTrackerEvent,
    },
    Game {
        game_loop: i64,
        user_id: i64,
        event: ReplayGameEvent,
    },
}

/// Reads the MPQ file and returns both the MPQ read file and the
pub fn read_mpq(path: &str) -> (MPQ, Vec<u8>) {
    tracing::info!("Processing MPQ file {}", path);
    let file_contents = parser::read_file(path);
    let (_, mpq) = parser::parse(&file_contents).unwrap();
    (mpq, file_contents)
}

#[derive(thiserror::Error, Debug)]
pub enum SwarmyError {
    #[error("Rerun Message Sender error")]
    RerunMsgSender(#[from] MsgSenderError),
    #[error("Rerun Data Table Error")]
    RerunDataTable(#[from] DataTableError),
    #[error("Rerun Eframe Error")]
    RerunEframe(#[from] eframe_Error),
    #[error("MPQ Error")]
    MPQ(#[from] MPQParserError),
}

/// Unit Attributes.
#[derive(Debug, Default)]
pub struct SC2Unit {
    /// The last time the unit was updated
    pub last_game_loop: i64,
    /// The owner user_id
    pub user_id: Option<u8>,
    /// The name of the unit.
    pub name: String,
    /// The XYZ position.
    pub pos: Vec3D,
    /// The target of this unit.
    pub target: Option<Vec3D>,
    /// The game loop in which the unit was created.
    pub init_game_loop: i64,
    /// The creator ability name.
    pub creator_ability_name: Option<String>,
    /// The radius of the unit.
    pub radius: f32,
    /// Whether the unit is selected
    pub is_selected: bool,
}

/// The currently selected units is stored as a group outside of the boundaries of the usable
/// groups.
pub const ACTIVE_UNITS_GROUP_IDX: usize = 10usize;

/// The user state as it's collected through time.
#[derive(Debug, Default, Clone)]
pub struct SC2UserState {
    /// An array of registered control groups per user, the control group indexed as 10th is the
    /// currently selected units.
    pub control_groups: Vec<Vec<u32>>,
}

impl SC2UserState {
    pub fn new() -> Self {
        let mut control_groups = vec![];
        // populate as empty control groups.
        for _ in 0..11 {
            control_groups.push(vec![]);
        }
        Self { control_groups }
    }
}

/// A set of filters to apply to the rerun session.
#[derive(Debug, Default, Clone)]
pub struct SC2ReplayFilters {
    /// Filters a specific user id.
    pub user_id: Option<i64>,

    /// Filters a specific unit tag.
    pub unit_tag: Option<i64>,

    /// Allows setting up a min event loop, in game_event units
    pub min_loop: Option<i64>,

    /// Allows setting up a max event loop
    pub max_loop: Option<i64>,

    /// Only show game of specific types
    pub event_type: Option<String>,

    /// Only show game of specific types
    pub unit_name: Option<String>,

    /// Allows setting up a max number of events of each type
    pub max_events: Option<usize>,
}

pub struct SC2Rerun {
    /// The registered units state as they change through time.
    /// These are with unit index as reference
    pub units: HashMap<u32, SC2Unit>,

    /// The absolute GameEvevnt loop timeline, the tracker loop should be relative to it.
    pub timeline: Timeline,

    /// The rerun session to display data.
    pub rerun_session: Session,

    /// The MPQ file being read.
    pub mpq: MPQ,

    /// The contents of the file
    pub file_contents: Vec<u8>,

    /// The filters to be applied to the collection.
    pub filters: SC2ReplayFilters,

    /// Whether or not the PlayerStats event should be shown. To be replaced by a proper filter
    pub include_stats: bool,

    /// The per-user state, the control groups, the supply, units, upgrades, as it progresses
    /// through time.
    pub user_state: HashMap<i64, SC2UserState>,
}
impl SC2Rerun {
    pub fn new(file_path: &str) -> Result<Self, SwarmyError> {
        let rerun_session = rerun::SessionBuilder::new("swarmy-rerun").buffered();
        let (mpq, file_contents) = read_mpq(file_path);
        let timeline = rerun::time::Timeline::new("game_timeline", time::TimeType::Sequence);
        Ok(Self {
            units: HashMap::new(),
            timeline,
            rerun_session,
            mpq,
            file_contents,
            filters: SC2ReplayFilters::default(),
            include_stats: false,
            user_state: HashMap::new(),
        })
    }

    pub fn add_events(&mut self) -> Result<usize, SwarmyError> {
        let filter_event_type = &self.filters.event_type.clone();
        let tracker_events = if let Some(event_type) = filter_event_type {
            if event_type.clone().to_lowercase().contains("tracker") {
                read_tracker_events(&self.mpq, &self.file_contents)
            } else {
                vec![]
            }
        } else {
            read_tracker_events(&self.mpq, &self.file_contents)
        };
        let mut sc2_events: HashMap<i64, Vec<SC2EventType>> = HashMap::new();
        let mut tracker_loop = 0i64;
        for game_step in tracker_events {
            tracker_loop += game_step.delta as i64;
            let adjusted_loop = (tracker_loop as f32 / TRACKER_SPEED_RATIO) as i64
                * MAX_EVENT_TYPES
                + TRACKER_PRIORITY;
            if let Some(step_evt) = sc2_events.get_mut(&adjusted_loop) {
                step_evt.push(SC2EventType::Tracker {
                    tracker_loop: (tracker_loop as f32 / TRACKER_SPEED_RATIO) as i64,
                    event: game_step.event,
                });
            } else {
                sc2_events.insert(
                    adjusted_loop,
                    vec![SC2EventType::Tracker {
                        tracker_loop: (tracker_loop as f32 / TRACKER_SPEED_RATIO) as i64,
                        event: game_step.event,
                    }],
                );
            }
        }
        let game_events = if let Some(event_type) = filter_event_type {
            if event_type.clone().to_lowercase().contains("game") {
                read_game_events(&self.mpq, &self.file_contents)
            } else {
                vec![]
            }
        } else {
            read_game_events(&self.mpq, &self.file_contents)
        };
        let mut game_loop = 0i64;
        for game_step in game_events {
            game_loop += game_step.delta as i64;
            let adjusted_loop = game_loop * MAX_EVENT_TYPES + GAME_PRIORITY;
            if let Some(step_evt) = sc2_events.get_mut(&adjusted_loop) {
                step_evt.push(SC2EventType::Game {
                    game_loop,
                    user_id: game_step.user_id,
                    event: game_step.event,
                });
            } else {
                sc2_events.insert(
                    adjusted_loop,
                    vec![SC2EventType::Game {
                        game_loop,
                        user_id: game_step.user_id,
                        event: game_step.event,
                    }],
                );
            }
        }
        let mut total_events = 0usize;
        let min_filter = self.filters.min_loop.clone();
        let max_filter = self.filters.max_loop.clone();
        let user_id_filter = self.filters.user_id.clone();
        let max_events = self.filters.max_events.clone();
        let mut ordered_event_loops: Vec<i64> = sc2_events.keys().map(|v| v.clone()).collect();
        ordered_event_loops.sort_unstable();
        for evt_loop in ordered_event_loops {
            for evt_type in sc2_events.get(&evt_loop).unwrap() {
                if let Some(min) = min_filter {
                    // Skip the events less than the requested filter.
                    if evt_loop / MAX_EVENT_TYPES < min {
                        continue;
                    }
                }
                if let Some(max) = max_filter {
                    // Skip the events greater than the requested filter.
                    if evt_loop / MAX_EVENT_TYPES > max {
                        break;
                    }
                }
                if let Some(max) = max_events {
                    // Cosue these max total events of any type.
                    if total_events > max {
                        break;
                    }
                }
                match evt_type {
                    SC2EventType::Tracker {
                        tracker_loop,
                        event,
                    } => {
                        tracing::info!("Trac [{:>08}]: {:?}", tracker_loop, event);
                        add_tracker_event(self, *tracker_loop, event)?;
                    }
                    SC2EventType::Game {
                        user_id,
                        game_loop,
                        event,
                    } => {
                        if let Some(target_user_id) = user_id_filter {
                            // Skip the events that are not for the requested user.
                            if target_user_id != *user_id {
                                continue;
                            }
                        }
                        tracing::info!("Game [{:>08}]: uid: {} {:?}", game_loop, *user_id, event);
                        add_game_event(self, *game_loop, *user_id, event)?;
                    }
                }
            }
            total_events += 1;
        }
        Ok(total_events)
    }

    pub fn show(&self) -> Result<(), SwarmyError> {
        Ok(rerun::native_viewer::show(&self.rerun_session)?)
    }

    pub fn with_filters(&mut self, filters: SC2ReplayFilters) {
        self.filters = filters;
    }

    /// Sets the include_stats value to true,
    pub fn include_stats(&mut self) {
        self.include_stats = true;
    }
}
