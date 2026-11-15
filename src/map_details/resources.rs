//! The resources and start locations of a map: their totals and distances, under the map picture
//! (`components::map_picture`).

use crate::components::map_cell::MapCell;
use crate::components::map_picture::thousands;
use leptos::prelude::*;
use swarmy_common::*;

/// A number of the data, emphasized against the words around it.
fn figure(value: String) -> AnyView {
    view! { <span class="font-medium text-base-content tabular-nums">{value}</span> }.into_any()
}

/// A count of the data, `-` when there are none.
fn count_or_dash(count: usize) -> AnyView {
    if count == 0 {
        view! { <span class="text-muted">"-"</span> }.into_any()
    } else {
        figure(count.to_string())
    }
}

/// The totals of the resources and the distances of the main base resources.
#[component]
pub fn MapResourcesPanel(resources: MapResources) -> impl IntoView {
    let rows: [(&str, &str, AnyView); 5] = [
        (
            "Minerals",
            "",
            view! {
                {figure(thousands(resources.minerals))}
                " in "
                {figure(resources.mineral_fields.to_string())}
                " mineral fields"
            }
            .into_any(),
        ),
        (
            "Vespene",
            "",
            view! {
                {figure(thousands(resources.vespene))}
                " in "
                {figure(resources.geysers.to_string())}
                " geysers"
            }
            .into_any(),
        ),
        (
            "Gold mineral fields",
            "",
            count_or_dash(resources.rich_mineral_fields),
        ),
        ("Rich geysers", "", count_or_dash(resources.rich_geysers)),
        (
            "Expansions",
            "All the bases of the map, the main bases included: every base has two geysers",
            figure((resources.geysers / 2).to_string()),
        ),
    ];
    let default_amounts = resources.default_amounts;
    view! {
        <div class="space-y-2">
            <h2 class="text-base/7 font-semibold text-base-content">"Resources"</h2>
            <table class="text-xs text-muted">
                <tbody>
                    {rows
                        .into_iter()
                        .map(|(label, help, value)| view! {
                            <tr title=(!help.is_empty()).then_some(help)>
                                <td class="pr-4 text-muted">{label}</td>
                                <td class="text-muted">{value}</td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
            <Show when=move || { default_amounts > 0 }>
                <p class="text-xs text-faint">
                    {format!(
                        "{default_amounts} resources use the default amount of their type (1,800 minerals per large field, 900 per small field, 2,250 vespene per geyser), the map does not set them.",
                    )}
                </p>
            </Show>
            <h2 class="text-base/7 font-semibold text-base-content">"Start locations"</h2>
            <table class="text-xs text-muted">
                <thead>
                    <tr class="text-left text-muted">
                        <th class="pr-3">"Start location"</th>
                        <th class="pr-3">"Position"</th>
                        <th class="pr-3 num" title="The mineral fields of the main base">"Fields"</th>
                        <th class="pr-3" title="From the start location to the mineral fields of the main base, in map units">
                            "Mineral distance"
                        </th>
                        <th title="From the start location to the geysers of the main base">"Geyser distance"</th>
                    </tr>
                </thead>
                <tbody>
                    {resources
                        .start_locations
                        .iter()
                        .map(|start| view! {
                            <tr>
                                <td class="pr-3 text-base-content">{start.name.clone()}</td>
                                <td class="pr-3"><MapCell x=start.x y=start.y centered=true /></td>
                                <td class="pr-3 num">{start.mineral_fields}</td>
                                <td class="pr-3">
                                    {format!(
                                        "{:.2} on average ({:.2} to {:.2})",
                                        start.avg_mineral_distance,
                                        start.min_mineral_distance,
                                        start.max_mineral_distance,
                                    )}
                                </td>
                                <td>{format!("{:.2}", start.avg_geyser_distance)}</td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}
