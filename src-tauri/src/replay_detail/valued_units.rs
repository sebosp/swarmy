//! The units and structures that cost resources: a frame parameter of the Deaths of a replay
//! (`deaths`, `death_density`, `epicenters`). Was in the Map Scout (`map_scout::death_points`)
//! until it was removed on 2026-11-12.

use crate::patch_overview::unit_stats::unit_variants;
use crate::replay_rerun::decisions::{has_cost, latest_balance_units};
use polars::prelude::*;
use swarmy_common::*;

/// The table name of the [`valued_units`] frame, for the SQL of the READMEs.
pub const VALUED_UNITS_IPC: &str = "valued_units.arrow";

/// The units and structures that cost resources in the latest balance data, with the names of
/// their variants: `variant` (`SiegeTankSieged`, `RoachBurrowed`) and `unit` (`SiegeTank`,
/// `Roach`). A Larva, a Broodling or an InvisibleTargetDummy dying is not a loss.
pub fn valued_units() -> Result<DataFrame, SwarmyError> {
    let balance = latest_balance_units()?;
    let (variants, units): (Vec<String>, Vec<String>) = balance
        .keys()
        .filter(|name| has_cost(balance, name))
        .flat_map(|name| {
            unit_variants(name)
                .into_iter()
                .map(|variant| (variant, name.clone()))
        })
        .unzip();
    Ok(df!("variant" => variants, "unit" => units)?)
}
