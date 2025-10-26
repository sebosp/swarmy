//! Starcraft 2 - Replay visualizer.
//!

use re_web_viewer_server::WebViewerServerPort;
use rerun::external::re_log_types::PathParseError;
use rerun::web_viewer::WebViewerSinkError;
use s2protocol::game_events::{read_balance_data_from_json, VersionedBalanceUnit};
use std::collections::HashMap;
use std::path::PathBuf;
// use rerun::external::re_viewer::external::eframe::Error as eframe_Error;
use nom_mpq::error::MPQParserError;
use rerun::{RecordingStream, RecordingStreamBuilder};
use s2protocol::state::SC2EventIterator;
use s2protocol::{InitData, S2ProtocolError, SC2EventType, SC2ReplayFilters};
pub use tracker_events::*;
pub mod unit_colors;
pub use unit_colors::*;
pub mod game_events;
pub use game_events::*;
pub mod tracker_events;
pub static SYSTEM_USERNAME: &str = "SYS";

// Some colors I really liked from a Freya Holmer presentation:
// https://www.youtube.com/watch?v=kfM-yu0iQBk
pub const FREYA_ORANGE: [u8; 4] = [0xeb, 0x79, 0x07, 0xff];
pub const FREYA_GOLD: [u8; 4] = [0xea, 0x9e, 0x36, 0xff];
pub const FREYA_RED: [u8; 4] = [0xf8, 0x10, 0x53, 0xff];
pub const FREYA_BLUE: [u8; 4] = [0x30, 0xb5, 0xf7, 0xff];
pub const FREYA_GREEN: [u8; 4] = [0x0a, 0xeb, 0x9f, 0xff];
pub const FREYA_LIGHT_BLUE: [u8; 4] = [0x72, 0xc5, 0xdd, 0xff];
pub const FREYA_GRAY: [u8; 4] = [0xb2, 0xc5, 0xc5, 0xff];
pub const FREYA_PINK: [u8; 4] = [0xea, 0xa4, 0x83, 0xff];
pub const FREYA_LIGHT_GRAY: [u8; 4] = [0xf4, 0xf5, 0xf8, 0xff];
pub const FREYA_DARK_BLUE: [u8; 4] = [0x4d, 0xa7, 0xc2, 0xff];
pub const FREYA_DARK_GREEN: [u8; 4] = [0x37, 0xbd, 0xa9, 0xff];
pub const FREYA_DARK_RED: [u8; 4] = [0xae, 0x20, 0x44, 0xff];
pub const FREYA_VIOLET: [u8; 4] = [0xa4, 0x01, 0xed, 0xff];
pub const FREYA_WHITE: [u8; 4] = [0xfa, 0xf8, 0xfb, 0xff];
pub const FREYA_YELLOW: [u8; 4] = [0xf7, 0xd4, 0x54, 0xff];
pub const FREYA_LIGHT_YELLOW: [u8; 4] = [0xea, 0xd8, 0xad, 0xff];
pub const FREYA_LIGHT_GREEN: [u8; 4] = [0x6e, 0xc2, 0x9c, 0xff];

// This was observed in a game with max game_loop = 13735 and a duration of 15:42 = 942 seconds.
// nanos 942000000000 / 13735 game_loops = 68583909 nanoseconds per game_loop
pub const GAME_LOOP_SPEED_NANOS: i64 = 68_583_909;

#[derive(thiserror::Error, Debug)]
pub enum SwarmyError {
    #[error("Rerun Message Sender error")]
    RerunMsgSender(#[from] rerun::external::anyhow::Error),
    #[error("Rerun Path Parse Error")]
    RerunDataTable(#[from] PathParseError),
    /*#[error("Rerun Eframe Error")]
    RerunEframe(#[from] eframe_Error),*/
    #[error("S2Protocol Error")]
    S2Protocol(#[from] S2ProtocolError),
    #[error("NomMPQ Error")]
    NomMPQ(#[from] MPQParserError),
    #[error("RecordingStream Error")]
    RecordingStream(#[from] rerun::RecordingStreamError),
    #[error("WebViewerServer Error")]
    RerunWebViewer(#[from] WebViewerSinkError),
    #[error("ParseAddr Error")]
    AddrParse(#[from] std::net::AddrParseError),
    #[error("I/O Error")]
    Io(#[from] std::io::Error),
}

pub struct SC2Rerun {
    /// The SC2 replay state as it steps through game loops.
    pub sc2_iterator: SC2EventIterator,

    /// The file path containing the SC2 Replay
    pub file_path: String,
}

impl SC2Rerun {
    pub fn new(
        file_path: &str,
        filters: SC2ReplayFilters,
        json_balance_data_dir: String,
    ) -> Result<Self, SwarmyError> {
        tracing::info!("Using balance data directory: {}", json_balance_data_dir);
        let versioned_abilities: HashMap<(u32, String), VersionedBalanceUnit> =
            match read_balance_data_from_json(PathBuf::from(&json_balance_data_dir)) {
                Ok(data) => data,
                Err(e) => {
                    tracing::error!("Failed to read balance data from JSON: {}", e);
                    HashMap::new()
                }
            };
        let file_contents: Vec<u8> = s2protocol::read_file(&PathBuf::from(&file_path))?;
        let (_input, mpq) = s2protocol::parser::parse(&file_contents)?;
        let init_data: InitData = InitData::new(file_path, 0u64, &mpq, &file_contents)?;
        let sc2_iterator =
            s2protocol::state::SC2EventIterator::new(&init_data, versioned_abilities)?
                .with_filters(filters);
        Ok(Self {
            sc2_iterator,
            file_path: file_path.to_string(),
        })
    }

    pub fn add_events(self, recording_stream: &RecordingStream) -> Result<(), SwarmyError> {
        let image = include_bytes!("../assets/Magannatha.png");

        recording_stream.log_static(
            "Map",
            &rerun::EncodedImage::from_file_contents(image.to_vec()),
        )?;
        for event_item in self.sc2_iterator {
            match event_item.event_type {
                SC2EventType::Tracker {
                    tracker_loop,
                    event,
                } => {
                    recording_stream.set_time_sequence("log", tracker_loop);
                    add_tracker_event(&event, event_item.change_hint, recording_stream)?
                }
                SC2EventType::Game {
                    game_loop,
                    user_id,
                    player_name,
                    event,
                } => {
                    let sys_player_name = String::from(SYSTEM_USERNAME);
                    recording_stream.set_time_sequence("log", game_loop);
                    add_game_event(
                        user_id,
                        player_name.as_ref().unwrap_or(&sys_player_name),
                        &event,
                        event_item.change_hint,
                        recording_stream,
                        game_loop,
                    )?
                }
            }
        }
        // Leave the rerun webviewer running... For now.
        std::thread::sleep(std::time::Duration::from_secs(100000));
        Ok(())
    }

    /// Calls the native viewer to display the recorded data.
    pub fn show(self) -> Result<(), SwarmyError> {
        let recording_stream = RecordingStreamBuilder::new(self.file_path.clone()).spawn()?;
        self.add_events(&recording_stream)
    }

    /// Connects to a remote address and ships the events
    pub fn connect(self, addr: Vec<String>) -> Result<(), SwarmyError> {
        //let mut endpoint = String::from("127.0.0.1:9876/proxy");
        // We need to find the current epoch in seconds:
        let epoch_seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| SwarmyError::Io(std::io::Error::other("Failed to get current time")))?
            .as_secs();
        let recording_stream =
            RecordingStreamBuilder::new(format!("{}{}", self.file_path.clone(), epoch_seconds))
                .serve_grpc()?;
        rerun::serve_web_viewer(rerun::web_viewer::WebViewerConfig {
            bind_ip: "0.0.0.0".to_string(),
            web_port: WebViewerServerPort(9101),
            connect_to: addr,
            ..Default::default()
        })
        .map_err(|e| {
            SwarmyError::RerunWebViewer(rerun::web_viewer::WebViewerSinkError::WebViewerServer(e))
        })?
        .detach();
        self.add_events(&recording_stream)
    }

    /// Saves the recording into an RRD file.
    pub fn save_to_file(self, output: &str) -> Result<(), SwarmyError> {
        let recording_stream = RecordingStreamBuilder::new(self.file_path.clone()).save(output)?;
        self.add_events(&recording_stream)
    }
}

pub fn from_vec3d(source: s2protocol::Vec3D) -> rerun::Vector3D {
    rerun::Vector3D::from(source.0)
}
