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

/// The icons of the units of the latest balance data, by unit name.
pub fn try_list_unit_icons() -> Result<BTreeMap<String, String>, SwarmyError> {
    // The melee units with a cost, the ones that are shown and linked to the Unit Overview.
    let units = try_list_patch_units()?.units;
    let balance = latest_balance_units()?;
    let mut icons = BTreeMap::new();
    for name in units.into_iter().map(|unit| unit.name) {
        let Some(icon) = balance.get(&name).and_then(|unit| unit.meta.icon.as_ref()) else {
            continue;
        };
        let Some(texture) = s2protocol::textures::icon_dds(icon) else {
            continue;
        };
        match dds::decode_dds(texture) {
            Some(image) => {
                icons.insert(name, png_data_url(&image)?);
            }
            None => tracing::warn!("Unsupported texture format: {icon}"),
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
        let marine =
            dds::decode_dds(s2protocol::textures::icon_dds("btn-unit-terran-marine").unwrap())
                .unwrap();
        assert_eq!((marine.width, marine.height), (76, 76));
    }
}
