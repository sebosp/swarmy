pub mod changelog;
pub mod common;
pub mod help;
pub mod home;
pub mod map_details;
pub mod map_scout;
pub mod map_stats;
pub mod navigation;
pub mod patch_overview;
pub mod player_detail;
pub mod race_detail;
pub mod replay_detail;
pub mod replay_list;
/// Swarmy Tauri Library
pub mod scan;
pub mod unit_overview;
pub use common::*;
pub mod components;
pub mod settings;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // invoke without arguments
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_without_args(cmd: &str) -> JsValue;
    // invoke with arguments (default)
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
    // listen to a backend event, resolves to the function that stops listening.
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    pub async fn listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> JsValue;
}
