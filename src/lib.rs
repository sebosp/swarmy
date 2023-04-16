//! Starcraft 2 - Replay visualizer.
//!

use nom_mpq::{parser, MPQParserError, MPQ};
use rerun::components::ColorRGBA;
use rerun::external::re_log_types::DataTableError;
use rerun::external::re_viewer::external::eframe::Error as eframe_Error;
use rerun::time::Timeline;
use rerun::Session;
use rerun::{time, MsgSenderError};
use std::collections::HashMap;
pub use tracker_events::*;
pub mod unit_colors;
pub use unit_colors::*;
pub mod game_events;
pub use game_events::*;
pub mod tracker_events;

/// The game events seem to be at this ratio when compared to Tracker Events.
pub const GAME_EVENT_POS_RATIO: f32 = 27_000f32;

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
    last_game_loop: i64,
    /// The owner user_id
    user_id: Option<i64>,
    /// The name of the unit.
    name: Option<String>,
    /// The X position.
    x: f32,
    /// The Y position.
    y: f32,
    /// The Z position.
    z: f32,
    /// The age in game_loops of the unit.
    age: f32,
}

pub struct SC2Replay {
    /// The registered units state as they change through time.
    units: HashMap<i64, SC2Unit>,
    /// The absolute GameEvevnt loop timeline, the tracker loop should be relative to it.
    timeline: Timeline,
    /// The rerun session to display data.
    rerun_session: Session,
    /// The MPQ file being read.
    mpq: MPQ,
    /// The contents of the file
    file_contents: Vec<u8>,
}
impl SC2Replay {
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
        })
    }

    pub fn add_events(&mut self) -> Result<usize, SwarmyError> {
        let mut total_events = 0usize;
        total_events += add_game_events(
            &self.mpq,
            &self.file_contents,
            &mut self.session,
            &self.timeline,
        )?;
        total_events += add_tracker_events(
            &self.mpq,
            &self.file_contents,
            &mut self.session,
            &self.timeline,
        )?;
        Ok(total_events)
    }

    pub fn show(&self) -> Result<(), SwarmyError> {
        Ok(rerun::native_viewer::show(&self.rerun_session)?)
    }
}
