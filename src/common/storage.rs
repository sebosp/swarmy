//! What a viewer prefers on this computer, i.e. the sort of the tables, in the storage of the
//! webview: it can be empty (cleared, a private window), the views work without it.

/// The value saved under the key.
pub fn load(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}

/// Saves the value under the key, the errors are ignored: it is only a preference.
pub fn save(key: &str, value: &str) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let _ = storage.set_item(key, value);
    }
}

/// A preference saved as JSON.
pub fn load_json<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    serde_json::from_str(&load(key)?).ok()
}

pub fn save_json<T: serde::Serialize>(key: &str, value: &T) {
    if let Ok(json) = serde_json::to_string(value) {
        save(key, &json);
    }
}
