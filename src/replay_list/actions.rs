//! Actions related to the Replay List
//!
use super::*;
use crate::*;
use leptos::leptos_dom::logging::{console_error, console_log};
use leptos::prelude::*;
use leptos::task::spawn_local;
use reactive_stores::{Patch, Store};
use swarmy_common::*;

pub async fn request_copy_path_to_clipboard(
    replay_file_name: CopyToClipboardContext,
) -> Result<ApiResponse, SwarmyError> {
    let args = serde_wasm_bindgen::to_value(&replay_file_name).unwrap();
    console_log(&format!(
        "Invoking trigger_copy_path_to_clipboard with args: {:?}",
        args
    ));
    let response = serde_wasm_bindgen::from_value::<ApiResponse>(
        invoke("copy_path_to_clipboard", args).await,
    )?;
    Ok(response)
}

pub fn trigger_request_copy_path_to_clipboard(replay_file_name: &str) {
    let replay_file_name = CopyToClipboardContext {
        data: replay_file_name.to_string(),
    };
    spawn_local(async move {
        console_log(&format!(
            "Fetching replay list with query: {:?}",
            replay_file_name
        ));
        match request_copy_path_to_clipboard(replay_file_name).await {
            Ok(_) => console_log("Successfully called request_copy_path_to_clipboard"),
            Err(err) => console_error(&format!("Error request_copy_path_to_clipboard: {:?}", err)),
        };
    });
}

pub async fn request_open_folder(
    replay_file_name: OpenFolderContext,
) -> Result<ApiResponse, SwarmyError> {
    let args = serde_wasm_bindgen::to_value(&replay_file_name).unwrap();
    console_log(&format!(
        "Invoking trigger_open_folder with args: {:?}",
        args
    ));
    let response =
        serde_wasm_bindgen::from_value::<ApiResponse>(invoke("open_folder", args).await)?;
    Ok(response)
}

pub fn trigger_request_open_folder(replay_file_name: &str) {
    let replay_file_name = OpenFolderContext {
        folder: replay_file_name.to_string(),
    };
    spawn_local(async move {
        console_log(&format!(
            "Fetching replay list with query: {:?}",
            replay_file_name
        ));
        match request_open_folder(replay_file_name).await {
            Ok(_) => console_log("Successfully called request_open_folder"),
            Err(err) => console_error(&format!("Error request_open_folder: {:?}", err)),
        };
    });
}
#[derive(serde::Serialize)]
struct ReplayListArgs {
    query: ReplayListQuery,
}

pub async fn fetch_query_replay_list(query: ReplayListQuery) -> Result<ApiResponse, SwarmyError> {
    let args = serde_wasm_bindgen::to_value(&ReplayListArgs { query }).unwrap();
    console_log(&format!(
        "Invoking fetch_query_replay_list with args: {:?}",
        args
    ));
    let response =
        serde_wasm_bindgen::from_value::<ApiResponse>(invoke("query_replay_list", args).await)?;
    Ok(response)
}

pub fn trigger_fetch_query_replay_list(
    data: Store<ReplayListDataTable>,
    query: ReplayListQuery,
    set_backend_response: WriteSignal<ApiResponse>,
) {
    *set_backend_response.write() = ApiResponse::new_incomplete();
    spawn_local(async move {
        console_log(&format!("Fetching replay list with query: {:?}", query));
        match fetch_query_replay_list(query).await {
            Err(e) => {
                *set_backend_response.write() = ApiResponse {
                    meta: ResponseMeta {
                        success: false,
                        duration_ms: 0,
                        is_complete: true,
                    },
                    message: format!("Error fetching replay list: {:?}", e),
                };
            }
            Ok(response) => {
                console_log(&format!("1. Successfully fetched replay list",));
                *set_backend_response.write() = response.clone();
                let mut list: ReplayList =
                    serde_json::from_str(&response.message).unwrap_or_default();
                data.data().write().retain(|_| false);
                data.total().patch(list.total);
                data.data().write().append(&mut list.replays);
            }
        }
    });
}

pub fn trigger_swarmy_rerun_on_replay(replay_file_name: &str) {
    let replay_file_name = replay_file_name.to_string();
    let swarmy_rerun_params = SwarmyRerunParams { replay_file_name };
    spawn_local(async move {
        let args = serde_wasm_bindgen::to_value(&swarmy_rerun_params).unwrap();
        console_log(&format!("Invoking exec_swarmy_rerun with args: {:?}", args));
        match serde_wasm_bindgen::from_value::<ApiResponse>(
            invoke("connect_swarmy_rerun", args).await,
        ) {
            Err(e) => {
                console_log(&format!("Error calling exec_swarmy_rerun: {:?}", e));
            }
            Ok(response) => {
                console_log(&format!(
                    "1. Successfully exec_swarmy_rerun: {:?}",
                    response
                ));
                console_log(&format!(
                    "2. Successfully deserialized replay list: {:?}",
                    response.message
                ));
            }
        }
    });
}

/// Opens the replay in swarmy-bevy, the units born and died over time on its map.
pub fn trigger_swarmy_bevy_on_replay(replay_file_name: &str) {
    trigger_swarmy_bevy_on_replay_at(replay_file_name, None)
}

/// Opens the replay in swarmy-bevy at a game second.
pub fn trigger_swarmy_bevy_on_replay_at(replay_file_name: &str, replay_seconds: Option<u32>) {
    let params = SwarmyBevyReplayParams {
        replay_file_name: replay_file_name.to_string(),
        replay_seconds,
    };
    spawn_local(async move {
        let args = serde_wasm_bindgen::to_value(&params).unwrap();
        match serde_wasm_bindgen::from_value::<ApiResponse>(
            invoke("exec_swarmy_bevy_replay", args).await,
        ) {
            Err(e) => console_log(&format!("Error calling exec_swarmy_bevy_replay: {e:?}")),
            Ok(response) => console_log(&format!("exec_swarmy_bevy_replay: {response:?}")),
        }
    });
}
