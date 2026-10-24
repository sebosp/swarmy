use crate::settings::WrapAppSettings;
/// Actions for the config module.
use crate::*;
use leptos::leptos_dom::logging::console_log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use swarmy_common::*;

pub fn trigger_set_current_app_config(
    cache_path: String,
    replay_path: String,
    disable_parallelism: bool,
    traverse_max_depth: i64,
    scan_max_files: i64,
    process_max_files: i64,
    min_version: i64,
    max_version: i64,
    set_backend_response: WriteSignal<ApiResponse>,
) {
    let app_settings = AppSettings {
        replay_path,
        cache_path,
        disable_parallelism,
        traverse_max_depth,
        scan_max_files,
        process_max_files,
        min_version,
        max_version,
        ..Default::default()
    };
    let wrapper = WrapAppSettings { app_settings };
    spawn_local(async move {
        let args = serde_wasm_bindgen::to_value(&wrapper).unwrap();
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        match serde_wasm_bindgen::from_value::<()>(invoke("set_current_app_config", args).await) {
            Ok(()) => {
                console_log("Loaded Config");
                set_backend_response.update(|response| {
                    response.meta.complete_with_success();
                    response.message = String::from("Config loaded successfully.");
                });
            }
            Err(e) => {
                console_log(&format!("Error invoking get_current_app_config: {:?}", e));
                set_backend_response.update(|response| {
                    response.meta.complete_with_failure();
                    response.message = format!("Error loading config: {:?}", e);
                });
            }
        }
    });
}
