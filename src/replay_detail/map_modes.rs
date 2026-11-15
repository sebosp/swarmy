//! The modes of the map of a replay: the bases (the default), the deaths as a density surface
//! drawn like a height map (the cells come from the `replay_detail::death_density` query of the
//! backend, here they are only drawn and contoured), and the fights over time. The conventions come from a research of
//! cartography and conflict mapping (2026-11-11, see the changelog): a kernel density estimate
//! on the map cells with the Silverman/Scott bandwidth, 6 quantile classes of a single-hue
//! sequential ramp with contour lines at the class thresholds (marching squares), the base map
//! faded; the fights as translucent discs sized by the resources lost and colored by time with a
//! perceptually uniform ordered ramp (viridis), painted in time order, 5 classes under 20 fights
//! and 7 above, a labeled legend; never a rainbow, never time as categorical hues. The report of
//! the research is `docs/research/replay-map-density-and-time.md`.

use crate::components::map_picture::MapFrame;
use crate::format_game_time;
use leptos::prelude::*;
use swarmy_common::*;

/// What the map of the replay shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapMode {
    /// The bases of the players and the spot hovered in the tables.
    #[default]
    Bases,
    /// Where the units died, as a height map: the more deaths, the higher.
    Deaths,
    /// The fights, colored by the time they happened.
    OverTime,
}

impl MapMode {
    pub const ALL: [MapMode; 3] = [MapMode::Bases, MapMode::Deaths, MapMode::OverTime];

    pub fn label(self) -> &'static str {
        match self {
            MapMode::Bases => "Bases",
            MapMode::Deaths => "Deaths",
            MapMode::OverTime => "Over time",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            MapMode::Bases => "The bases of the players and what is hovered in the tables",
            MapMode::Deaths => {
                "Where the units died, drawn as a height map: the darker, the more deaths around"
            }
            MapMode::OverTime => {
                "The fights as discs sized by what was lost, colored from the start of the game to its end, the latest on top"
            }
        }
    }

    /// The opacity of the minimap under the marks: faded under a surface or the discs, so that
    /// the colors of the map are not read as the ones of the data.
    pub fn base_opacity(self) -> f32 {
        match self {
            MapMode::Bases => 1.,
            MapMode::Deaths | MapMode::OverTime => 0.35,
        }
    }
}

/// ColorBrewer YlOrBr, 6 classes: a single sequence from light to dark, colorblind safe, the
/// browns of a relief map.
pub const DENSITY_RAMP: [&str; DENSITY_CLASSES as usize] = [
    "#ffffd4", "#fee391", "#fec44f", "#fe9929", "#d95f0e", "#993404",
];

/// The contour lines between the classes.
pub const CONTOUR_COLOR: &str = "#662506";

/// Viridis at 7 stops: perceptually uniform, ordered, colorblind safe, for the time of the
/// fights. The classes take the first 5 or all 7 stops.
pub const TIME_RAMP: [&str; 7] = [
    "#440154", "#443983", "#31688e", "#21918c", "#35b779", "#90d743", "#fde725",
];

/// The density of the deaths over the cells of the frame (`replay_detail::death_density`, the
/// query of the backend), as a grid for the contour lines: row by row from the bottom left, 0
/// where the query left open ground.
pub struct DensityGrid {
    pub columns: usize,
    pub rows: usize,
    pub values: Vec<f32>,
}

impl DensityGrid {
    /// The grid of the frame from the cells of the query.
    pub fn from_cells(cells: &[DensityCell], frame: &MapFrame) -> Self {
        let columns = (frame.width() / DENSITY_CELL).ceil().max(1.) as usize;
        let rows = (frame.height() / DENSITY_CELL).ceil().max(1.) as usize;
        let mut values = vec![0f32; columns * rows];
        for cell in cells {
            let (column, row) = (cell.column as usize, cell.row as usize);
            if column < columns && row < rows {
                values[row * columns + column] = cell.density;
            }
        }
        Self {
            columns,
            rows,
            values,
        }
    }

    fn value(&self, column: usize, row: usize) -> f32 {
        self.values[row * self.columns + column]
    }

    /// The thresholds between the classes, the lowest density of each class from the second: the
    /// contour lines run between the tints.
    pub fn thresholds(cells: &[DensityCell]) -> Vec<f32> {
        (2..=DENSITY_CLASSES)
            .filter_map(|class| {
                cells
                    .iter()
                    .filter(|cell| cell.class == class)
                    .map(|cell| cell.density)
                    .min_by(f32::total_cmp)
            })
            .collect()
    }

    /// The segments of the contour line at `threshold`, by marching squares over the cell
    /// centers: for each square of four neighbouring centers the corners above the threshold
    /// give one of 16 cases, the line crosses the edges where the value interpolates to the
    /// threshold; the saddles (two opposite corners above) are resolved by the value at the
    /// center of the square. In map units.
    pub fn contour(&self, threshold: f32, frame: &MapFrame) -> Vec<[(f32, f32); 2]> {
        let mut segments = vec![];
        let center = |column: usize, row: usize| {
            (
                frame.left + (column as f32 + 0.5) * DENSITY_CELL,
                frame.bottom + (row as f32 + 0.5) * DENSITY_CELL,
            )
        };
        for row in 0..self.rows.saturating_sub(1) {
            for column in 0..self.columns.saturating_sub(1) {
                // The corners clockwise from the bottom left: bl, tl, tr, br.
                let corners = [
                    (center(column, row), self.value(column, row)),
                    (center(column, row + 1), self.value(column, row + 1)),
                    (center(column + 1, row + 1), self.value(column + 1, row + 1)),
                    (center(column + 1, row), self.value(column + 1, row)),
                ];
                let case = corners
                    .iter()
                    .enumerate()
                    .fold(0usize, |case, (idx, (_, value))| {
                        case | (((*value >= threshold) as usize) << idx)
                    });
                if case == 0 || case == 15 {
                    continue;
                }
                // The crossing on the edge from corner a to corner b.
                let crossing = |a: usize, b: usize| {
                    let ((xa, ya), va) = corners[a];
                    let ((xb, yb), vb) = corners[b];
                    let t = if (vb - va).abs() < f32::EPSILON {
                        0.5
                    } else {
                        ((threshold - va) / (vb - va)).clamp(0., 1.)
                    };
                    (xa + (xb - xa) * t, ya + (yb - ya) * t)
                };
                // The edges: 0 left (bl-tl), 1 top (tl-tr), 2 right (tr-br), 3 bottom (br-bl).
                let edge = |e: usize| crossing(e, (e + 1) % 4);
                let mut push = |e1: usize, e2: usize| segments.push([edge(e1), edge(e2)]);
                match case {
                    1 | 14 => push(0, 3),
                    2 | 13 => push(0, 1),
                    3 | 12 => push(1, 3),
                    4 | 11 => push(1, 2),
                    6 | 9 => push(0, 2),
                    7 | 8 => push(2, 3),
                    5 | 10 => {
                        let middle = corners.iter().map(|(_, v)| v).sum::<f32>() / 4.;
                        // bl and tr above (5), or tl and br above (10): the center decides
                        // which pairs of edges connect.
                        if (middle >= threshold) == (case == 5) {
                            push(0, 1);
                            push(2, 3);
                        } else {
                            push(0, 3);
                            push(1, 2);
                        }
                    }
                    _ => {}
                }
            }
        }
        segments
    }
}

/// The time classes of the fights: 5 under 20 fights, 7 above, the stops of [`TIME_RAMP`].
pub fn time_classes(fights: usize) -> usize {
    if fights < 20 { 5 } else { 7 }
}

/// The class of a second of the game, 0 to `classes - 1`.
pub fn time_class(seconds: u32, duration: u32, classes: usize) -> usize {
    if duration == 0 {
        return 0;
    }
    ((seconds as f32 / duration as f32 * classes as f32).floor() as usize).min(classes - 1)
}

/// The color of a time class: the stops of the ramp spread over the classes.
pub fn time_color(class: usize, classes: usize) -> &'static str {
    let position = if classes <= 1 {
        0
    } else {
        (class as f32 * (TIME_RAMP.len() - 1) as f32 / (classes - 1) as f32).round() as usize
    };
    TIME_RAMP[position.min(TIME_RAMP.len() - 1)]
}

/// The density of the deaths as a height map over the frame: the cells of the query tinted by
/// class under the contour lines of the thresholds, as SVG in map units through `frame.to_svg`.
pub fn density_marks(cells: &[DensityCell], frame: &MapFrame) -> AnyView {
    if cells.is_empty() {
        return ().into_any();
    }
    let rects = cells
        .iter()
        .map(|cell| {
            let x = frame.left + cell.column as f32 * DENSITY_CELL;
            let y = frame.bottom + (cell.row as f32 + 1.) * DENSITY_CELL;
            let (sx, sy) = frame.to_svg(x, y);
            let color = DENSITY_RAMP[(cell.class.clamp(1, DENSITY_CLASSES) - 1) as usize];
            view! { <rect x=sx y=sy width=DENSITY_CELL height=DENSITY_CELL fill=color /> }
        })
        .collect_view();
    let grid = DensityGrid::from_cells(cells, frame);
    let contours = DensityGrid::thresholds(cells)
        .into_iter()
        .map(|threshold| {
            let path: String = grid
                .contour(threshold, frame)
                .into_iter()
                .map(|[(x1, y1), (x2, y2)]| {
                    let (a, b) = (frame.to_svg(x1, y1), frame.to_svg(x2, y2));
                    format!("M{:.2} {:.2}L{:.2} {:.2}", a.0, a.1, b.0, b.1)
                })
                .collect();
            view! {
                <path d=path fill="none" stroke=CONTOUR_COLOR stroke-width=0.25 stroke-opacity=0.7 />
            }
        })
        .collect_view();
    view! {
        // The opacity is of the group, not of each cell: nothing compounds.
        <g opacity=0.6 shape-rendering="crispEdges">{rects}</g>
        <g>{contours}</g>
    }
    .into_any()
}

/// The radius of the disc of a fight, in map units: proportional to the square root of the
/// resources lost in it, as a proportional symbol, between 1.5 (still a target for the pointer)
/// and 9 (a sixteenth of a map).
pub fn fight_radius(resources_lost: u32) -> f32 {
    (resources_lost as f32 / 100.).sqrt().clamp(1.5, 9.)
}

/// The fill of the discs: lighter with many fights, so that the stacks do not saturate.
pub fn fight_fill_opacity(fights: usize) -> f32 {
    match fights {
        0..10 => 0.4,
        10..=25 => 0.3,
        _ => 0.25,
    }
}

/// The fights as discs sized by what was lost, colored by the time they happened, the earliest
/// painted first and the latest on top with a thicker ring (the newest fifth).
pub fn over_time_marks(engagements: &[Engagement], duration: u32, frame: &MapFrame) -> AnyView {
    let with_place: Vec<&Engagement> = engagements
        .iter()
        .filter(|e| e.epicenter.is_some())
        .collect();
    let classes = time_classes(with_place.len());
    let fill_opacity = fight_fill_opacity(with_place.len());
    let latest = with_place
        .len()
        .saturating_sub(with_place.len().div_ceil(5));
    with_place
        .iter()
        .enumerate()
        .map(|(idx, engagement)| {
            let epicenter = engagement.epicenter.as_ref().expect("filtered");
            let (cx, cy) = frame.to_svg(epicenter.x, epicenter.y);
            let class = time_class(engagement.start_seconds, duration, classes);
            let color = time_color(class, classes);
            let lost: u32 = engagement
                .losses
                .iter()
                .map(|loss| loss.resources.total())
                .sum();
            let title = format!(
                "Fight {} - {}: {} deaths, {} resources lost, spread {:.0}",
                format_game_time(engagement.start_seconds),
                format_game_time(engagement.end_seconds),
                epicenter.deaths,
                lost,
                epicenter.radius,
            );
            // The newest fifth of the fights has a thicker ring, it is what happened last.
            let stroke_width = if idx >= latest { 0.8 } else { 0.4 };
            view! {
                <circle
                    cx=cx
                    cy=cy
                    r=fight_radius(lost)
                    fill=color
                    fill-opacity=fill_opacity
                    stroke=color
                    stroke-opacity=0.9
                    stroke-width=stroke_width
                >
                    <title>{title}</title>
                </circle>
            }
        })
        .collect_view()
        .into_any()
}

/// The highest density of the Deaths legend: whole deaths from 10, one decimal below.
pub fn format_peak(peak: f32) -> String {
    if peak >= 10. {
        format!("> {peak:.0}")
    } else {
        format!("> {peak:.1}")
    }
}

/// The legend of a mode under the map: the classes as patches with the ends labeled. `peak` is
/// the highest density of the Deaths mode, in deaths per cell.
#[component]
pub fn MapModeLegend(mode: MapMode, duration: u32, fights: usize, peak: f32) -> impl IntoView {
    let (colors, start, end): (Vec<&'static str>, String, String) = match mode {
        MapMode::Bases => return ().into_any(),
        MapMode::Deaths => (DENSITY_RAMP.to_vec(), "1 <".to_string(), format_peak(peak)),
        MapMode::OverTime => {
            let classes = time_classes(fights);
            (
                (0..classes).map(|c| time_color(c, classes)).collect(),
                format_game_time(0),
                format_game_time(duration),
            )
        }
    };
    view! {
        <div class="flex items-center gap-1 text-xs text-faint">
            <span>{start}</span>
            <span class="flex h-2 flex-1 overflow-hidden rounded-sm">
                {colors
                    .into_iter()
                    .map(|color| view! { <span class="flex-1" style=format!("background: {color}")></span> })
                    .collect_view()}
            </span>
            <span>{end}</span>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_peak_of_the_legend_is_whole_from_ten() {
        assert_eq!(format_peak(23.6), "> 24");
        assert_eq!(format_peak(3.44), "> 3.4");
        assert_eq!(format_peak(0.), "> 0.0");
    }

    fn frame() -> MapFrame {
        MapFrame {
            left: 0.,
            bottom: 0.,
            right: 20.,
            top: 20.,
        }
    }

    /// A Gaussian bump of the query's making at the center of the frame, as cells.
    fn bump() -> Vec<DensityCell> {
        let mut cells = vec![];
        for row in 0..10u32 {
            for column in 0..10u32 {
                let (x, y) = (column as f32 * 2. + 1., row as f32 * 2. + 1.);
                let density = (-((x - 11.).powi(2) + (y - 11.).powi(2)) / 32.).exp();
                if density > 0.05 {
                    cells.push(DensityCell {
                        column,
                        row,
                        density,
                        class: 1 + (density * 5.99) as u32,
                    });
                }
            }
        }
        cells
    }

    #[test]
    fn the_grid_holds_the_cells_of_the_query() {
        let cells = bump();
        let grid = DensityGrid::from_cells(&cells, &frame());
        assert_eq!((grid.columns, grid.rows), (10, 10));
        assert!((grid.value(5, 5) - 1.).abs() < 1e-6, "the peak");
        assert_eq!(grid.value(0, 0), 0., "open ground");
        let thresholds = DensityGrid::thresholds(&cells);
        assert_eq!(thresholds.len() as u32, DENSITY_CLASSES - 1);
        assert!(thresholds.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn the_contour_surrounds_a_peak() {
        let grid = DensityGrid::from_cells(&bump(), &frame());
        let segments = grid.contour(0.5, &frame());
        assert!(segments.len() >= 4, "{segments:?}");
        // Every crossing is at about the same distance from the peak: a circle.
        for [(x1, y1), (x2, y2)] in &segments {
            for (x, y) in [(x1, y1), (x2, y2)] {
                let distance = ((x - 11.).powi(2) + (y - 11.).powi(2)).sqrt();
                assert!((3.5..7.).contains(&distance), "{distance} at ({x}, {y})");
            }
        }
    }

    #[test]
    fn the_discs_grow_with_the_losses_within_bounds() {
        assert_eq!(fight_radius(0), 1.5);
        assert_eq!(fight_radius(400), 2.);
        assert_eq!(fight_radius(2500), 5.);
        assert_eq!(fight_radius(100_000), 9.);
        assert_eq!(fight_fill_opacity(5), 0.4);
        assert_eq!(fight_fill_opacity(20), 0.3);
        assert_eq!(fight_fill_opacity(40), 0.25);
    }

    #[test]
    fn the_time_classes_and_colors() {
        assert_eq!(time_classes(10), 5);
        assert_eq!(time_classes(30), 7);
        assert_eq!(time_class(0, 600, 5), 0);
        assert_eq!(time_class(599, 600, 5), 4);
        assert_eq!(time_class(600, 600, 5), 4);
        assert_eq!(time_color(0, 5), TIME_RAMP[0]);
        assert_eq!(time_color(4, 5), TIME_RAMP[6]);
        assert_eq!(time_color(6, 7), TIME_RAMP[6]);
    }
}
