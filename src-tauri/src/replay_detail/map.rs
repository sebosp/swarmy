//! The map of a replay: its picture from the map cache, the bases of the players, the epicenters
//! of the engagements, the deaths and their density from the snapshot (`bases`, `epicenters`,
//! `deaths` and `death_density`, the query files).

use super::{bases, death_density, epicenters};
use crate::data::SnapshotSource;
use crate::map_details::{map_picture, open_map_cache};
use crate::replay_detail::valued_units::valued_units;
use swarmy_common::*;

/// Adds the map picture, the bases, the deaths and the epicenters to the replay. Without the map
/// cache they are still there, the Engagements table shows the positions.
pub fn annotate(
    detail: &mut ReplayDetail,
    source: &dyn SnapshotSource,
    cache_path: &str,
    cache_handles: &str,
) -> Result<(), SwarmyError> {
    detail.map = match open_map_cache(cache_path, cache_handles) {
        Ok(map_cache) => Some(map_picture(&map_cache)),
        Err(err) => {
            tracing::warn!("No map cache for replay {}: {err:?}", detail.ext_fs_id);
            None
        }
    };
    detail.bases = bases::shape(bases::plan(source, detail.ext_fs_id)?.collect()?)?;
    let valued = valued_units()?;
    // The density needs the playable area of the map: the frame of the minimap.
    if let Some(image) = detail.map.as_ref().and_then(|map| map.minimap.as_ref()) {
        let frame = death_density::Frame {
            left: image.left,
            bottom: image.bottom,
            right: image.right,
            top: image.top,
        };
        detail.death_density = death_density::shape(
            death_density::plan(
                source,
                detail.ext_fs_id,
                valued.clone(),
                death_density::kernel_offsets()?,
                frame,
            )?
            .collect()?,
        )?;
    }
    let windows = epicenters::engagement_windows(&detail.engagements)?;
    let found =
        epicenters::shape(epicenters::plan(source, detail.ext_fs_id, windows, valued)?.collect()?)?;
    for (position, epicenter) in found {
        if let Some(engagement) = detail.engagements.get_mut(position) {
            engagement.epicenter = Some(epicenter);
        }
    }
    Ok(())
}
