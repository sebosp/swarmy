//! Module for application settings management.
use super::snapshot_stats::SnapshotStats;
use serde::{Deserialize, Serialize};

pub const DEFAULT_DISABLE_PARALLELISM: bool = false;
pub const DEFAULT_SCAN_MAX_FILES: i64 = 100000i64;
pub const DEFAULT_PROCESS_MAX_FILES: i64 = 100000i64;
pub const DEFAULT_TRAVERSE_MAX_DEPTH: i64 = 16i64;
pub const DEFAULT_MIN_VERSION: i64 = 0i64;
pub const DEFAULT_MAX_VERSION: i64 = 0i64;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    /// The current directory for replay_path.
    /// Altho it may be possible to create separate snapshots...
    pub replay_path: String,
    /// The cache_path, this is shared across replays/snapshots and downloaded from blizzard.
    pub cache_path: String,
    /// Whether to disable parallel scan/optimization
    pub disable_parallelism: bool,
    /// The current snapshots stats from the current replay_path
    pub snapshot_stats: SnapshotStats,
    /// The max files to scan, decrease in case the scan seems stuck.
    /// This is not ideal since there's no ordering to the scan at this point.
    pub scan_max_files: i64,
    /// The max number of files to process for Arrow snapshot generation.
    pub process_max_files: i64,
    /// The maximum number of subdirectories to inspect, decreate in case it seems
    /// that either scan or optimization is stuck, maybe some circular references we may not be
    /// handling properly.
    pub traverse_max_depth: i64,
    /// The minimum SC2 patch version ot consider when creating the snapshot.
    pub min_version: i64,
    /// The maximum SC2 patch version ot consider when creating the snapshot.
    pub max_version: i64,
    /// Watches the replay directory in the background for new replays, so that the user is told
    /// to Optimize the snapshot.
    #[serde(default)]
    pub watch_new_replays: bool,
    /// Scan moves the duplicate replays to this directory, outside of the replay path, keeping the
    /// directories they were found in. Nothing is deleted.
    #[serde(default)]
    pub duplicates_path: String,
    /// Shows a message when the watcher finds a new replay, with `watch_new_replays`.
    #[serde(default)]
    pub notify_new_replays: bool,
    /// The name of the user in their games, without the clan tag, i.e. `Sazed`: the player the
    /// analyses of one's own games are about.
    #[serde(default)]
    pub main_player_name: String,
    /// The swarmy started with `--serve` that runs the queries of the snapshot, i.e.
    /// `http://wormgat-zt:7878`, for a computer that cannot handle them. Empty to run them here.
    #[serde(default)]
    pub remote_backend_url: String,
    /// The token of the remote backend, its `--token`.
    #[serde(default)]
    pub remote_backend_token: String,
    /// The zoom of the interface in percent, 100 is the size of the system: Ctrl+= and Ctrl+-
    /// change it, Ctrl+0 resets it.
    #[serde(default = "default_zoom_percent")]
    pub zoom_percent: u32,
    /// The color theme, one of [`THEMES`].
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Reduces the animations and transitions: `system` follows the preference of the desktop,
    /// `on` and `off` override it.
    #[serde(default = "default_reduce_motion")]
    pub reduce_motion: String,
    /// Underlines every link, not only on hover.
    #[serde(default)]
    pub underline_links: bool,
    /// Shows the focus outline on every focused element, also with the mouse.
    #[serde(default)]
    pub focus_outline: bool,
}

/// The values of `reduce_motion`, `(id, label)`.
pub const REDUCE_MOTION_OPTIONS: &[(&str, &str)] = &[
    ("system", "Follow the system"),
    ("on", "Reduce"),
    ("off", "Animate"),
];

pub fn default_reduce_motion() -> String {
    "system".to_string()
}

pub const ZOOM_MIN_PERCENT: u32 = 50;
pub const ZOOM_MAX_PERCENT: u32 = 250;
pub const ZOOM_STEP_PERCENT: u32 = 10;

pub fn default_zoom_percent() -> u32 {
    100
}

/// The theme swarmy had before the themes, the dark gray and indigo one.
pub const DEFAULT_THEME: &str = "swarmy";

pub fn default_theme() -> String {
    DEFAULT_THEME.to_string()
}

/// Follows the light or dark preference of the system, `swarmy` when dark, `light` when light.
pub const SYSTEM_THEME: &str = "system";

/// The themes of the interface, `(id, label)`: the ids are the daisyUI themes of styles.css.
pub const THEMES: &[(&str, &str)] = &[
    (DEFAULT_THEME, "Swarmy, dark gray and indigo"),
    (SYSTEM_THEME, "Follow the system, Swarmy or Light"),
    ("light", "Light"),
    ("dark", "Dark"),
    ("high-contrast", "High contrast, dark"),
    ("catppuccin-latte", "Catppuccin Latte (light)"),
    ("catppuccin-frappe", "Catppuccin Frappé"),
    ("catppuccin-macchiato", "Catppuccin Macchiato"),
    ("catppuccin-mocha", "Catppuccin Mocha"),
    ("tokyonight-day", "Tokyo Night Day (light)"),
    ("tokyonight-storm", "Tokyo Night Storm"),
    ("tokyonight-night", "Tokyo Night Night"),
];

/// The zoom clamped to its bounds, in steps.
pub fn clamp_zoom_percent(percent: i64) -> u32 {
    let percent = percent.clamp(ZOOM_MIN_PERCENT as i64, ZOOM_MAX_PERCENT as i64) as u32;
    percent / ZOOM_STEP_PERCENT * ZOOM_STEP_PERCENT
}

/// The settings of the interface only, read without the snapshot: the theme and the zoom are
/// applied as soon as the window opens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub zoom_percent: u32,
    pub theme: String,
    pub reduce_motion: String,
    pub underline_links: bool,
    pub focus_outline: bool,
}

impl AppSettings {
    /// Whether the queries of the snapshot run in a remote backend.
    pub fn is_remote(&self) -> bool {
        !self.remote_backend_url.trim().is_empty()
    }
}

/// The button icons of the units as PNG data URLs, by unit name.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct UnitIcons {
    pub icons: std::collections::BTreeMap<String, String>,
}
