//! What the backend commands need from the program running them: the desktop app, or swarmy
//! started with `--serve` for another swarmy to use as its backend, see `server`.

use crate::majordomo::AsyncTask;
use serde_json::{Map, Value};
use std::path::PathBuf;
use swarmy_common::SwarmyError;
use tauri::{Emitter, Manager};
use tauri_plugin_store::StoreBuilder;
use tokio::sync::mpsc;

/// The store of the settings in the app data directory.
pub const SETTINGS_STORE: &str = "settings.json";

pub trait SwarmyHost: std::fmt::Debug + Send + Sync + 'static {
    /// The app data directory, with the settings store and the personal database.
    fn data_dir(&self) -> Result<PathBuf, SwarmyError>;
    /// The values of the settings store.
    fn read_store(&self) -> Result<Map<String, Value>, SwarmyError>;
    /// Saves a value in the settings store.
    fn write_store(&self, key: &str, value: Value) -> Result<(), SwarmyError>;
    /// Sends an event to the frontends, i.e. the progress of Optimize.
    fn emit(&self, event: &str, payload: Value);
    /// The replays of the replay path were moved or added to the snapshot.
    fn replays_changed(&self);
    /// The coordinator of the long running tasks.
    fn majordomo(&self) -> mpsc::Sender<AsyncTask>;
}

impl SwarmyHost for tauri::AppHandle {
    fn data_dir(&self) -> Result<PathBuf, SwarmyError> {
        self.path()
            .app_data_dir()
            .map_err(|err| SwarmyError::Other(format!("No app data directory: {err}")))
    }

    fn read_store(&self) -> Result<Map<String, Value>, SwarmyError> {
        let store = StoreBuilder::new(self, SETTINGS_STORE).build()?;
        // If there are no saved settings yet, this will return an error so we ignore the return value.
        let _ = store.reload();
        Ok(store.entries().into_iter().collect())
    }

    fn write_store(&self, key: &str, value: Value) -> Result<(), SwarmyError> {
        let store = StoreBuilder::new(self, SETTINGS_STORE).build()?;
        let _ = store.reload();
        store.set(key, value);
        store.save()?;
        Ok(())
    }

    fn emit(&self, event: &str, payload: Value) {
        if let Err(err) = Emitter::emit(self, event, payload) {
            tracing::error!("Error sending the {event} event: {err}");
        }
    }

    fn replays_changed(&self) {
        crate::replay_watcher::refresh(self);
    }

    fn majordomo(&self) -> mpsc::Sender<AsyncTask> {
        self.state::<crate::SetupState>().majordomo_tx.clone()
    }
}
