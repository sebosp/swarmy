//! The picture of a map (`swarmy_common::MapPicture`): the minimap of the map cache with the
//! resources and start locations drawn on top, or the terrain levels when the cache has no
//! minimap. The views draw their own marks over it (the bases of a replay, the fights) and
//! highlight the spot hovered in their tables (`map_cell`), with a line across its x, one across
//! its y and a square or a circle on it.

use crate::components::map_cell::MapHighlightContext;
use crate::components::terrain::MapMinimap;
use leptos::prelude::*;
use swarmy_common::*;

pub const MINERAL_COLOR: &str = "#67e8f9";
pub const RICH_MINERAL_COLOR: &str = "#facc15";
pub const GEYSER_COLOR: &str = "#4ade80";
// Rich vespene is purple in the game, gold is the color of the gold minerals.
pub const RICH_GEYSER_COLOR: &str = "#a78bfa";
/// The spot hovered in the tables of the view, a color no resource has.
pub const HIGHLIGHT_COLOR: &str = "#f472b6";
pub const START_COLOR: &str = "#f87171";

/// The number with thousands separators, i.e. `61,200`.
pub fn thousands(value: u32) -> String {
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

/// The frame of the map in map units: the playable area the picture covers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapFrame {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

impl MapFrame {
    /// Map coordinates to the ones of the SVG: the map y axis goes up, the SVG one down.
    pub fn to_svg(&self, x: f32, y: f32) -> (f32, f32) {
        (x - self.left, self.top - y)
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom.max(self.top) - self.bottom
    }
}

/// The marks of a view on the map, in map units: the closure gets the frame of the map, with
/// the conversion from map coordinates to the ones of the SVG.
pub type MapMarks = Box<dyn Fn(&MapFrame) -> AnyView>;

/// The minimap of the map cache with the resources and start locations drawn on top, the
/// terrain levels when the map cache has no minimap. `marks` draws the marks of the view over
/// them; `compact` leaves out the checkbox and the legend, for a small map in a header;
/// `base_opacity` fades the minimap under a surface of data.
#[component]
pub fn MapPicture(
    picture: MapPicture,
    #[prop(optional)] marks: Option<MapMarks>,
    #[prop(optional)] compact: bool,
    #[prop(optional, into)] base_opacity: Option<Signal<f32>>,
) -> impl IntoView {
    let show_resources = RwSignal::new(true);
    let highlight = use_context::<MapHighlightContext>();
    let Some(image) = picture.minimap.clone() else {
        return match picture.terrain.clone() {
            Some(terrain) => view! { <MapMinimap terrain /> }.into_any(),
            None => view! {
                <p class="text-xs text-faint">"The map cache is not downloaded, no minimap."</p>
            }
            .into_any(),
        };
    };
    let frame = MapFrame {
        left: image.left,
        bottom: image.bottom,
        right: image.right,
        top: image.top,
    };
    let width = frame.width();
    let height = frame.height();
    // The svg y axis goes down, the map y axis goes up.
    let to_svg = move |x: f32, y: f32| frame.to_svg(x, y);
    let resources = picture.resources.clone().unwrap_or_default();
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
    let view_marks = marks.map(|marks| marks(&frame));
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
                    opacity=move || base_opacity.map(|opacity| opacity.get()).unwrap_or(1.)
                />
                <g class:hidden=move || !show_resources.get()>{resource_marks} {start_marks}</g>
                {view_marks}
                // The spots hovered in the tables: a square (or a circle, for a radius) on each,
                // and for a single spot a line across its x and one across its y.
                {move || {
                    let spots = highlight
                        .map(|MapHighlightContext(spots)| spots.get())
                        .unwrap_or_default();
                    let first = *spots.first()?;
                    let shapes = spots
                        .iter()
                        .map(|spot| {
                            let (center_x, center_y) = spot.center();
                            let (cx, cy) = to_svg(center_x, center_y);
                            let side = spot.size.max(2.);
                            if spot.round {
                                view! {
                                    <circle
                                        cx=cx
                                        cy=cy
                                        r=side / 2.
                                        fill=HIGHLIGHT_COLOR
                                        fill-opacity=0.25
                                        stroke=HIGHLIGHT_COLOR
                                        stroke-width=0.6
                                    />
                                }
                                .into_any()
                            } else {
                                view! {
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
                                }
                                .into_any()
                            }
                        })
                        .collect_view();
                    let lines = (spots.len() == 1).then(|| {
                        let (center_x, center_y) = first.center();
                        let (cx, cy) = to_svg(center_x, center_y);
                        view! {
                            <line x1=0 y1=cy x2=width y2=cy stroke=HIGHLIGHT_COLOR stroke-width=0.4 stroke-opacity=0.8 />
                            <line x1=cx y1=0 x2=cx y2=height stroke=HIGHLIGHT_COLOR stroke-width=0.4 stroke-opacity=0.8 />
                        }
                    });
                    Some(view! {
                        <g pointer-events="none">
                            {lines}
                            {shapes}
                        </g>
                    })
                }}
            </svg>
            <Show when=move || !compact>
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
            </Show>
        </div>
    }
    .into_any()
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
