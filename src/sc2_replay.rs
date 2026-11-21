//! A replay played in StarCraft II and driven by swarmy (`src-tauri/src/sc2_game.rs`): the
//! overlay drawn on top of the game, with the controls of the replay (the game's own do not work
//! while swarmy drives it).

pub mod overlay;

use crate::common::{invoke_json, invoke_response};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::time::Duration;
use swarmy_common::*;

/// The state of the replay StarCraft II plays, read every `period` while the view is shown; never
/// active in the browser version.
pub fn watch_sc2_state(period: Duration) -> RwSignal<Sc2ReplayState> {
    #[derive(serde::Serialize)]
    struct NoArgs {}
    let state = RwSignal::new(Sc2ReplayState::default());
    if crate::components::web_session::is_web() {
        return state;
    }
    let busy = StoredValue::new(false);
    let poll = move || {
        if busy.get_value() {
            return;
        }
        busy.set_value(true);
        spawn_local(async move {
            if let Ok(new) = invoke_json::<Sc2ReplayState>("get_sc2_replay_state", &NoArgs {}).await
                && state.get_untracked() != new
            {
                state.set(new);
            }
            busy.set_value(false);
        });
    };
    poll();
    if let Ok(handle) = set_interval_with_handle(poll, period) {
        on_cleanup(move || handle.clear());
    }
    state
}

/// Shows or hides the overlay window, collapsed to its button or open at the size the user gave it.
pub fn set_overlay_layout(visible: bool, collapsed: bool) {
    #[derive(serde::Serialize)]
    struct Args {
        visible: bool,
        collapsed: bool,
    }
    spawn_local(async move {
        let _ = invoke_response("set_sc2_overlay_layout", &Args { visible, collapsed }).await;
    });
}

/// Sends a control to the replay, `error` tells why it failed.
pub fn send_control(control: Sc2ReplayControl, error: RwSignal<Option<String>>) {
    #[derive(serde::Serialize)]
    struct Args {
        control: Sc2ReplayControl,
    }
    spawn_local(async move {
        match invoke_response("sc2_replay_control", &Args { control }).await {
            Ok(_) => error.set(None),
            Err(err) => error.set(Some(err)),
        }
    });
}
