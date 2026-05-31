//! Swarmy Tauri Application

use leptos::prelude::*;
use swarmy_ui::components::left_sidebar_menu::LeftSideBarMenu;
use swarmy_ui::map_details::view::MapDetails;
use swarmy_ui::map_stats::view::StatsByMap;
use swarmy_ui::replay_list::view::ReplayList;
use swarmy_ui::scan::view::ScanDirectory;
use swarmy_ui::settings::view::Config;

#[component]
pub fn Main() -> impl IntoView {
    let active_page = RwSignal::new("Config".to_string());

    view! {
        <div id="swarmy-tauri-window" class="flex w-screen h-screen bg-gray-800 rounded">
            <LeftSideBarMenu active_page=active_page />
            <div
                id="swarmy-tauri-content"
                class="flex grow w-full h-full overflow-auto text-gray-400 bg-gray-900 rounded"
            >
                <Show when=move || active_page.get() == "Scan">
                    <div
                        id="swarmy-tauri-scan-directory-content"
                        class="flex flex-col grow p-2 rounded"
                    >
                        <ScanDirectory />
                    </div>
                </Show>
                <Show when=move || active_page.get() == "Stats By Map">
                    <div
                        id="swarmy-tauri-stats-by-map-content"
                        class="flex flex-col grow p-2 rounded"
                    >
                        <StatsByMap />
                    </div>
                </Show>
                <Show when=move || active_page.get() == "Map Details">
                    <div
                        id="swarmy-tauri-map-details-content"
                        class="flex flex-col grow p-2 rounded"
                    >
                        <MapDetails active_page />
                    </div>
                </Show>
                <Show when=move || active_page.get() == "Replay List">
                    <div
                        id="swarmy-tauri-stats-by-map-content"
                        class="flex flex-col grow p-2 rounded"
                    >
                        <ReplayList active_page />
                    </div>
                </Show>
                <Show when=move || active_page.get() == "Config">
                    <div id="swarmy-tauri-config-content">
                        <Config />
                    </div>
                </Show>
            </div>
        </div>
    }
}
