//! Bordered sections, so that the parts of a view are clearly separated.

use leptos::prelude::*;

/// A titled section with a fixed height, the content scrolls inside it with a visible
/// scrollbar. Sections side by side with the same height line up.
#[component]
pub fn Section(title: &'static str, height: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="mt-2 rounded-md border border-white/15 bg-gray-800/40">
            <h3 class="px-2 py-1 text-sm font-semibold text-white border-b border-white/15 bg-gray-800/80 rounded-t-md">
                {title}
            </h3>
            <div class=format!("scroll-section px-2 py-1 {height}")>{children()}</div>
        </div>
    }
}

/// A titled section as high as its content.
#[component]
pub fn Panel(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="rounded-md border border-white/15 bg-gray-800/40">
            <h2 class="px-2 py-1 text-base/7 font-semibold text-white border-b border-white/15 bg-gray-800/80 rounded-t-md">
                {title}
            </h2>
            <div class="px-2 py-1">{children()}</div>
        </div>
    }
}
