//! The terrain levels of a map drawn as SVG.

use leptos::prelude::*;
use swarmy_common::MapTerrain;

/// The terrain levels, from low ground (or void) to the highest ground.
const LEVEL_COLORS: [&str; 7] = [
    "#111827", "#1f2937", "#374151", "#4b5563", "#6b7280", "#9ca3af", "#d1d5db",
];

/// The terrain as SVG rectangles in map units, consecutive squares of the same level are drawn
/// as one rectangle. They must be inside a group that flips the y axis, the map y axis goes up.
pub fn terrain_rects(terrain: &MapTerrain) -> impl IntoView + use<> {
    let cell = terrain.cell_size.max(1) as f32;
    let width = terrain.width as usize;
    let mut rects = vec![];
    for (row, levels) in terrain.levels.chunks(width.max(1)).enumerate() {
        let mut start = 0;
        for column in 1..=levels.len() {
            if column == levels.len() || levels[column] != levels[start] {
                let level = (levels[start] as usize).min(LEVEL_COLORS.len() - 1);
                rects.push(view! {
                    <rect
                        x=start as f32 * cell
                        y=row as f32 * cell
                        width=(column - start) as f32 * cell
                        height=cell
                        fill=LEVEL_COLORS[level]
                    />
                });
                start = column;
            }
        }
    }
    rects.collect_view()
}

/// The terrain of a map on its own.
#[component]
pub fn MapMinimap(terrain: MapTerrain) -> impl IntoView {
    let width = terrain.width * terrain.cell_size;
    let height = terrain.height * terrain.cell_size;
    view! {
        <svg
            viewBox=format!("0 0 {width} {height}")
            class="w-full max-w-md border border-white/10 bg-gray-950"
        >
            <g transform=format!("translate(0 {height}) scale(1 -1)")>{terrain_rects(&terrain)}</g>
        </svg>
    }
}
