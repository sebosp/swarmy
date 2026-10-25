//! The button textures of the units, from the Assets/Textures directory of the balance data
//! exported from the SC2 editor, i.e. ~/SC2Replays/BalanceData/<patch version>/Assets/Textures.

pub mod dds;

use crate::patch_overview::data::try_list_patch_units;
use crate::replay_rerun::decisions::latest_balance_units;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use swarmy_common::*;

/// The directories searched when the balance data path is not configured, relative to the replay
/// path.
const DEFAULT_BALANCE_DATA_DIRS: [&str; 2] = ["BalanceData", "../BalanceData"];

/// The Assets/Textures directory of the newest patch in the balance data directory.
fn newest_textures_dir(balance_data_path: &Path) -> Option<PathBuf> {
    std::fs::read_dir(balance_data_path)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let version: u32 = entry.file_name().to_str()?.parse().ok()?;
            let textures = entry.path().join("Assets").join("Textures");
            textures.is_dir().then_some((version, textures))
        })
        .max_by_key(|(version, _)| *version)
        .map(|(_, textures)| textures)
}

fn resolve_textures_dir(settings: &AppSettings) -> Option<PathBuf> {
    if !settings.balance_data_path.is_empty() {
        return newest_textures_dir(Path::new(&settings.balance_data_path));
    }
    DEFAULT_BALANCE_DATA_DIRS
        .iter()
        .find_map(|dir| newest_textures_dir(&Path::new(&settings.replay_path).join(dir)))
}

/// Encodes the image as a PNG data URL.
fn png_data_url(image: &dds::RgbaImage) -> Result<String, SwarmyError> {
    crate::common::rgba_png_data_url(image.width, image.height, &image.pixels)
}

/// The icons of the units of the latest balance data, by unit name.
pub fn try_list_unit_icons(textures_dir: &Path) -> Result<BTreeMap<String, String>, SwarmyError> {
    // The icon names in the balance data do not always match the case of the file names.
    let textures: HashMap<String, PathBuf> = std::fs::read_dir(textures_dir)?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let stem = path.file_stem()?.to_str()?.to_lowercase();
            (path.extension()? == "dds").then_some((stem, path))
        })
        .collect();
    // The melee units with a cost, the ones that are shown and linked to the Unit Overview.
    let units = try_list_patch_units()?.units;
    let balance = latest_balance_units()?;
    let mut icons = BTreeMap::new();
    for name in units.into_iter().map(|unit| unit.name) {
        let Some(icon) = balance.get(&name).and_then(|unit| unit.meta.icon.as_ref()) else {
            continue;
        };
        let Some(path) = textures.get(&icon.to_lowercase()) else {
            continue;
        };
        match dds::decode_dds(&std::fs::read(path)?) {
            Some(image) => {
                icons.insert(name, png_data_url(&image)?);
            }
            None => tracing::warn!("Unsupported texture format: {}", path.display()),
        }
    }
    Ok(icons)
}

/// The icons are decoded once per textures directory.
fn cached_unit_icons(textures_dir: &Path) -> Result<UnitIcons, SwarmyError> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, UnitIcons>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(icons) = cache
        .lock()
        .ok()
        .and_then(|cache| cache.get(textures_dir).cloned())
    {
        return Ok(icons);
    }
    let icons = UnitIcons {
        textures_dir: textures_dir.display().to_string(),
        icons: try_list_unit_icons(textures_dir)?,
    };
    if let Ok(mut cache) = cache.lock() {
        cache.insert(textures_dir.to_path_buf(), icons.clone());
    }
    Ok(icons)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_unit_icons(app_handle: tauri::AppHandle) -> Result<ApiResponse, SwarmyError> {
    let settings = crate::settings::load_stored_app_settings(&app_handle)?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        match resolve_textures_dir(&settings) {
            Some(textures_dir) => cached_unit_icons(&textures_dir).map_err(|err| err.to_string()),
            // Without the textures the names are shown without icons.
            None => Ok(UnitIcons::default()),
        }
    })
    .await
    .map_err(|err| SwarmyError::Other(err.to_string()))?
    .map_err(SwarmyError::Other);
    Ok(ApiResponseBuilder::new().process_result(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_decodes_the_unit_icons_of_the_balance_data() {
        let balance_data = Path::new(env!("HOME")).join("SC2Replays/BalanceData");
        let Some(textures_dir) = newest_textures_dir(&balance_data) else {
            // The balance data with textures is exported locally, it is not in the repository.
            return;
        };
        let icons = try_list_unit_icons(&textures_dir).unwrap();
        assert!(icons["Marine"].starts_with("data:image/png;base64,"));
        assert!(icons.contains_key("Hatchery"));
        let marine = dds::decode_dds(
            &std::fs::read(textures_dir.join("btn-unit-terran-marine.dds")).unwrap(),
        )
        .unwrap();
        assert_eq!((marine.width, marine.height), (76, 76));
        println!(
            "{} icons, {} bytes",
            icons.len(),
            icons.values().map(String::len).sum::<usize>()
        );
    }
}
