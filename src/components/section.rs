//! Bordered sections, so that the parts of a view are clearly separated. Their header has an
//! arrow that collapses or expands them, remembered on this computer by their title.

use crate::common::storage;
use leptos::prelude::*;
use phosphor_leptos::{Icon, IconWeight, CARET_DOWN, CARET_RIGHT};

/// Whether the section of this title is collapsed, as the viewer left it.
fn collapsed_state(title: &str) -> RwSignal<bool> {
    let key = format!("swarmy.collapsed.{title}");
    let collapsed = RwSignal::new(storage::load(&key).as_deref() == Some("1"));
    Effect::new(move |_| storage::save(&key, if collapsed.get() { "1" } else { "0" }));
    collapsed
}

/// The header of a section: its title and the arrow that collapses it.
#[component]
pub fn CollapsibleHeader(
    title: String,
    collapsed: RwSignal<bool>,
    /// The tailwind classes of the header, i.e. its text size.
    class: &'static str,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=format!(
                "flex w-full items-center gap-1 px-2 py-1 text-left font-semibold text-base-content bg-base-200/80 rounded-t-md hover:bg-base-300/60 {class}",
            )
            class:border-b=move || !collapsed.get()
            style="border-color: color-mix(in oklab, var(--color-base-content) 15%, transparent)"
            aria-expanded=move || (!collapsed.get()).to_string()
            title=move || if collapsed.get() { "Expand this section" } else { "Collapse this section" }
            on:click=move |_| collapsed.update(|collapsed| *collapsed = !*collapsed)
        >
            {move || {
                let icon = if collapsed.get() { CARET_RIGHT } else { CARET_DOWN };
                view! { <Icon icon=icon weight=IconWeight::Bold size="14px" /> }
            }}
            {title.clone()}
        </button>
    }
}

/// A titled section with a fixed height, the content scrolls inside it with a visible
/// scrollbar. Sections side by side with the same height line up.
#[component]
pub fn Section(title: &'static str, height: &'static str, children: Children) -> impl IntoView {
    let collapsed = collapsed_state(title);
    let body = children();
    view! {
        <div class="mt-2 rounded-md border border-base-content/15 bg-base-200/40">
            <CollapsibleHeader title=title.to_string() collapsed class="text-sm" />
            <div class=format!("scroll-section px-2 py-1 {height}") class:hidden=move || collapsed.get()>
                {body}
            </div>
        </div>
    }
}

/// A titled section as high as its content.
#[component]
pub fn Panel(title: &'static str, children: Children) -> impl IntoView {
    let collapsed = collapsed_state(title);
    let body = children();
    view! {
        <div class="rounded-md border border-base-content/15 bg-base-200/40">
            <CollapsibleHeader title=title.to_string() collapsed class="text-base/7" />
            <div class="px-2 py-1" class:hidden=move || collapsed.get()>{body}</div>
        </div>
    }
}
