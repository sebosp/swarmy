//! Navigation between views, the views showing a replay or a player read which one from here.

use leptos::prelude::*;
use swarmy_common::{ReplayListQuery, TimeFrame};

pub const HOME_PAGE: &str = "Home";
pub const REPLAY_DETAIL_PAGE: &str = "Replay Detail";
pub const PLAYER_PAGE: &str = "Player";
pub const MAP_SCOUT_PAGE: &str = "Map Scout";
pub const MAP_LIST_PAGE: &str = "Map List";
pub const MAP_DETAILS_PAGE: &str = "Map Details";
pub const PATCH_OVERVIEW_PAGE: &str = "Patch Overview";
pub const UNIT_OVERVIEW_PAGE: &str = "Unit Overview";
pub const SCAN_PAGE: &str = "Scan";
pub const REPLAY_LIST_PAGE: &str = "Replay List";
pub const HELP_PAGE: &str = "Help";
pub const CHANGELOG_PAGE: &str = "Changelog";

#[derive(Debug, Clone, Copy)]
pub struct Navigation {
    pub active_page: RwSignal<String>,
    /// The snapshot id of the replay shown in the Replay Detail view.
    pub replay_id: RwSignal<Option<u64>>,
    /// The player shown in the Player view, as stored in the replay.
    pub player_name: RwSignal<String>,
    /// The map shown in the Map Details view, also the initial map of the Map Scout view.
    pub map_title: RwSignal<String>,
    /// The unit shown in the Unit Overview view.
    pub unit_name: RwSignal<String>,
    /// The patch shown in the Patch Overview view, the closest one in the balance data is used.
    pub patch_version: RwSignal<Option<u32>>,
    /// The race the Patch Overview view is filtered by, empty for all races.
    pub race: RwSignal<String>,
    /// The filters and the sort of the Replay List view, kept when going to other views.
    pub replay_query: RwSignal<ReplayListQuery>,
    /// The time frame of the Replay List view, its dates are in `replay_query`.
    pub replay_time_frame: RwSignal<TimeFrame>,
    /// Set to run Optimize when the Scan view opens, from the new replays banner.
    pub optimize_requested: RwSignal<bool>,
}

impl Navigation {
    pub fn new(active_page: &str) -> Self {
        Self {
            active_page: RwSignal::new(active_page.to_string()),
            replay_id: RwSignal::new(None),
            player_name: RwSignal::new(String::new()),
            map_title: RwSignal::new(String::new()),
            unit_name: RwSignal::new("Marine".to_string()),
            patch_version: RwSignal::new(None),
            race: RwSignal::new(String::new()),
            replay_query: RwSignal::new(ReplayListQuery::newest_first()),
            replay_time_frame: RwSignal::new(TimeFrame::AllTime),
            optimize_requested: RwSignal::new(false),
        }
    }

    pub fn open_replay(&self, ext_fs_id: u64) {
        self.replay_id.set(Some(ext_fs_id));
        self.active_page.set(REPLAY_DETAIL_PAGE.to_string());
    }

    pub fn open_map(&self, map_title: &str) {
        self.map_title.set(map_title.to_string());
        self.active_page.set(MAP_DETAILS_PAGE.to_string());
    }

    pub fn scout_map(&self, map_title: &str) {
        self.map_title.set(map_title.to_string());
        self.active_page.set(MAP_SCOUT_PAGE.to_string());
    }

    pub fn open_unit(&self, unit_name: &str) {
        self.unit_name.set(unit_name.to_string());
        self.active_page.set(UNIT_OVERVIEW_PAGE.to_string());
    }

    pub fn open_patch(&self, version: u32) {
        self.patch_version.set(Some(version));
        self.active_page.set(PATCH_OVERVIEW_PAGE.to_string());
    }

    /// Opens the Patch Overview view with the changes of the race.
    pub fn open_race(&self, race: &str) {
        self.race.set(race.to_string());
        self.active_page.set(PATCH_OVERVIEW_PAGE.to_string());
    }

    /// Opens the Replay List view with the games of a map and matchup, i.e. `PvT`, the newest
    /// first.
    pub fn open_replays(&self, map_title: &str, matchup: &str) {
        self.replay_query.set(ReplayListQuery {
            map_title: map_title.to_string(),
            matchup: matchup.to_string(),
            ..ReplayListQuery::newest_first()
        });
        self.replay_time_frame.set(TimeFrame::AllTime);
        self.active_page.set(REPLAY_LIST_PAGE.to_string());
    }

    /// Opens the Scan view and runs Optimize, it only adds the replays not in the snapshot.
    pub fn optimize_snapshot(&self) {
        self.optimize_requested.set(true);
        self.active_page.set(SCAN_PAGE.to_string());
    }

    pub fn open_player(&self, player_name: &str) {
        self.player_name.set(player_name.to_string());
        self.active_page.set(PLAYER_PAGE.to_string());
    }
}

/// The navigation is provided by the Main component.
pub fn use_navigation() -> Navigation {
    expect_context::<Navigation>()
}
