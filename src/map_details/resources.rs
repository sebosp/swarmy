//! The minimap of a map with its resources and start locations, and their totals and distances.

use crate::components::map_cell::{MapCell, MapHighlightContext};
use crate::components::terrain::MapMinimap;
use leptos::prelude::*;
use swarmy_common::*;

const MINERAL_COLOR: &str = "#67e8f9";
const RICH_MINERAL_COLOR: &str = "#facc15";
const GEYSER_COLOR: &str = "#4ade80";
// Rich vespene is purple in the game, gold is the color of the gold minerals.
const RICH_GEYSER_COLOR: &str = "#a78bfa";
/// The spot hovered in the tables of Map Details, a color no resource has.
const HIGHLIGHT_COLOR: &str = "#f472b6";
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
    let highlight = use_context::<MapHighlightContext>();
    let Some(image) = details.minimap.clone() else {
        return match details.terrain.clone() {
            Some(terrain) => view! { <MapMinimap terrain /> }.into_any(),
            None => view! {
                <p class="text-xs text-faint">"The map cache is not downloaded, no minimap."</p>
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
                class="w-full max-w-md border border-base-content/10 bg-base-100"
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
                // The spot hovered in the tables: a line across its x, one across its y and a
                // square on it.
                {move || {
                    let spot = highlight.and_then(|MapHighlightContext(spot)| spot.get())?;
                    let (center_x, center_y) = spot.center();
                    let (cx, cy) = to_svg(center_x, center_y);
                    let side = spot.size.max(2.);
                    Some(view! {
                        <g pointer-events="none">
                            <line x1=0 y1=cy x2=width y2=cy stroke=HIGHLIGHT_COLOR stroke-width=0.4 stroke-opacity=0.8 />
                            <line x1=cx y1=0 x2=cx y2=height stroke=HIGHLIGHT_COLOR stroke-width=0.4 stroke-opacity=0.8 />
                            <rect
                                x=cx - side / 2.
                                y=cy - side / 2.
                                width=side
                                height=side
                                fill=HIGHLIGHT_COLOR
                                fill-opacity=0.25
                                stroke=HIGHLIGHT_COLOR
                                stroke-width=0.6
                            />
                        </g>
                    })
                }}
            </svg>
            <label class="flex items-center gap-2 text-xs text-muted">
                <input
                    type="checkbox"
                    class="checkbox checkbox-xs"
                    prop:checked=move || show_resources.get()
                    on:change=move |_| show_resources.update(|show| *show = !*show)
                />
                "Show resources and start locations"
            </label>
            <p class="flex flex-wrap gap-x-3 text-xs text-faint">
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
