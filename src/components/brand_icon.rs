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

/// The title of a button that opens Rerun or swarmy-bevy: in the browser version they are dimmed
/// and do nothing (the `native-only` class, see `public/web_bridge.js`), the title says why.
pub fn native_title(title: &str) -> String {
    if crate::components::web_session::is_web() {
        format!("{title}. Only in the swarmy app: the browser cannot start Rerun or swarmy-bevy")
    } else {
        title.to_string()
    }
}
