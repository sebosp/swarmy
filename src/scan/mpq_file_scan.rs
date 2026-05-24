use super::*;
use leptos::prelude::*;
use reactive_graph::traits::Read;
use reactive_stores::Store;

#[component]
pub fn ReplayScanTable(dir_stats_data: Store<SC2ReplaysDirStatsTable>) -> impl IntoView {
    view! {
        <div class="flex flex-row">
            <div class="flex-item basis-128">
                <h3 class="text-neutral-content">
                    "Replays: "
                    <div class="badge badge-sm badge-ghost">
                        {move || dir_stats_data.total_files().get()}
                    </div>
                </h3>
            </div>
            <div class="flex-item basis-128">
                <h3
                    class="text-info"
                    title="Replays successfully parsed for GameEvents and TrackerEvents"
                >
                    "Basic: "
                    <div class="badge badge-sm badge-info">
                        {move || dir_stats_data.total_supported_replays().get()}
                    </div>
                </h3>
            </div>
            <div class="flex-item basis-128">
                <h3 class="text-success" title="Replays with balance data available for abilities">
                    "Enhanced Support: "
                    <div class="badge badge-sm badge-success">
                        {move || dir_stats_data.ability_supported_replays().get()}
                    </div>
                </h3>
            </div>
        </div>
        <div class="flex gap-4">
            <div class="flex-item grow">
                <h2 class="text-neutral-content flex justify-center bg-gray-800 rounded-sm">
                    "Top 10 Players"
                </h2>
                <table class="relative divide-white/15">
                    <thead>
                        <tr>
                            <th
                                scope="col"
                                class="py-3.5 pr-3 pl-4 text-left text-sm font-semibold whitespace-nowrap sm:pl-0 text-white"
                            >
                                ""
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Clan"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Name"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Games"
                            </th>
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-white/10 bg-gray-900">
                        <For
                            each=move || dir_stats_data.top_10_players()
                            key=|row| {
                                format!("{}:{}", row.read().name.clone(), row.read().clan.clone())
                            }
                            children=|child| {
                                let idx = child.clone().idx();
                                let clan = child.clone().clan().clone();
                                let name = child.clone().name().clone();
                                let count = child.clone().count();
                                view! {
                                    <tr>
                                        <td class="py-2 pr-3 pl-4 text-sm font-medium whitespace-nowrap text-white">
                                            {move || idx.get()}
                                        </td>
                                        <td class="px-2 py-2 text-sm whitespace-nowrap text-gray-400">
                                            {move || clan.get()}
                                        </td>
                                        <td class="px-2 py-2 text-sm whitespace-nowrap text-gray-400">
                                            {move || name.get()}
                                        </td>
                                        <td class="px-2 py-2 text-sm whitespace-nowrap text-gray-400">
                                            {move || count.get()}
                                        </td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
            <div class="flex-item grow">
                <h2 class="text-neutral-content flex justify-center bg-gray-800 rounded-sm">
                    "Top 10 Maps"
                </h2>
                <table class="relative divide-white/15">
                    <thead>
                        <tr>
                            <th
                                scope="col"
                                class="py-3.5 pr-3 pl-4 text-left text-sm font-semibold whitespace-nowrap sm:pl-0 text-white"
                            >
                                ""
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Map Title"
                            </th>
                            <th
                                scope="col"
                                class="px-2 py-3.5 text-left text-sm font-semibold whitespace-nowrap text-white"
                            >
                                "Games"
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || dir_stats_data.top_10_maps()
                            key=|row| row.read().title.clone()
                            children=|child| {
                                let idx = child.clone().idx();
                                let title = child.clone().title().clone();
                                let count = child.clone().count();
                                view! {
                                    <tr>
                                        <td class="py-2 pr-3 pl-4 text-sm font-medium whitespace-nowrap text-white">
                                            {move || idx.get()}
                                        </td>
                                        <td class="px-2 py-2 text-sm whitespace-nowrap text-gray-400">
                                            {move || title.get()}
                                        </td>
                                        <td class="px-2 py-2 text-sm whitespace-nowrap text-gray-400">
                                            {move || count.get()}
                                        </td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </div>
        </div>
    }
}
