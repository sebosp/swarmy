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
        // An ability with its command (`Yoink.Execute`) falls back to the ability (`Yoink`).
        let src = icons?.0.with(|icons| {
            icons.icons.get(&unit_name).cloned().or_else(|| {
                let (ability, _) = unit_name.split_once('.')?;
                icons.icons.get(ability).cloned()
            })
        })?;
        Some(view! {
            <img
                src=src
                alt=""
                class=format!("inline-block {size} mr-1 rounded-sm align-text-bottom")
            />
        })
    }
}

/// The icon of a property of the Unit Overview named after an upgrade level, a research or a
/// weapon (`Upgrade TerranInfantryWeaponsLevel1`, `Research Burrow`, `GuassRifle range`), before
/// the label; the label alone for the other properties.
#[component]
pub fn PropertyLabel(#[prop(into)] label: String) -> impl IntoView {
    let key = ["Upgrade ", "Research "]
        .iter()
        .find_map(|prefix| label.strip_prefix(prefix).map(str::to_string))
        .or_else(|| {
            [" range", " cooldown", " targets"]
                .iter()
                .find_map(|suffix| label.strip_suffix(suffix).map(str::to_string))
        });
    view! {
        {key.map(|key| view! { <UnitIcon unit_name=key size="w-5 h-5" /> })}
        {label}
    }
}

/// The ability ids of the Abilities row of the Unit Overview, one per line with their icons; an
/// ability with energy or a cooldown (`Blink`) is a link to the Ability view, as its name in the
/// other views.
#[component]
pub fn AbilityList(
    #[prop(into)] abilities: String,
    /// Wrapped in the width, instead of one per line.
    #[prop(optional)]
    inline: bool,
) -> impl IntoView {
    let class = if inline {
        "mr-3 whitespace-nowrap"
    } else {
        "block whitespace-nowrap"
    };
    abilities
        .split(", ")
        .filter(|id| !id.is_empty())
        .map(|id| {
            let id = id.to_string();
            let tracked = swarmy_common::TRACKED_ABILITIES
                .iter()
                .find(|tracked| tracked.ability.split('.').next() == Some(id.as_str()));
            view! {
                <span class=class>
                    {match tracked {
                        Some(tracked) => view! {
                            <crate::components::abilities::AbilityName ability=tracked.ability.to_string() />
                        }
                            .into_any(),
                        None => view! {
                            <UnitIcon unit_name=id.clone() />
                            {id}
                        }
                            .into_any(),
                    }}
                </span>
            }
        })
        .collect_view()
}
