//! SC2Replay Directory Scan and Export to Arrow IPC Module

pub mod actions;
pub mod arrow_ipc_stats;
pub mod mpq_file_scan;
pub mod view;

use leptos::leptos_dom::logging::console_log;
use phosphor_leptos::{
    BARCODE, DATABASE, FOLDERS, HOURGLASS, IconWeightData, KEYBOARD, SHIPPING_CONTAINER, X_CIRCLE,
};
use reactive_stores::Store;
use s2protocol::dir_stats::SC2ReplaysDirStats;
use serde::{Deserialize, Serialize};
use swarmy_common::AppSettings;

/// The different stages from source selection to snapshot and cache download completion.
#[derive(Debug, Default, PartialEq, Clone, Eq)]
pub enum ActivityStage {
    #[default]
    None,
    /// A directory has been entered and the process of Scanning SC2Replays can begin.
    DirectoryEntered,
    /// The basic SC2 scan has began.
    ScanInit,
    /// The basic SC2 scan has finished with failure
    ScanFailure,
    /// The basic SC2 scan has finished.
    ScanDone,
    /// The Optimization has began.
    OptimizeInit,
    /// The Optimization has completed with failure
    OptimizeFailure,
    /// The Optimization has completed.
    OptimizeDone,
    /// Downloading Caches process has began.
    DownloadingCachesInit,
    /// Downloading Caches process has completed with failure.
    DownloadingCachesFailure,
    /// Downloading Caches process has completed.
    DownloadingCachesDone,
}

impl ActivityStage {
    pub fn color(&self) -> &'static str {
        match self {
            Self::None => "gray",
            Self::DirectoryEntered => "blue",
            Self::ScanFailure
            | ActivityStage::OptimizeFailure
            | ActivityStage::DownloadingCachesFailure => "red",
            Self::ScanInit | ActivityStage::OptimizeInit | ActivityStage::DownloadingCachesInit => {
                "teal"
            }
            Self::ScanDone | ActivityStage::OptimizeDone | ActivityStage::DownloadingCachesDone => {
                "green"
            }
        }
    }

    pub fn top_container_class(&self) -> &'static str {
        match self {
            Self::None => "border-l-4 mt-1 p-1 border-gray-500 bg-gray-500/10",
            Self::DirectoryEntered => "border-l-4 mt-1 p-1 border-blue-500 bg-blue-500/10",
            Self::ScanFailure
            | ActivityStage::OptimizeFailure
            | ActivityStage::DownloadingCachesFailure => {
                "border-l-4 mt-1 p-1 border-red-500 bg-red-500/10"
            }
            Self::ScanInit | ActivityStage::OptimizeInit | ActivityStage::DownloadingCachesInit => {
                "border-l-4 mt-1 p-1 border-teal-500 bg-teal-500/10"
            }
            Self::ScanDone | ActivityStage::OptimizeDone | ActivityStage::DownloadingCachesDone => {
                "border-l-4 mt-1 p-1 border-green-500 bg-green-500/10"
            }
        }
    }
    /// The div containing the message about the current stage status.
    pub fn alert_container_class(&self) -> &'static str {
        // TODO: For some reason I can't use format!("ext-{}-500", self.color(), it doesn't seem to
        // have effect...
        match self {
            Self::None => "shrink-0 size-5 text-gray-500 bg/text-gray-500/10",
            Self::DirectoryEntered => "shrink-0 size-5 text-blue-500 bg/text-blue-500/10",
            Self::ScanFailure
            | ActivityStage::OptimizeFailure
            | ActivityStage::DownloadingCachesFailure => {
                "shrink-0 size-5 text-red-500 bg/text-red-500/10"
            }
            Self::ScanInit | ActivityStage::OptimizeInit | ActivityStage::DownloadingCachesInit => {
                "shrink-0 size-5 text-teal-500 bg/text-teal-500/10"
            }
            Self::ScanDone | ActivityStage::OptimizeDone | ActivityStage::DownloadingCachesDone => {
                "shrink-0 size-5 text-green-500 bg/text-green-500/10"
            }
        }
    }

    /// The text class for the current stage status.
    pub fn text_class(&self) -> Vec<String> {
        vec!["text-sm".to_string(), format!("text-{}-300", self.color())]
    }

    pub fn text_content(&self) -> String {
        match self {
            Self::None => "Please enter a directory to scan".to_string(),
            Self::DirectoryEntered=> "Proceed to Scan.".to_string(),
            Self::ScanInit=> "Scanning in progres...".to_string(),
            Self::ScanFailure => "Scan has finished with failure, please check the directory and permissions or the logs.".to_string(),
            Self::ScanDone => "Scan has finished, proceed to Optimize.".to_string(),
            Self::OptimizeInit=> "Data Optimization for analysis in progress.".to_string(),
            Self::OptimizeFailure=> "Scan has finished with failure, please check the directory and permissions or the logs.".to_string(),
            Self::OptimizeDone => "Data Optimization for analysis completed, proceed to Download Caches.".to_string(),
            Self::DownloadingCachesInit => "Downloading Caches.".to_string(),
            Self::DownloadingCachesFailure=> "Download Cache has finished with failure, please check the directory and permissions or the logs.".to_string(),
            Self::DownloadingCachesDone => "Snapshot setup is complete.".to_string()
            }
    }

    pub fn icon_data(&self) -> &'static IconWeightData {
        match self {
            Self::None => KEYBOARD,
            Self::DirectoryEntered => FOLDERS,
            Self::ScanInit | Self::OptimizeInit | Self::DownloadingCachesInit => HOURGLASS,
            Self::ScanFailure | Self::OptimizeFailure | Self::DownloadingCachesFailure => X_CIRCLE,
            Self::ScanDone => BARCODE,
            Self::OptimizeDone => DATABASE,
            Self::DownloadingCachesDone => SHIPPING_CONTAINER,
        }
    }
    fn enum_index(&self) -> u8 {
        match *self {
            Self::None => 0,
            Self::DirectoryEntered => 1,
            Self::ScanInit => 2,
            Self::ScanFailure => 3,
            Self::ScanDone => 4,
            Self::OptimizeInit => 5,
            Self::OptimizeFailure => 6,
            Self::OptimizeDone => 7,
            Self::DownloadingCachesInit => 8,
            Self::DownloadingCachesFailure => 9,
            Self::DownloadingCachesDone => 10,
        }
    }
}

impl Ord for ActivityStage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.enum_index().cmp(&other.enum_index())
    }
}
impl PartialOrd for ActivityStage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

impl From<AppSettings> for ActivityStage {
    fn from(source: AppSettings) -> Self {
        console_log(&format!(
            "Transforming AppSettings into ActivityStage {:?}",
            source
        ));
        if source.snapshot_stats.caches_size > 0 {
            Self::DownloadingCachesDone
        } else if source.snapshot_stats.ipc_dir_size > 0 {
            Self::OptimizeDone
        } else if !source.replay_path.is_empty() {
            Self::DirectoryEntered
        } else {
            Self::None
        }
    }
}

#[derive(Store, Debug, Clone, Serialize, Deserialize)]
pub struct SC2ReplaysDirStatsTable {
    pub total_files: usize,
    pub total_supported_replays: usize,
    pub ability_supported_replays: usize,
    #[store(key: String = |row| row.name.clone())]
    pub top_10_players: Vec<SC2ReplaysDirPlayerEntry>,
    #[store(key: String = |row| row.title.clone())]
    pub top_10_maps: Vec<SC2ReplaysDirMapEntry>,
}

#[derive(Store, Debug, Clone, Serialize, Deserialize)]
pub struct SC2ReplaysDirPlayerEntry {
    pub idx: usize,
    pub clan: String,
    pub name: String,
    pub count: usize,
}

#[derive(Store, Debug, Clone, Serialize, Deserialize)]
pub struct SC2ReplaysDirMapEntry {
    pub idx: usize,
    pub title: String,
    pub count: usize,
}

impl From<SC2ReplaysDirStats> for SC2ReplaysDirStatsTable {
    fn from(stats: SC2ReplaysDirStats) -> Self {
        Self {
            total_files: stats.total_files,
            total_supported_replays: stats.total_supported_replays,
            ability_supported_replays: stats.ability_supported_replays,
            top_10_players: stats
                .top_10_players
                .into_iter()
                .enumerate()
                .map(|(idx, (name, count))| {
                    let (clan, name) = if let Some((clan, name)) = name.split_once("<sp/>") {
                        let clan = clan.replace("&gt;", "").replace("&lt;", "");
                        (clan.to_string(), name.to_string())
                    } else {
                        (String::new(), name)
                    };
                    SC2ReplaysDirPlayerEntry {
                        idx: idx + 1,
                        clan,
                        name,
                        count,
                    }
                })
                .collect(),
            top_10_maps: stats
                .top_10_maps
                .into_iter()
                .enumerate()
                .map(|(idx, (title, count))| SC2ReplaysDirMapEntry {
                    idx: idx + 1,
                    title,
                    count,
                })
                .collect(),
        }
    }
}
