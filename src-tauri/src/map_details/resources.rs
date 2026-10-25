//! The resources and start locations of a map, from the objects placed in its map cache.

use s2protocol::cache_handles::cache_objects::PlacedObjects;
use s2protocol::cache_handles::MapCache;
use swarmy_common::*;

/// The minerals of a large mineral field, since patch 4.7.1. The map caches do not have the
/// amounts unless the map changes them, and the balance data does not have them either.
pub const LARGE_MINERAL_FIELD: u32 = 1800;

/// The minerals of a small mineral field, the `750` types, since patch 4.7.1.
pub const SMALL_MINERAL_FIELD: u32 = 900;

/// The minerals of the reduced mineral fields, the `450` types.
pub const REDUCED_MINERAL_FIELD: u32 = 450;

/// The vespene of a geyser, rich geysers have the same amount but are mined faster.
pub const GEYSER: u32 = 2250;

/// The main base resources are the closest ones to the start location within this distance.
pub const MAIN_BASE_RADIUS: f32 = 12.;

/// The mineral fields and geysers of a main base.
pub const MAIN_MINERAL_FIELDS: usize = 8;
pub const MAIN_GEYSERS: usize = 2;

/// Whether the unit type is a geyser, rich, and its default amount, None for other units.
pub fn resource_type(unit_type: &str) -> Option<(bool, bool, u32)> {
    let is_rich = unit_type.contains("Rich");
    if unit_type.contains("MineralField") {
        let amount = if unit_type.ends_with("750") {
            SMALL_MINERAL_FIELD
        } else if unit_type.ends_with("450") {
            REDUCED_MINERAL_FIELD
        } else {
            LARGE_MINERAL_FIELD
        };
        Some((false, is_rich, amount))
    } else if unit_type.contains("Geyser") {
        Some((true, is_rich, GEYSER))
    } else {
        None
    }
}

/// Parses the `x,y,z` position of a placed object.
fn position(position: &Option<String>) -> Option<(f32, f32)> {
    let mut coords = position.as_deref()?.split(',');
    let x = coords.next()?.trim().parse().ok()?;
    let y = coords.next()?.trim().parse().ok()?;
    Some((x, y))
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// The distances to the closest resources within [`MAIN_BASE_RADIUS`], at most `count`.
fn closest_distances<'a>(
    from: (f32, f32),
    resources: impl Iterator<Item = &'a MapResource>,
    count: usize,
) -> Vec<f32> {
    let mut distances: Vec<f32> = resources
        .map(|resource| distance(from, (resource.x, resource.y)))
        .filter(|distance| *distance <= MAIN_BASE_RADIUS)
        .collect();
    distances.sort_by(f32::total_cmp);
    distances.truncate(count);
    distances
}

fn average(values: &[f32]) -> f32 {
    if values.is_empty() {
        0.
    } else {
        values.iter().sum::<f32>() / values.len() as f32
    }
}

/// The resources and the start locations with their main base resources.
pub fn map_resources(objects: &PlacedObjects) -> MapResources {
    let mut summary = MapResources::default();
    for unit in &objects.units {
        let (Some((is_geyser, is_rich, default_amount)), Some((x, y))) =
            (resource_type(&unit.unit_kind), position(&unit.position))
        else {
            continue;
        };
        let amount = unit.resources.unwrap_or(default_amount);
        if unit.resources.is_none() {
            summary.default_amounts += 1;
        }
        if is_geyser {
            summary.geysers += 1;
            summary.rich_geysers += is_rich as usize;
            summary.vespene += amount;
        } else {
            summary.mineral_fields += 1;
            summary.rich_mineral_fields += is_rich as usize;
            summary.minerals += amount;
        }
        summary.resources.push(MapResource {
            unit_type: unit.unit_kind.clone(),
            x,
            y,
            is_geyser,
            is_rich,
            amount,
        });
    }
    for point in objects
        .points
        .iter()
        .filter(|point| point.kind == "StartLoc")
    {
        let Some(start) = position(&point.position) else {
            continue;
        };
        let minerals = closest_distances(
            start,
            summary
                .resources
                .iter()
                .filter(|resource| !resource.is_geyser),
            MAIN_MINERAL_FIELDS,
        );
        let geysers = closest_distances(
            start,
            summary
                .resources
                .iter()
                .filter(|resource| resource.is_geyser),
            MAIN_GEYSERS,
        );
        summary.start_locations.push(StartLocation {
            name: point.name.clone(),
            x: start.0,
            y: start.1,
            mineral_fields: minerals.len(),
            avg_mineral_distance: average(&minerals),
            min_mineral_distance: minerals.first().copied().unwrap_or_default(),
            max_mineral_distance: minerals.last().copied().unwrap_or_default(),
            avg_geyser_distance: average(&geysers),
        });
    }
    summary.start_locations.sort_by(|a, b| a.name.cmp(&b.name));
    summary
}

/// The minimap cropped to the playable area as a PNG, None if the map cache does not have it.
pub fn map_image(map_cache: &MapCache) -> Option<MapImage> {
    let map_info = &map_cache.map_info;
    let minimap = map_cache.minimap.as_ref()?.crop_to_playable(map_info);
    let data_url = crate::common::rgba_png_data_url(minimap.width, minimap.height, &minimap.pixels)
        .inspect_err(|err| tracing::warn!("Unable to encode the minimap: {err:?}"))
        .ok()?;
    Some(MapImage {
        data_url,
        left: map_info.cell_left as f32,
        bottom: map_info.cell_bottom as f32,
        right: map_info.cell_right as f32,
        top: map_info.cell_top as f32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_classifies_the_resources() {
        assert_eq!(
            resource_type("MineralField"),
            Some((false, false, LARGE_MINERAL_FIELD))
        );
        assert_eq!(
            resource_type("LabMineralField750"),
            Some((false, false, SMALL_MINERAL_FIELD))
        );
        assert_eq!(
            resource_type("RichMineralField750"),
            Some((false, true, SMALL_MINERAL_FIELD))
        );
        assert_eq!(
            resource_type("RichVespeneGeyser"),
            Some((true, true, GEYSER))
        );
        assert_eq!(
            resource_type("SpacePlatformGeyser"),
            Some((true, false, GEYSER))
        );
        assert_eq!(resource_type("DestructibleRockEx16x6"), None);
    }

    #[test]
    fn it_measures_the_main_base_resources() {
        let xml = r#"<PlacedObjects Version="27">
            <ObjectPoint Id="1" Position="10,10,0" Scale="1,1,1" Type="StartLoc" Name="Start Location 001"/>
            <ObjectUnit Id="2" Position="16,10,0" Scale="1,1,1" UnitType="MineralField"/>
            <ObjectUnit Id="3" Position="10,18,0" Scale="1,1,1" UnitType="RichMineralField750"/>
            <ObjectUnit Id="4" Position="50,50,0" Scale="1,1,1" UnitType="MineralField" Resources="1000"/>
            <ObjectUnit Id="5" Position="13,6,0" Scale="1,1,1" UnitType="VespeneGeyser"/>
        </PlacedObjects>"#;
        let objects = PlacedObjects::parse("id".to_string(), xml.as_bytes()).unwrap();
        let resources = map_resources(&objects);
        assert_eq!(resources.mineral_fields, 3);
        assert_eq!(resources.rich_mineral_fields, 1);
        assert_eq!(resources.minerals, 1800 + 900 + 1000);
        assert_eq!(resources.vespene, GEYSER);
        assert_eq!(resources.default_amounts, 3);
        let start = &resources.start_locations[0];
        // The mineral field at 50,50 is not part of the main base.
        assert_eq!(start.mineral_fields, 2);
        assert_eq!(start.avg_mineral_distance, 7.);
        assert_eq!(start.avg_geyser_distance, 5.);
    }
}
