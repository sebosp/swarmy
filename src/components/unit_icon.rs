//! The button icons of the units, from the textures of the balance data, loaded once for all
//! the views.

use crate::invoke_json;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::UnitIcons;

/// The icons shared by the views, empty until loaded or when there are no textures.
#[derive(Debug, Clone, Copy)]
pub struct UnitIconsContext(pub RwSignal<UnitIcons>);

/// Provides the icons context and loads the icons.
pub fn provide_unit_icons() {
    let icons = UnitIconsContext(RwSignal::new(UnitIcons::default()));
    provide_context(icons);
    load_unit_icons(icons);
}

/// Loads the icons, again after the balance data path changes.
pub fn load_unit_icons(icons: UnitIconsContext) {
    spawn_local(async move {
        match invoke_json::<UnitIcons>("list_unit_icons", &()).await {
            Ok(loaded) => icons.0.set(loaded),
            Err(err) => leptos::logging::error!("Error loading the unit icons: {err}"),
        }
    });
}

/// The button icon of the unit, nothing when there is no texture for it.
#[component]
pub fn UnitIcon(
    #[prop(into)] unit_name: String,
    /// The size, i.e. "w-4 h-4".
    #[prop(default = "w-4 h-4")]
    size: &'static str,
) -> impl IntoView {
    let icons = use_context::<UnitIconsContext>();
    move || {
        let src = icons?
            .0
            .with(|icons| icons.icons.get(&unit_name).cloned())?;
        Some(view! {
            <img
                src=src
                alt=""
                class=format!("inline-block {size} mr-1 rounded-sm align-text-bottom")
            />
        })
    }
}
