//! The logos of the tools swarmy opens a replay or a map in, Rerun and Bevy (swarmy-bevy), as
//! icons in the color of the text: a CSS mask of the logo, so that they follow the theme.

use leptos::prelude::*;

/// `rerun` or `bevy`.
#[component]
pub fn BrandIcon(name: &'static str) -> impl IntoView {
    let file = match name {
        "rerun" => "public/icons/rerun.png",
        _ => "public/icons/bevy.svg",
    };
    view! { <span class="brand-icon" aria-hidden="true" style=format!("--brand-icon: url({file})")></span> }
}
