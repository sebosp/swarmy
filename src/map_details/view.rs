//! Leptos view for map stats.

use leptos::prelude::*;

#[component]
pub fn MapDetails(active_page: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="grid grid-cols-10 gap-1">
            <div class="col-span-4">
                {move || active_page.get()}
                <span class="block text-lg/6 font-large text-white">"Under construction."</span>
            </div>
        </div>
    }
}
