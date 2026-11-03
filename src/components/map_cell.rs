//! The cells of a map in Map Details: hovering one highlights it on the map, with a line across
//! its x, one across its y and a square on it.

use leptos::prelude::*;

/// A spot of the map to highlight, in map cells: its lower left corner and its size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapHighlight {
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

impl MapHighlight {
    /// The center of the spot.
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.size / 2., self.y + self.size / 2.)
    }
}

/// The spot hovered in the view, provided by Map Details and read by its map.
#[derive(Debug, Clone, Copy)]
pub struct MapHighlightContext(pub RwSignal<Option<MapHighlight>>);

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
/// start location, the others its lower left corner.
#[component]
pub fn MapCell(
    x: f32,
    y: f32,
    #[prop(default = 1.)] size: f32,
    #[prop(optional)] centered: bool,
) -> impl IntoView {
    let highlight = use_context::<MapHighlightContext>();
    let spot = if centered {
        MapHighlight {
            x: x - size / 2.,
            y: y - size / 2.,
            size,
        }
    } else {
        MapHighlight { x, y, size }
    };
    let show = move || {
        if let Some(MapHighlightContext(signal)) = highlight {
            signal.set(Some(spot));
        }
    };
    let hide = move || {
        if let Some(MapHighlightContext(signal)) = highlight {
            signal.set(None);
        }
    };
    view! {
        <span
            class="map-cell"
            tabindex="0"
            title="Hover to see this spot on the map"
            on:mouseenter=move |_| show()
            on:mouseleave=move |_| hide()
            on:focus=move |_| show()
            on:blur=move |_| hide()
        >
            {format_cell(x, y)}
        </span>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_formats_the_cells_the_same_way() {
        assert_eq!(format_cell(36., 120.), "(36, 120)");
        assert_eq!(format_cell(36.5, 120.25), "(36.5, 120.2)");
    }
}
