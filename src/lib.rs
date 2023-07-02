//! Starcraft 2 - Replay visualizer.
//!

use rerun::components::{ColorRGBA, Vec3D};
use rerun::external::re_log_types::DataTableError;
use rerun::external::re_viewer::external::eframe::Error as eframe_Error;
use rerun::time::Timeline;
use rerun::{time, MsgSenderError};
use rerun::{RecordingStream, RecordingStreamBuilder};
use s2protocol::{S2ProtocolError, SC2EventType, SC2ReplayFilters, SC2ReplayState};
pub use tracker_events::*;
pub mod unit_colors;
pub use unit_colors::*;
pub mod game_events;
pub use game_events::*;
pub mod tracker_events;

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
    #[error("S2Protocol Error")]
    S2Protocol(#[from] S2ProtocolError),
}

pub struct SC2Rerun {
    /// The absolute GameEvevnt loop timeline, the tracker loop should be relative to it.
    pub timeline: Timeline,

    /// The SC2 replay state as it steps through game loops.
    pub sc2_state: SC2ReplayState,

    /// The file path containing the SC2 Replay
    pub file_path: String,
}

impl SC2Rerun {
    pub fn new(
        file_path: &str,
        filters: SC2ReplayFilters,
        include_stats: bool,
    ) -> Result<Self, SwarmyError> {
        let timeline = rerun::time::Timeline::new("game_timeline", time::TimeType::Sequence);
        let sc2_state = SC2ReplayState::new(file_path, filters, include_stats)?;
        Ok(Self {
            timeline,
            sc2_state,
            file_path: file_path.to_string(),
        })
    }

    pub fn add_events(&mut self, recording_stream: &RecordingStream) -> Result<(), SwarmyError> {
        while let Some((event, updated_units)) = self.sc2_state.transduce() {
            match event {
                SC2EventType::Tracker {
                    tracker_loop,
                    event,
                } => {
                    add_tracker_event(self, tracker_loop, &event, updated_units, recording_stream)?
                }
                SC2EventType::Game {
                    game_loop,
                    user_id,
                    event,
                } => add_game_event(
                    self,
                    game_loop,
                    user_id,
                    &event,
                    updated_units,
                    recording_stream,
                )?,
            }
        }
        Ok(())
    }

    /// Calls the native viewer to display the recorded data.
    pub fn show(mut self) -> Result<(), SwarmyError> {
        let recording_info = rerun::new_recording_info(self.file_path.clone());
        rerun::native_viewer::spawn(recording_info, Default::default(), move |rec_stream| {
            self.add_events(&rec_stream).unwrap();
        })?;
        Ok(())
    }

    /// Saves the recording into an RRD file.
    pub fn save_to_file(self, output: &str) -> Result<(), SwarmyError> {
        let _ = RecordingStreamBuilder::new(self.file_path.clone())
            .save(output)
            .unwrap();
        Ok(())
    }
}

pub fn from_vec3d(source: s2protocol::Vec3D) -> Vec3D {
    Vec3D::new(source.0[0], source.0[1], source.0[2])
}
