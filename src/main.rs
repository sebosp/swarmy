mod app;

use app::*;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    // The overlay of a replay played in StarCraft II is a window of its own, alone in it.
    let hash = window().location().hash().unwrap_or_default();
    if hash.starts_with(swarmy_ui::sc2_replay::overlay::OVERLAY_ROUTE) {
        mount_to_body(|| view! { <swarmy_ui::sc2_replay::overlay::Sc2Overlay /> })
    } else {
        mount_to_body(|| view! { <Main /> })
    }
}
