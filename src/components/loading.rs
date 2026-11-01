//! Shown while the backend collects the data of a view.

use leptos::prelude::*;

/// A spinner with what is being collected.
#[component]
pub fn Loading(#[prop(into)] message: String) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2 py-4 text-sm text-base-content/80">
            <span class="loading loading-spinner loading-sm text-primary"></span>
            {message}
        </div>
    }
}
