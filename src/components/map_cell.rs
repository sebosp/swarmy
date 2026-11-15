//! The cells of a map in Map Details and the Replay view: hovering one highlights it on the map,
//! with a line across its x, one across its y and a square (or a circle) on it; hovering a group
//! of spots (the structures of an opening) highlights them all.

use leptos::prelude::*;

/// A spot of the map to highlight, in map cells: its lower left corner and its size; `round`
/// draws a circle of that diameter instead of a square, for a radius around a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapHighlight {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub round: bool,
}

impl MapHighlight {
    /// A square of `size` cells around a center, i.e. a structure at its position.
    pub fn around(center: (f32, f32), size: f32) -> Self {
        Self {
            x: center.0 - size / 2.,
            y: center.1 - size / 2.,
            size,
            round: false,
        }
    }

    /// The center of the spot.
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.size / 2., self.y + self.size / 2.)
    }
}

/// The spots hovered in the view, provided by the view and read by its map; empty when nothing
/// is hovered.
#[derive(Debug, Clone, Copy)]
pub struct MapHighlightContext(pub RwSignal<Vec<MapHighlight>>);

impl MapHighlightContext {
    pub fn new() -> Self {
        Self(RwSignal::new(vec![]))
    }
}

impl Default for MapHighlightContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A position written the same way in every view, `(x, y)`, without decimals when it is a whole
/// cell.
pub fn format_cell(x: f32, y: f32) -> String {
    let coordinate = |value: f32| {
        if value.fract() == 0. {
            format!("{value:.0}")
        } else {
            format!("{value:.1}")
        }
    };
    format!("({}, {})", coordinate(x), coordinate(y))
}

/// A cell, or a square of `size` cells from it, written as `(x, y)`: hovering or focusing it
/// highlights it on the map of the view. `centered` positions are the center of the spot, i.e. a
/// start location, the others its lower left corner; `round` highlights a circle of `size`
/// diameter around it, i.e. the spread of a fight. `title` replaces the default help.
#[component]
pub fn MapCell(
    x: f32,
    y: f32,
    #[prop(default = 1.)] size: f32,
    #[prop(optional)] centered: bool,
    #[prop(optional)] round: bool,
    #[prop(optional)] title: Option<String>,
) -> impl IntoView {
    let spot = if centered || round {
        MapHighlight {
            round,
            ..MapHighlight::around((x, y), size)
        }
    } else {
        MapHighlight { x, y, size, round }
    };
    let title = title.unwrap_or_else(|| "Show coords on map".to_string());
    view! {
        <MapSpots spots=vec![spot] title>
            {format_cell(x, y)}
        </MapSpots>
    }
}

/// Text that highlights several spots of the map while hovered or focused, i.e. the structures
/// of an opening; nothing special without spots.
#[component]
pub fn MapSpots(
    spots: Vec<MapHighlight>,
    #[prop(optional)] title: Option<String>,
    children: Children,
) -> impl IntoView {
    let highlight = use_context::<MapHighlightContext>();
    if spots.is_empty() {
        return children().into_any();
    }
    let title = title.unwrap_or_else(|| {
        if spots.len() == 1 {
            "Show coords on map".to_string()
        } else {
            "Show the coords on map".to_string()
        }
    });
    let shown = spots.clone();
    let show = move || {
        if let Some(MapHighlightContext(signal)) = highlight {
            signal.set(shown.clone());
        }
    };
    let hide = move || {
        if let Some(MapHighlightContext(signal)) = highlight {
            signal.set(vec![]);
        }
    };
    let focus = show.clone();
    view! {
        <span
            class="map-cell"
            tabindex="0"
            title=title
            on:mouseenter=move |_| show()
            on:mouseleave=move |_| hide()
            on:focus=move |_| focus()
            on:blur=move |_| hide()
        >
            {children()}
        </span>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_formats_the_cells_the_same_way() {
        assert_eq!(format_cell(36., 120.), "(36, 120)");
        assert_eq!(format_cell(36.5, 120.25), "(36.5, 120.2)");
    }

    #[test]
    fn a_spot_around_a_center() {
        let spot = MapHighlight::around((10., 20.), 4.);
        assert_eq!((spot.x, spot.y), (8., 18.));
        assert_eq!(spot.center(), (10., 20.));
    }
}
