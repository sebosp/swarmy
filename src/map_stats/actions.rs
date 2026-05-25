//! Actions related to the Map Stats.
use super::*;
use crate::*;
use leptos::leptos_dom::logging::console_log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use reactive_stores::{Patch, Store};
use swarmy_common::*;

pub async fn fetch_query_map_stats(query: MapStatsQuery) -> Result<ApiResponse, SwarmyError> {
    let args = serde_wasm_bindgen::to_value(&query).unwrap();
    console_log(&format!(
        "Invoking fetch_query_map_stats with args: {:?}",
        args
    ));
    let response =
        serde_wasm_bindgen::from_value::<ApiResponse>(invoke("query_map_stats", args).await)?;
    Ok(response)
}

pub fn trigger_fetch_query_map_stats(
    data: Store<MapStatsDataTable>,
    query: MapStatsQuery,
    set_backend_response: WriteSignal<ApiResponse>,
) {
    *set_backend_response.write() = ApiResponse::new_incomplete();
    spawn_local(async move {
        console_log(&format!("Fetching map stats with query: {:?}", query));
        match fetch_query_map_stats(query).await {
            Err(e) => {
                console_log(&format!("Error fetching map stats: {:?}", e));
                *set_backend_response.write() = ApiResponse {
                    meta: ResponseMeta::incomplete(),
                    message: format!("Error fetching map stats: {:?}", e),
                };
            }
            Ok(response) => {
                console_log(&format!("1. Successfully fetched map stats",));
                *set_backend_response.write() = response.clone();
                let mut rows: Vec<MapStats> =
                    serde_json::from_str(&response.message).unwrap_or_default();
                console_log(&format!("2. Successfully deserialized map stats",));
                data.data().write().retain(|_| false);
                data.total().patch(rows.len());
                data.data().write().append(&mut rows);
            }
        }
    });
}

pub fn trigger_swarmy_bevy_exec_on_caches(map_title: &str, cache_ids: &str) {
    let cache_ids = cache_ids.to_string();
    let map_title = map_title.to_string();
    let swarmy_bevy_params = SwarmyBevyMapCacheParams {
        map_title,
        cache_ids,
    };
    spawn_local(async move {
        let args = serde_wasm_bindgen::to_value(&swarmy_bevy_params).unwrap();
        console_log(&format!(
            "Invoking exec_swarmy_bevy_map_caches with args: {:?}",
            args
        ));
        match serde_wasm_bindgen::from_value::<ApiResponse>(
            invoke("exec_swarmy_bevy_map_caches", args).await,
        ) {
            Err(e) => {
                console_log(&format!(
                    "Error calling exec_swarmy_bevy_map_caches: {:?}",
                    e
                ));
            }
            Ok(response) => {
                console_log(&format!(
                    "1. Successfully exec_swarmy_bevy_map_caches: {:?}",
                    response
                ));
                console_log(&format!(
                    "2. Successfully deserialized map stats: {:?}",
                    response.message
                ));
            }
        }
    });
}
