//! How a unit property changed from one patch to the next.

use leptos::prelude::*;
use swarmy_common::{PatchChange, PatchRowKind};

/// The marker and its color: a red up triangle when the unit costs more (or got worse), a green
/// down triangle when it costs less, green up and red down for the properties that are better
/// the higher they are, i.e. life.
pub fn change_marker(
    kind: PatchRowKind,
    change: PatchChange,
) -> Option<(&'static str, &'static str)> {
    const WORSE_UP: (&str, &str) = ("▲", "text-red-500");
    const BETTER_DOWN: (&str, &str) = ("▼", "text-green-500");
    const BETTER_UP: (&str, &str) = ("▲", "text-green-500");
    const WORSE_DOWN: (&str, &str) = ("▼", "text-red-500");
    match (kind, change) {
        (_, PatchChange::Same) => None,
        (_, PatchChange::Changed) => Some(("●", "text-yellow-400")),
        // A benefit added, i.e. a unit it is strong against, is better.
        (PatchRowKind::Benefit, PatchChange::Added) => Some(BETTER_UP),
        (PatchRowKind::Benefit, PatchChange::Removed) => Some(WORSE_DOWN),
        // Anything else added, i.e. a requirement or an attribute, makes the unit cost more.
        (_, PatchChange::Added) => Some(WORSE_UP),
        (_, PatchChange::Removed) => Some(BETTER_DOWN),
        (PatchRowKind::Benefit, PatchChange::Increased) => Some(BETTER_UP),
        (PatchRowKind::Benefit, PatchChange::Decreased) => Some(WORSE_DOWN),
        (_, PatchChange::Increased) => Some(WORSE_UP),
        (_, PatchChange::Decreased) => Some(BETTER_DOWN),
    }
}

#[component]
pub fn ChangeMarker(kind: PatchRowKind, change: PatchChange) -> impl IntoView {
    change_marker(kind, change).map(|(marker, color)| {
        view! { <span class=format!("{color} mr-1")>{marker}</span> }
    })
}
