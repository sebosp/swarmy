//! The map terrain from the t3HeightMap of the map cache handles.

use s2protocol::cache_handles::{CacheCollection, MapCache};
use swarmy_common::{MapTerrain, SwarmyError};

/// The minimap is reduced by this factor to keep the responses small.
pub const MINIMAP_DOWNSAMPLE: usize = 2;

/// The height in map units of each terrain level, SC2 cliffs are roughly this high.
pub const TERRAIN_LEVEL_HEIGHT: f32 = 1.5;

/// Terrain level colors from low ground (or void) to the highest ground.
const LEVEL_COLORS: [[u8; 4]; 7] = [
    [0x1f, 0x29, 0x37, 0xff],
    [0x37, 0x41, 0x51, 0xff],
    [0x4b, 0x55, 0x63, 0xff],
    [0x6b, 0x72, 0x80, 0xff],
    [0x9c, 0xa3, 0xaf, 0xff],
    [0xd1, 0xd5, 0xdb, 0xff],
    [0xf3, 0xf4, 0xf6, 0xff],
];

/// The terrain level of each vertex of the map cells grid.
#[derive(Debug, Clone)]
pub struct Terrain {
    /// The vertices per row, one more than the map cells.
    pub width: usize,
    pub height: usize,
    /// The levels, row major starting from the bottom (y = 0) of the map.
    pub levels: Vec<u8>,
    /// The playable area in map cells: left, bottom, right, top.
    pub playable: (usize, usize, usize, usize),
}

/// The replay details cache handles are prefixed by `s2ma`, two null bytes and the region, i.e.
/// `73326d6100005553` for US, the cache files are named after the sha256 that follows.
pub fn cache_handle_id(cache_handle: &str) -> &str {
    const SHA256_HEX_LEN: usize = 64;
    match cache_handle.len().checked_sub(SHA256_HEX_LEN) {
        Some(prefix_len) => &cache_handle[prefix_len..],
        None => cache_handle,
    }
}

/// Loads the map info, height map, description, etc from the cache handles downloaded into
/// `cache_path`.
pub fn load_map_cache(cache_path: &str, cache_handles: &[String]) -> Result<MapCache, SwarmyError> {
    let cache_ids: Vec<String> = cache_handles
        .iter()
        .map(|handle| cache_handle_id(handle).to_string())
        .collect();
    let mut cache = CacheCollection::new(cache_path.to_string());
    cache.add_cache_ids(&cache_ids);
    Ok(cache.build_map_cache(&cache_ids)?)
}

impl Terrain {
    /// Loads the terrain from the cache handles downloaded into `cache_path`.
    pub fn load(cache_path: &str, cache_handles: &[String]) -> Result<Self, SwarmyError> {
        Ok(Self::from(&load_map_cache(cache_path, cache_handles)?))
    }

    /// The downsampled levels to draw the map in the views.
    pub fn minimap(&self) -> MapTerrain {
        let (levels, width, height) = self.downsample(MINIMAP_DOWNSAMPLE);
        MapTerrain {
            levels,
            width: width as u32,
            height: height as u32,
            cell_size: MINIMAP_DOWNSAMPLE as u32,
        }
    }

    /// Returns the level of the closest vertex to the map position.
    pub fn level_at(&self, x: f32, y: f32) -> u8 {
        let x = (x.round().max(0.) as usize).min(self.width - 1);
        let y = (y.round().max(0.) as usize).min(self.height - 1);
        self.levels[y * self.width + x]
    }

    /// Returns the highest level of the vertices of the map cell under the position, the mesh
    /// triangles of a cell can be as high as any of its corners.
    pub fn max_level_around(&self, x: f32, y: f32) -> u8 {
        let clamp_x = |x: f32| (x.max(0.) as usize).min(self.width - 1);
        let clamp_y = |y: f32| (y.max(0.) as usize).min(self.height - 1);
        let (left, right) = (clamp_x(x.floor()), clamp_x(x.ceil()));
        let (bottom, top) = (clamp_y(y.floor()), clamp_y(y.ceil()));
        [(left, bottom), (right, bottom), (left, top), (right, top)]
            .into_iter()
            .map(|(x, y)| self.levels[y * self.width + x])
            .max()
            .unwrap_or_default()
    }

    /// Returns the height of the top of the ground at the map position.
    pub fn z_at(&self, x: f32, y: f32) -> f32 {
        self.max_level_around(x, y) as f32 * TERRAIN_LEVEL_HEIGHT
    }

    /// A mesh of the playable area, two triangles per map cell.
    pub fn mesh(&self) -> rerun::Mesh3D {
        let (left, bottom, right, top) = self.playable;
        let right = right.min(self.width - 1);
        let top = top.min(self.height - 1);
        let columns = right - left + 1;
        let mut positions = Vec::with_capacity(columns * (top - bottom + 1));
        let mut colors = Vec::with_capacity(positions.capacity());
        for y in bottom..=top {
            for x in left..=right {
                let level = self.levels[y * self.width + x];
                positions.push([x as f32, y as f32, level as f32 * TERRAIN_LEVEL_HEIGHT]);
                colors.push(LEVEL_COLORS[(level as usize).min(LEVEL_COLORS.len() - 1)]);
            }
        }
        let vertex = |x: usize, y: usize| ((y - bottom) * columns + (x - left)) as u32;
        let mut triangles = Vec::with_capacity((columns - 1) * (top - bottom) * 2);
        for y in bottom..top {
            for x in left..right {
                triangles.push([vertex(x, y), vertex(x + 1, y), vertex(x + 1, y + 1)]);
                triangles.push([vertex(x, y), vertex(x + 1, y + 1), vertex(x, y + 1)]);
            }
        }
        rerun::Mesh3D::new(positions)
            .with_triangle_indices(triangles)
            .with_vertex_colors(colors)
    }

    /// Returns the max level in each `factor` x `factor` square of the playable area, rows from
    /// the bottom, with the number of columns and rows.
    pub fn downsample(&self, factor: usize) -> (Vec<u8>, usize, usize) {
        let columns = self.width.div_ceil(factor);
        let rows = self.height.div_ceil(factor);
        let mut levels = vec![0u8; columns * rows];
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = &mut levels[(y / factor) * columns + x / factor];
                *cell = (*cell).max(self.levels[y * self.width + x]);
            }
        }
        (levels, columns, rows)
    }
}

impl From<&MapCache> for Terrain {
    fn from(map_cache: &MapCache) -> Self {
        let map_info = &map_cache.map_info;
        let height_map = &map_cache.t3_height_map;
        Self {
            width: height_map.width,
            height: height_map.height,
            levels: height_map.data.clone(),
            playable: (
                map_info.cell_left,
                map_info.cell_bottom,
                map_info.cell_right,
                map_info.cell_top,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terrain() -> Terrain {
        // 3x3 vertices, the top row is high ground.
        Terrain {
            width: 3,
            height: 3,
            levels: vec![1, 1, 1, 1, 1, 1, 3, 3, 3],
            playable: (0, 0, 2, 2),
        }
    }

    #[test]
    fn it_reads_levels_from_the_bottom() {
        let terrain = terrain();
        assert_eq!(terrain.level_at(0., 0.), 1);
        assert_eq!(terrain.level_at(1.2, 2.), 3);
        // Out of bounds positions are clamped.
        assert_eq!(terrain.level_at(-5., 10.), 3);
        assert_eq!(terrain.z_at(0., 2.), 3. * TERRAIN_LEVEL_HEIGHT);
    }

    #[test]
    fn it_uses_the_highest_corner_of_the_cell() {
        let terrain = terrain();
        // Below the cliff the nearest vertex is low ground, the cell rises to the high ground.
        assert_eq!(terrain.level_at(0.6, 1.4), 1);
        assert_eq!(terrain.max_level_around(0.6, 1.4), 3);
        assert_eq!(terrain.max_level_around(1., 0.), 1);
    }

    #[test]
    fn it_strips_the_cache_handle_prefix() {
        let sha = "6de41503baccd05656360b6f027db88169fa1989bb6357b1b215a2547939f5fb";
        assert_eq!(cache_handle_id(&format!("73326d6100005553{sha}")), sha);
        assert_eq!(cache_handle_id(sha), sha);
    }

    #[test]
    fn it_downsamples_with_the_max_level() {
        let (levels, columns, rows) = terrain().downsample(2);
        assert_eq!((columns, rows), (2, 2));
        assert_eq!(levels, vec![1, 1, 3, 3]);
    }
}
