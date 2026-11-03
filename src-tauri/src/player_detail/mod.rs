//! Statistics of a player: races, results over time, units and opponents.

pub mod data;
pub mod head_to_head;
pub mod insights;
use crate::host::SwarmyHost;
use crate::run_query;
use crate::settings::load_stored_app_settings;
use data::try_query_player_detail;
use head_to_head::{try_list_players, try_query_head_to_head};
use swarmy_common::*;

pub async fn query_player_detail(
    host: &dyn SwarmyHost,
    player_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let groups = crate::personal_db::read_player_groups(host);
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_player_detail(
            app_config.replay_path,
            player_name,
            groups,
        ))
    })
    .await
}

/// The players of the snapshot, for the player dropdowns.
pub async fn list_players(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let groups = crate::personal_db::read_player_groups(host);
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_list_players(app_config.replay_path, groups))
    })
    .await
}

pub async fn query_head_to_head(
    host: &dyn SwarmyHost,
    player_name: String,
    opponent_name: String,
) -> Result<ApiResponse, SwarmyError> {
    let app_config = load_stored_app_settings(host)?;
    let groups = crate::personal_db::read_player_groups(host);
    let query = HeadToHeadQuery {
        player_name,
        opponent_name,
    };
    run_query(move || {
        ApiResponseBuilder::new().process_result(try_query_head_to_head(
            app_config.replay_path,
            query,
            groups,
        ))
    })
    .await
}
