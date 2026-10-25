//! The minimap of a map with its resources and start locations, and their totals and distances.

use crate::components::terrain::MapMinimap;
use leptos::prelude::*;
use swarmy_common::*;

const MINERAL_COLOR: &str = "#67e8f9";
const RICH_MINERAL_COLOR: &str = "#facc15";
const GEYSER_COLOR: &str = "#4ade80";
const RICH_GEYSER_COLOR: &str = "#fb923c";
const START_COLOR: &str = "#f87171";

/// The number with thousands separators, i.e. `61,200`.
fn thousands(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (idx, digit) in digits.chars().enumerate() {
        if idx > 0 && (digits.len() - idx) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn resource_color(resource: &MapResource) -> &'static str {
    match (resource.is_geyser, resource.is_rich) {
        (false, false) => MINERAL_COLOR,
        (false, true) => RICH_MINERAL_COLOR,
        (true, false) => GEYSER_COLOR,
        (true, true) => RICH_GEYSER_COLOR,
    }
}

/// The minimap of the map cache with the resources and start locations drawn on top, the
/// terrain levels when the map cache has no minimap.
#[component]
pub fn MapPicture(details: MapDetails) -> impl IntoView {
    let show_resources = RwSignal::new(true);
    let Some(image) = details.minimap.clone() else {
        return match details.terrain.clone() {
            Some(terrain) => view! { <MapMinimap terrain /> }.into_any(),
            None => view! {
                <p class="text-xs text-gray-500">"The map cache is not downloaded, no minimap."</p>
            }
            .into_any(),
        };
    };
    let width = image.right - image.left;
    let height = image.top - image.bottom;
    // The svg y axis goes down, the map y axis goes up.
    let to_svg = move |x: f32, y: f32| (x - image.left, image.top - y);
    let resources = details.resources.clone().unwrap_or_default();
    let resource_marks = resources
        .resources
        .iter()
        .map(|resource| {
            let (x, y) = to_svg(resource.x, resource.y);
            let color = resource_color(resource);
            let title = format!("{} · {}", resource.unit_type, thousands(resource.amount));
            if resource.is_geyser {
                view! {
                    <circle cx=x cy=y r=1.5 fill=color fill-opacity=0.8 stroke="black" stroke-width=0.2>
                        <title>{title}</title>
                    </circle>
                }
                .into_any()
            } else {
                // Mineral fields are 2x1 cells.
                view! {
                    <rect x=x - 1. y=y - 0.5 width=2 height=1 fill=color stroke="black" stroke-width=0.15>
                        <title>{title}</title>
                    </rect>
                }
                .into_any()
            }
        })
        .collect_view();
    let start_marks = resources
        .start_locations
        .iter()
        .map(|start| {
            let (x, y) = to_svg(start.x, start.y);
            let label = start
                .name
                .trim_start_matches("Start Location")
                .trim()
                .trim_start_matches('0')
                .to_string();
            view! {
                <circle cx=x cy=y r=2.5 fill="none" stroke=START_COLOR stroke-width=0.6>
                    <title>{start.name.clone()}</title>
                </circle>
                <text x=x y=y + 1.2 font-size=3.5 text-anchor="middle" fill=START_COLOR font-weight="bold">
                    {label}
                </text>
            }
        })
        .collect_view();
    view! {
        <div class="space-y-1">
            <svg
                viewBox=format!("0 0 {width} {height}")
                class="w-full max-w-md border border-white/10 bg-gray-950"
            >
                <image
                    href=image.data_url.clone()
                    x=0
                    y=0
                    width=width
                    height=height
                    preserveAspectRatio="none"
                />
                <g class:hidden=move || !show_resources.get()>{resource_marks} {start_marks}</g>
            </svg>
            <label class="flex items-center gap-2 text-xs text-gray-400">
                <input
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    prop:checked=move || show_resources.get()
                    on:change=move |_| show_resources.update(|show| *show = !*show)
                />
                "Show resources and start locations"
            </label>
            <p class="flex flex-wrap gap-x-3 text-xs text-gray-500">
                <span style=format!("color: {MINERAL_COLOR}")>"■ Minerals"</span>
                <span style=format!("color: {RICH_MINERAL_COLOR}")>"■ Gold minerals"</span>
                <span style=format!("color: {GEYSER_COLOR}")>"● Vespene"</span>
                <span style=format!("color: {RICH_GEYSER_COLOR}")>"● Rich vespene"</span>
                <span style=format!("color: {START_COLOR}")>"○ Start locations"</span>
            </p>
        </div>
    }
    .into_any()
}

fn yes_no(count: usize, what: &str) -> String {
    match count {
        0 => "No".to_string(),
        count => format!("Yes, {count} {what}"),
    }
}

/// The totals of the resources and the distances of the main base resources.
#[component]
pub fn MapResourcesPanel(resources: MapResources) -> impl IntoView {
    let rows = [
        (
            "Minerals",
            format!(
                "{} in {} mineral fields",
                thousands(resources.minerals),
                resources.mineral_fields
            ),
        ),
        (
            "Vespene",
            format!(
                "{} in {} geysers",
                thousands(resources.vespene),
                resources.geysers
            ),
        ),
        (
            "Gold minerals",
            yes_no(resources.rich_mineral_fields, "gold mineral fields"),
        ),
        (
            "Rich vespene",
            yes_no(resources.rich_geysers, "rich geysers"),
        ),
        // Every base has two geysers.
        ("Bases", format!("about {}", resources.geysers / 2)),
    ];
    let default_amounts = resources.default_amounts;
    view! {
        <div class="space-y-2">
            <h2 class="text-base/7 font-semibold text-white">"Resources"</h2>
            <table class="text-xs text-gray-400">
                <tbody>
                    {rows
                        .into_iter()
                        .map(|(label, value)| view! {
                            <tr>
                                <td class="pr-4 text-gray-300">{label}</td>
                                <td class="text-white">{value}</td>
                            </tr>
                        })
                        .collect_view()}
                </tbody>
            </table>
            <Show when=move || { default_amounts > 0 }>
                <p class="text-xs text-gray-500">
                    {format!(
                        "{default_amounts} resources use the default amount of their type (1,800 minerals per large field, 900 per small field, 2,250 vespene per geyser), the map does not set them.",
                    )}
                </p>
            </Show>
            <h2 class="text-base/7 font-semibold text-white">"Start locations"</h2>
            <table class="text-xs text-gray-400">
                <thead>
                    <tr class="text-left text-gray-300">
                        <th class="pr-3">"Start location"</th>
                        <th class="pr-3">"Position"</th>
                        <th class="pr-3" title="The mineral fields of the main base">"Fields"</th>
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
                                <td class="pr-3 text-white">{start.name.clone()}</td>
                                <td class="pr-3">{format!("{:.1}, {:.1}", start.x, start.y)}</td>
                                <td class="pr-3">{start.mineral_fields}</td>
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_separates_the_thousands() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(900), "900");
        assert_eq!(thousands(61200), "61,200");
        assert_eq!(thousands(1234567), "1,234,567");
    }
}
