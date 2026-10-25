//! Swarmy Tauri Application

use leptos::prelude::*;
use swarmy_ui::changelog::view::Changelog;
use swarmy_ui::components::left_sidebar_menu::LeftSideBarMenu;
use swarmy_ui::components::new_replays::NewReplaysBanner;
use swarmy_ui::help::view::Help;
use swarmy_ui::home::view::Home;
use swarmy_ui::map_details::view::MapDetailsView;
use swarmy_ui::map_scout::view::MapScoutView;
use swarmy_ui::map_stats::view::StatsByMap;
use swarmy_ui::navigation::*;
use swarmy_ui::patch_overview::view::PatchOverviewView;
use swarmy_ui::player_detail::view::PlayerDetailView;
use swarmy_ui::replay_detail::view::ReplayDetailView;
use swarmy_ui::replay_list::view::ReplayList;
use swarmy_ui::scan::view::ScanDirectory;
use swarmy_ui::settings::view::Config;
use swarmy_ui::unit_overview::view::UnitOverviewView;

#[component]
pub fn Main() -> impl IntoView {
    let navigation = Navigation::new(HOME_PAGE);
    provide_context(navigation);
    swarmy_ui::components::unit_icon::provide_unit_icons();
    let active_page = navigation.active_page;

    view! {
        <div id="swarmy-tauri-window" class="flex w-screen h-screen bg-gray-800 rounded">
            <LeftSideBarMenu active_page=active_page />
            <div class="flex flex-col grow w-full h-full min-w-0">
            <NewReplaysBanner />
            <div
                id="swarmy-tauri-content"
                class="flex grow w-full min-h-0 overflow-auto text-gray-400 bg-gray-900 rounded"
            >
                <Show when=move || active_page.get() == HOME_PAGE>
                    <div id="swarmy-tauri-home-content" class="flex flex-col grow min-w-0 p-2 rounded">
                        <Home />
                    </div>
                </Show>
                <Show when=move || active_page.get() == SCAN_PAGE>
                    <div
                        id="swarmy-tauri-scan-directory-content"
                        class="flex flex-col grow min-w-0 p-2 rounded"
                    >
                        <ScanDirectory />
                    </div>
                </Show>
                <Show when=move || active_page.get() == MAP_LIST_PAGE>
                    <div
                        id="swarmy-tauri-stats-by-map-content"
                        class="flex flex-col grow min-w-0 p-2 rounded"
                    >
                        <StatsByMap />
                    </div>
                </Show>
                <Show when=move || active_page.get() == MAP_DETAILS_PAGE>
                    <div
                        id="swarmy-tauri-map-details-content"
                        class="flex flex-col grow min-w-0 p-2 rounded"
                    >
                        <MapDetailsView />
                    </div>
                </Show>
                <Show when=move || active_page.get() == REPLAY_LIST_PAGE>
                    <div
                        id="swarmy-tauri-stats-by-map-content"
                        class="flex flex-col grow min-w-0 p-2 rounded"
                    >
                        <ReplayList />
                    </div>
                </Show>
                <Show when=move || active_page.get() == REPLAY_DETAIL_PAGE>
                    <div
                        id="swarmy-tauri-replay-detail-content"
                        class="flex flex-col grow min-w-0 p-2 rounded"
                    >
                        <ReplayDetailView />
                    </div>
                </Show>
                <Show when=move || active_page.get() == PLAYER_PAGE>
                    <div id="swarmy-tauri-player-content" class="flex flex-col grow min-w-0 p-2 rounded">
                        <PlayerDetailView />
                    </div>
                </Show>
                <Show when=move || active_page.get() == MAP_SCOUT_PAGE>
                    <div id="swarmy-tauri-map-scout-content" class="flex flex-col grow min-w-0 p-2 rounded">
                        <MapScoutView />
                    </div>
                </Show>
                <Show when=move || active_page.get() == PATCH_OVERVIEW_PAGE>
                    <div
                        id="swarmy-tauri-patch-overview-content"
                        class="flex flex-col grow min-w-0 p-2 rounded"
                    >
                        <PatchOverviewView />
                    </div>
                </Show>
                <Show when=move || active_page.get() == UNIT_OVERVIEW_PAGE>
                    <div
                        id="swarmy-tauri-unit-overview-content"
                        class="flex flex-col grow min-w-0 p-2 rounded"
                    >
                        <UnitOverviewView />
                    </div>
                </Show>
                <Show when=move || active_page.get() == HELP_PAGE>
                    <div id="swarmy-tauri-help-content" class="flex flex-col grow min-w-0 p-2 rounded">
                        <Help />
                    </div>
                </Show>
                <Show when=move || active_page.get() == CHANGELOG_PAGE>
                    <div id="swarmy-tauri-changelog-content" class="flex flex-col grow min-w-0 p-2 rounded">
                        <Changelog />
                    </div>
                </Show>
                <Show when=move || active_page.get() == "Config">
                    <div id="swarmy-tauri-config-content">
                        <Config />
                    </div>
                </Show>
            </div>
            </div>
        </div>
    }
}
