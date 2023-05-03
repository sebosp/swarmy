//! Starcraft 2 - Replay visualizer.
//!

use nom_mpq::MPQParserError;
use rerun::components::{ColorRGBA, Point3D};
use rerun::external::re_log_types::DataTableError;
use rerun::external::re_viewer::external::eframe::Error as eframe_Error;
use rerun::time::Timeline;
use rerun::Session;
use rerun::{time, MsgSenderError};
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

pub struct SC2Rerun {
    /// The absolute GameEvevnt loop timeline, the tracker loop should be relative to it.
    pub timeline: Timeline,

    /// The rerun session to display data.
    pub rerun_session: Session,

    /// The SC2 Replay SC2UserState
    sc2_state: SC2ReplayState,
}
impl SC2Rerun {
    pub fn new(file_path: &str) -> Result<Self, SwarmyError> {
        let rerun_session = rerun::SessionBuilder::new(file_path).buffered();
        let timeline = rerun::time::Timeline::new("game_timeline", time::TimeType::Sequence);
        Ok(Self {
            timeline,
            rerun_session,
            sc2_state: SC2ReplayState::new(file_path)?,
        })
    }

    pub fn add_events(&mut self) -> Result<usize, SwarmyError> {
        self.sc2_state.add_events()
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

impl From<S2protocol::Vec3D> for Point3D {
    fn from(source: s2protocol::Vec3D) -> Point3D {
        Point3D::new(source.x as f32, source.y as f32, source.z as f32)
    }
}
