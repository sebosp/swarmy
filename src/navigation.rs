//! Navigation between views, the views showing a replay or a player read which one from here.

use leptos::prelude::*;
use swarmy_common::{ReplayListQuery, ReplayListSort, TimeFrame};

pub const HOME_PAGE: &str = "Home";
pub const TOUR_PAGE: &str = "Tour";
pub const REPLAY_DETAIL_PAGE: &str = "Replay Detail";
pub const PLAYER_PAGE: &str = "Player";
pub const MAP_SCOUT_PAGE: &str = "Map Scout";
pub const MAP_LIST_PAGE: &str = "Map List";
pub const MAP_DETAILS_PAGE: &str = "Map Details";
pub const PATCH_OVERVIEW_PAGE: &str = "Patch Overview";
pub const UNIT_OVERVIEW_PAGE: &str = "Unit Overview";
pub const PATCH_EVOLUTION_PAGE: &str = "Patch Evolution";
pub const RACE_DETAIL_PAGE: &str = "Race Detail";
pub const ABILITY_PAGE: &str = "Ability";
pub const PROGRESS_PAGE: &str = "Progress";
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
    /// The race shown in the Race Detail view.
    pub race_detail: RwSignal<String>,
    /// The filters and the sort of the Replay List view, kept when going to other views.
    pub replay_query: RwSignal<ReplayListQuery>,
    /// The time frame of the Replay List view, its dates are in `replay_query`.
    pub replay_time_frame: RwSignal<TimeFrame>,
    /// Set to run Optimize when the Scan view opens, from the new replays banner.
    pub optimize_requested: RwSignal<bool>,
    /// The map and players locked in Replay Detail, the previous and next replays are of them.
    pub replay_locks: RwSignal<ReplayLocks>,
    /// What the view that opened the replay knew of it, for the message while it is analysed.
    pub replay_preview: RwSignal<Option<ReplayPreview>>,
    /// The ability shown in the Ability view, [`swarmy_common::TrackedAbility::ability`].
    pub ability: RwSignal<String>,
    /// The replay shown is the newest one of the locked map and players: there is no next one.
    pub replay_is_latest: RwSignal<bool>,
}

/// The map and players of a replay, as the view that opens it shows them.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ReplayPreview {
    pub ext_fs_id: u64,
    pub map_title: String,
    pub players: Vec<String>,
}

/// The map and players locked in Replay Detail.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ReplayLocks {
    pub map_title: Option<String>,
    /// The player names as stored in the replay.
    pub player_names: Vec<String>,
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
            race_detail: RwSignal::new("Terran".to_string()),
            replay_query: RwSignal::new(ReplayListQuery::newest_first()),
            replay_time_frame: RwSignal::new(TimeFrame::AllTime),
            optimize_requested: RwSignal::new(false),
            replay_locks: RwSignal::new(ReplayLocks::default()),
            replay_preview: RwSignal::new(None),
            ability: RwSignal::new(String::new()),
            replay_is_latest: RwSignal::new(false),
        }
    }

    pub fn open_replay(&self, ext_fs_id: u64) {
        self.replay_preview.set(None);
        self.replay_is_latest.set(false);
        self.replay_id.set(Some(ext_fs_id));
        self.active_page.set(REPLAY_DETAIL_PAGE.to_string());
    }

    /// Opens a replay whose map and players the view knows, they are shown while it is analysed.
    pub fn open_replay_with(&self, preview: ReplayPreview) {
        let ext_fs_id = preview.ext_fs_id;
        self.replay_preview.set(Some(preview));
        self.replay_is_latest.set(false);
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

    /// Opens the Ability view of an ability, i.e. `SpawnLarva.Execute`.
    pub fn open_ability(&self, ability: &str) {
        self.ability.set(ability.to_string());
        self.active_page.set(ABILITY_PAGE.to_string());
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

    /// Opens the Race Detail view of the race: its matchups, maps, units and abilities.
    pub fn open_race_detail(&self, race: &str) {
        self.race_detail.set(race.to_string());
        self.active_page.set(RACE_DETAIL_PAGE.to_string());
    }

    /// Opens the Replay List view with the games of a map and matchup, i.e. `PvT`, the newest
    /// first.
    pub fn open_replays(&self, map_title: &str, matchup: &str) {
        // The sort the viewer chose is kept.
        let sort = self.replay_query.get_untracked();
        self.replay_query.set(ReplayListQuery {
            map_title: map_title.to_string(),
            matchup: matchup.to_string(),
            sort_by: sort.sort_by,
            descending: sort.descending,
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

/// The key of the sort of the Replay List in the storage of the webview.
const REPLAY_LIST_SORT_KEY: &str = "swarmy.replay_list.sort";

/// Restores the sort of the Replay List the viewer chose last time, and saves it as it changes.
pub fn remember_replay_list_sort(navigation: Navigation) {
    if let Some((sort_by, descending)) =
        crate::common::storage::load_json::<(ReplayListSort, bool)>(REPLAY_LIST_SORT_KEY)
    {
        navigation.replay_query.update(|query| {
            query.sort_by = sort_by;
            query.descending = descending;
        });
    }
    Effect::new(move |_| {
        let query = navigation.replay_query.get();
        crate::common::storage::save_json(REPLAY_LIST_SORT_KEY, &(query.sort_by, query.descending));
    });
}

/// The navigation is provided by the Main component.
pub fn use_navigation() -> Navigation {
    expect_context::<Navigation>()
}

/// The address of a view: `#/<page>?<what it shows>`, i.e. `#/replay-detail?replay=1234` or
/// `#/player?name=Serral`. The browser version can then be shared and the Back button works; the app
/// shows its views the same way.
pub fn view_route(navigation: &Navigation) -> String {
    let page = navigation.active_page.get();
    let encode = |value: &str| String::from(js_sys::encode_uri_component(value));
    let query = match page.as_str() {
        REPLAY_DETAIL_PAGE => navigation.replay_id.get().map(|id| format!("replay={id}")),
        PLAYER_PAGE => Some(navigation.player_name.get())
            .filter(|name| !name.is_empty())
            .map(|name| format!("name={}", encode(&name))),
        MAP_DETAILS_PAGE | MAP_SCOUT_PAGE => Some(navigation.map_title.get())
            .filter(|map| !map.is_empty())
            .map(|map| format!("map={}", encode(&map))),
        UNIT_OVERVIEW_PAGE => Some(format!("unit={}", encode(&navigation.unit_name.get()))),
        PATCH_OVERVIEW_PAGE => {
            let mut parts = vec![];
            if let Some(version) = navigation.patch_version.get() {
                parts.push(format!("version={version}"));
            }
            let race = navigation.race.get();
            if !race.is_empty() {
                parts.push(format!("race={}", encode(&race)));
            }
            (!parts.is_empty()).then(|| parts.join("&"))
        }
        RACE_DETAIL_PAGE => Some(format!("race={}", encode(&navigation.race_detail.get()))),
        ABILITY_PAGE => Some(navigation.ability.get())
            .filter(|ability| !ability.is_empty())
            .map(|ability| format!("ability={}", encode(&ability))),
        _ => None,
    };
    let slug = page.to_lowercase().replace(' ', "-");
    match query {
        Some(query) => format!("#/{slug}?{query}"),
        None => format!("#/{slug}"),
    }
}

/// The page of a route slug, i.e. `replay-detail`.
fn page_of_slug(slug: &str) -> Option<&'static str> {
    [
        HOME_PAGE,
        TOUR_PAGE,
        REPLAY_DETAIL_PAGE,
        PLAYER_PAGE,
        MAP_SCOUT_PAGE,
        MAP_LIST_PAGE,
        MAP_DETAILS_PAGE,
        PATCH_OVERVIEW_PAGE,
        UNIT_OVERVIEW_PAGE,
        PATCH_EVOLUTION_PAGE,
        RACE_DETAIL_PAGE,
        ABILITY_PAGE,
        PROGRESS_PAGE,
        SCAN_PAGE,
        REPLAY_LIST_PAGE,
        HELP_PAGE,
        CHANGELOG_PAGE,
        "Config",
    ]
    .into_iter()
    .find(|page| page.to_lowercase().replace(' ', "-") == slug)
}

/// Shows the view of a route, an unknown route is left alone.
pub fn open_route(navigation: &Navigation, route: &str) {
    let route = route.trim_start_matches('#').trim_start_matches('/');
    let (slug, query) = route.split_once('?').unwrap_or((route, ""));
    let Some(page) = page_of_slug(slug) else {
        return;
    };
    let decode = |value: &str| {
        js_sys::decode_uri_component(value)
            .map(String::from)
            .unwrap_or_else(|_| value.to_string())
    };
    for (key, value) in query.split('&').filter_map(|part| part.split_once('=')) {
        let value = decode(value);
        match key {
            "replay" => {
                if let Ok(id) = value.parse() {
                    navigation.replay_preview.set(None);
                    navigation.replay_is_latest.set(false);
                    navigation.replay_id.set(Some(id));
                }
            }
            "name" => navigation.player_name.set(value),
            "map" => navigation.map_title.set(value),
            "unit" => navigation.unit_name.set(value),
            "version" => navigation.patch_version.set(value.parse().ok()),
            "race" if page == RACE_DETAIL_PAGE => navigation.race_detail.set(value),
            "race" => navigation.race.set(value),
            "ability" => navigation.ability.set(value),
            _ => {}
        }
    }
    navigation.active_page.set(page.to_string());
}

/// Keeps the address of the window on the view shown: opens the view of the address when swarmy
/// starts and when the address changes (Back, a link pasted), and writes the address of each view
/// opened.
pub fn sync_with_address(navigation: Navigation) {
    let current_hash = || window().location().hash().unwrap_or_default();
    let hash = current_hash();
    if !hash.is_empty() {
        open_route(&navigation, &hash);
    }
    Effect::new(move |_| {
        let route = view_route(&navigation);
        if current_hash() != route {
            // A new entry of the history: Back returns to the previous view.
            let _ = window().location().set_hash(&route);
        }
    });
    window_event_listener(leptos::ev::hashchange, move |_| {
        let hash = current_hash();
        if hash != untrack(|| view_route(&navigation)) {
            open_route(&navigation, &hash);
        }
    });
}
