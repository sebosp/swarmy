//! The button textures of the units, embedded by s2protocol (the `textures` feature), as PNG data
//! URLs for the frontend.

pub mod dds;

use crate::patch_overview::data::try_list_patch_units;
use crate::replay_rerun::decisions::latest_balance_units;
use std::collections::BTreeMap;
use std::sync::OnceLock;
use swarmy_common::*;

/// Encodes the image as a PNG data URL.
fn png_data_url(image: &dds::RgbaImage) -> Result<String, SwarmyError> {
    crate::common::rgba_png_data_url(image.width, image.height, &image.pixels)
}

/// The icons of the latest balance data, by name: the units (the melee ones with a cost, the
/// ones shown and linked to the Unit Overview), and the levels of their upgrades, the researches
/// of the structures, the abilities and the weapons by their ids, as the Unit Overview lists
/// them. A texture is decoded once however many names share it.
pub fn try_list_unit_icons() -> Result<BTreeMap<String, String>, SwarmyError> {
    let units = try_list_patch_units()?.units;
    let balance = latest_balance_units()?;
    let mut decoded: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut icons = BTreeMap::new();
    let mut add = |name: &str, icon: &str| -> Result<(), SwarmyError> {
        if icons.contains_key(name) {
            return Ok(());
        }
        let key = icon.to_lowercase();
        let url = match decoded.get(&key) {
            Some(url) => url.clone(),
            None => {
                let url = match s2protocol::textures::icon_dds(icon).map(dds::decode_dds) {
                    Some(Some(image)) => Some(png_data_url(&image)?),
                    Some(None) => {
                        tracing::warn!("Unsupported texture format: {icon}");
                        None
                    }
                    None => None,
                };
                decoded.insert(key, url.clone());
                url
            }
        };
        if let Some(url) = url {
            icons.insert(name.to_string(), url);
        }
        Ok(())
    };
    for name in units.into_iter().map(|unit| unit.name) {
        let Some(unit) = balance.get(&name) else {
            continue;
        };
        if let Some(icon) = unit.meta.icon.as_ref() {
            add(&name, icon)?;
        }
        for upgrade in &unit.upgrades.inner {
            for level in &upgrade.levels {
                if let Some(icon) = level.meta.icon.as_ref() {
                    add(&crate::patch_overview::data::level_id(upgrade, level), icon)?;
                }
            }
        }
        for research in unit.researches.iter().flat_map(|r| r.inner.iter()) {
            if let Some(icon) = research.meta.icon.as_ref() {
                add(&research.id, icon)?;
            }
        }
        // What the unit builds and trains, as the game events name the commands
        // (`Build.CreepTumorQueen`, `Train.Marine`), and by the id of what is made.
        for (prefix, made) in [("Build", &unit.builds.inner), ("Train", &unit.trains.inner)] {
            for product in made {
                if let Some(icon) = product.meta.as_ref().and_then(|meta| meta.icon.as_ref()) {
                    add(&format!("{prefix}.{}", product.id), icon)?;
                    add(&product.id, icon)?;
                }
            }
        }
        for weapon in &unit.weapons.inner {
            if let Some(icon) = weapon.meta.icon.as_ref() {
                add(&weapon.id, icon)?;
            }
        }
        // An ability has the icon of its first named command; each named command its own under
        // `Ability.Command`, the name of the game events (`Build.CreepTumorQueen`).
        for ability in &unit.abilities.inner {
            if let Some(icon) = ability
                .commands
                .iter()
                .find(|command| !command.meta.name.is_empty())
                .and_then(|command| command.meta.icon.as_ref())
            {
                add(&ability.id, icon)?;
            }
            for command in &ability.commands {
                if let (Some(id), Some(icon)) = (command.id.as_ref(), command.meta.icon.as_ref())
                    && !command.meta.name.is_empty()
                {
                    add(&format!("{}.{id}", ability.id), icon)?;
                }
            }
        }
    }
    Ok(icons)
}

/// The icons are decoded once.
fn cached_unit_icons() -> Result<UnitIcons, SwarmyError> {
    static CACHE: OnceLock<UnitIcons> = OnceLock::new();
    if let Some(icons) = CACHE.get() {
        return Ok(icons.clone());
    }
    let icons = UnitIcons {
        icons: try_list_unit_icons()?,
    };
    Ok(CACHE.get_or_init(|| icons).clone())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_unit_icons() -> Result<ApiResponse, SwarmyError> {
    // SwarmyError is not Send, it crosses the thread boundary as a String.
    let result = tauri::async_runtime::spawn_blocking(|| cached_unit_icons().map_err(String::from))
        .await
        .map_err(|err| SwarmyError::Other(err.to_string()))?
        .map_err(SwarmyError::Other);
    Ok(ApiResponseBuilder::new().process_result(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_decodes_the_embedded_unit_icons() {
        let icons = try_list_unit_icons().unwrap();
        assert!(icons["Marine"].starts_with("data:image/png;base64,"));
        assert!(icons.contains_key("Hatchery"));
        // The upgrades, researches, abilities and weapons by their ids.
        assert!(icons.contains_key("TerranInfantryWeaponsLevel1"));
        assert!(icons.contains_key("Stimpack"), "the ability of the Marine");
        assert!(icons.contains_key("GuassRifle"), "the weapon of the Marine");
        assert!(icons.contains_key("Burrow"), "a research of the Hatchery");
        assert!(
            icons.contains_key("Build.CreepTumorQueen"),
            "a command of the Queen, as the game events name it"
        );
        let marine =
            dds::decode_dds(s2protocol::textures::icon_dds("btn-unit-terran-marine").unwrap())
                .unwrap();
        assert_eq!((marine.width, marine.height), (76, 76));
    }
}
