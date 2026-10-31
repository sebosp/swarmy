//! Map Details module.

pub mod data;
pub mod map_layers;
pub mod resources;
pub mod siege_spots;
use crate::host::SwarmyHost;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::{try_list_maps, try_query_map_details};
use swarmy_common::*;

pub async fn query_map_details(
    host: &dyn SwarmyHost,
    map_title: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_map_details(
            app_config.replay_path,
            app_config.cache_path,
            map_title,
        ))
    })
    .await
}

/// The maps of the snapshot, for the map dropdown.
pub async fn list_maps(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_list_maps(app_config.replay_path))
    })
    .await
}
