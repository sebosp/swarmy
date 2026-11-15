#![doc = include_str!("death_density/README.md")]

use crate::data::*;
use polars::prelude::*;
use swarmy_common::*;

pub const META: QueryMeta = query_meta!();

/// The table name of the [`kernel_offsets`] frame, for the SQL of the README.
pub const OFFSETS_IPC: &str = "kernel_offsets.arrow";

/// The bandwidth of the kernel, in map units, is kept between these: the Silverman rule assumes
/// one blob and oversmooths a game with fights at four bases; the tools (d3-contour, QGIS,
/// ArcGIS) land on 6 to 8 cells for a map of this size; under 5 the deaths show one by one, over
/// 12 separate battles merge.
pub const MIN_BANDWIDTH: f32 = 5.;
pub const MAX_BANDWIDTH: f32 = 9.;

/// The cells a death reaches, in cells of [`DENSITY_CELL`] each way: three times the largest
/// bandwidth, beyond which a Gaussian is nothing.
pub const KERNEL_REACH: i32 = (3. * MAX_BANDWIDTH / DENSITY_CELL) as i32 + 1;

/// Under this share of the peak density a cell is open ground, left out.
pub const FLOOR_SHARE: f32 = 0.05;

/// The playable area of the map in map units, the frame of the density grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}

impl Frame {
    /// The cells across and up.
    pub fn cells(&self) -> (u32, u32) {
        (
            ((self.right - self.left) / DENSITY_CELL).ceil().max(1.) as u32,
            ((self.top - self.bottom) / DENSITY_CELL).ceil().max(1.) as u32,
        )
    }
}

/// The offsets of the cells around a death, `dx` and `dy` (i32, in cells) within
/// [`KERNEL_REACH`]: a frame parameter of [`plan`], the kernel is summed over them.
pub fn kernel_offsets() -> Result<DataFrame, SwarmyError> {
    let mut dxs = vec![];
    let mut dys = vec![];
    for dy in -KERNEL_REACH..=KERNEL_REACH {
        for dx in -KERNEL_REACH..=KERNEL_REACH {
            dxs.push(dx);
            dys.push(dy);
        }
    }
    Ok(df!("dx" => dxs, "dy" => dys)?)
}

/// The density of the deaths of the game over the cells of the frame, a Gaussian kernel density
/// estimate: the deaths that cost resources (`deaths`, the query of the same name) are binned
/// to the cells, the bandwidth is the Silverman and Scott rule for two dimensions
/// (`n^(-1/6)` times the standard distance of the deaths) within [`MIN_BANDWIDTH`] and
/// [`MAX_BANDWIDTH`], each bin spreads its deaths over the cells of `offsets` within three
/// bandwidths; the cells under [`FLOOR_SHARE`] of the peak are left out and the others are cut
/// in [`DENSITY_CLASSES`] classes of the same area by their rank: `column`, `row` (u32),
/// `density` (f32, deaths per cell) and `class` (u32, 1 the sparsest); by row and column.
pub fn plan(
    source: &dyn SnapshotSource,
    ext_fs_id: u64,
    valued_units: DataFrame,
    offsets: DataFrame,
    frame: Frame,
) -> Result<LazyFrame, SwarmyError> {
    let (columns, rows) = frame.cells();
    let cell = lit(DENSITY_CELL);
    let deaths = super::deaths::plan(source, ext_fs_id, valued_units)?;
    // The bandwidth, one row: the Silverman and Scott rule within the bounds, in map units.
    let standard_distance = ((col("x").var(0) + col("y").var(0)) / lit(2.)).sqrt();
    let bandwidth = deaths.clone().select([(standard_distance
        * len().cast(DataType::Float32).pow(lit(-1. / 6.)))
    .clip(lit(MIN_BANDWIDTH), lit(MAX_BANDWIDTH))
    .fill_null(lit(MIN_BANDWIDTH))
    .alias("bandwidth")]);
    // The deaths by cell.
    let bins = deaths
        .select([
            ((col("x") - lit(frame.left)) / cell.clone())
                .floor()
                .cast(DataType::Int32)
                .alias("column"),
            ((col("y") - lit(frame.bottom)) / cell.clone())
                .floor()
                .cast(DataType::Int32)
                .alias("row"),
        ])
        .group_by([col("column"), col("row")])
        .agg([len().alias("deaths")]);
    // Each bin spreads its deaths over the cells around, with the Gaussian weight of the distance
    // between the cell centers (the density is per cell: the kernel is scaled by the cell area).
    let two_h2 = lit(2.) * col("bandwidth") * col("bandwidth");
    let distance2 = (col("dx").cast(DataType::Float32) * cell.clone()).pow(2)
        + (col("dy").cast(DataType::Float32) * cell.clone()).pow(2);
    let spread = bins
        .cross_join(bandwidth, None)
        .cross_join(offsets.lazy(), None)
        .with_column(distance2.alias("distance2"))
        .filter(col("distance2").lt_eq(lit(9.) * col("bandwidth") * col("bandwidth")))
        .select([
            (col("column") + col("dx")).alias("column"),
            (col("row") + col("dy")).alias("row"),
            (col("deaths").cast(DataType::Float32)
                * (cell.clone() * cell.clone() / (lit(std::f32::consts::PI) * two_h2.clone()))
                * (lit(0.) - col("distance2") / two_h2).exp())
            .alias("weight"),
        ])
        .filter(
            col("column")
                .gt_eq(lit(0i32))
                .and(col("column").lt(lit(columns as i32)))
                .and(col("row").gt_eq(lit(0i32)))
                .and(col("row").lt(lit(rows as i32))),
        )
        .group_by([col("column"), col("row")])
        .agg([col("weight").sum().cast(DataType::Float32).alias("density")]);
    // The cells above the floor, in classes of the same area by rank.
    Ok(spread
        .filter(col("density").gt(col("density").max() * lit(FLOOR_SHARE)))
        .sort_by_exprs(
            [col("density"), col("row"), col("column")],
            Default::default(),
        )
        .with_column(
            (lit(1u32)
                + (int_range(lit(0), len(), 1, DataType::UInt32) * lit(DENSITY_CLASSES)
                    / len().cast(DataType::UInt32)))
            .alias("class"),
        )
        .select([
            col("column").cast(DataType::UInt32),
            col("row").cast(DataType::UInt32),
            col("density"),
            col("class"),
        ])
        .sort_by_exprs([col("row"), col("column")], Default::default()))
}

/// The cells as the map draws them.
pub fn shape(frame: DataFrame) -> Result<Vec<DensityCell>, SwarmyError> {
    Ok(u32_values(&frame, "column")?
        .into_iter()
        .zip(u32_values(&frame, "row")?)
        .zip(f32_values(&frame, "density")?)
        .zip(u32_values(&frame, "class")?)
        .map(|(((column, row), density), class)| DensityCell {
            column,
            row,
            density,
            class,
        })
        .collect())
}

#[cfg(test)]
mod tests;
