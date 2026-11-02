//! The commands of the snapshot: they run in the app, or in the remote backend of Config, see
//! `remote` and `server`. Each one is a function of its module that takes the host first, the
//! table below makes from it the Tauri command, and the route of the server.

use crate::host::SwarmyHost;
use serde_json::Value;
use std::collections::BTreeMap;
use swarmy_common::*;

macro_rules! backend_commands {
    ($($module:ident :: $name:ident ( $($arg:ident : $ty:ty),* );)*) => {
        // The commands are private, tauri::command exports the macros of public ones and the
        // exported macros of a macro expansion cannot be used by path.
        $(
            #[tauri::command(rename_all = "snake_case")]
            async fn $name(
                app_handle: tauri::AppHandle,
                $($arg: $ty),*
            ) -> Result<ApiResponse, SwarmyError> {
                crate::$module::$name(&app_handle $(, $arg)*).await
            }
        )*

        /// The commands that run in the remote backend when one is set.
        pub const BACKEND_COMMANDS: &[&str] = &[$(stringify!($name)),*];

        /// The handler of the commands in the app.
        pub fn backend_handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
            tauri::generate_handler![$($name),*]
        }

        /// Runs a command with its arguments as a JSON object, the ones of the frontend.
        pub async fn dispatch(
            host: &dyn SwarmyHost,
            command: &str,
            args: Value,
        ) -> Result<ApiResponse, SwarmyError> {
            match command {
                $(stringify!($name) => {
                    #[derive(serde::Deserialize)]
                    struct Args {
                        $($arg: $ty),*
                    }
                    let Args { $($arg),* } = serde_json::from_value(args)?;
                    crate::$module::$name(host $(, $arg)*).await
                })*
                _ => Err(SwarmyError::Other(format!("Unknown command {command}"))),
            }
        }
    };
}

backend_commands! {
    mpq_file_scan::basic_scan_replay_path();
    mpq_file_scan::optimize_replay_path(
        replay_path: String,
        cache_path: String,
        disable_parallelism: bool,
        recreate: bool
    );
    snapshot_stats::get_snapshot_metadata();
    map_stats::query_map_stats(map_title: String, player_name: String);
    swarmy_bevy::query_siege_spots(map_title: String);
    swarmy_bevy::query_map_layers(map_title: String);
    replay_list::query_replay_list(query: ReplayListQuery);
    replay_list::list_replay_filters();
    replay_detail::query_replay_detail(ext_fs_id: u64);
    replay_detail::query_adjacent_replay(query: AdjacentReplayQuery);
    replay_detail::render_replay_report(ext_fs_id: u64);
    personal_db::get_replay_notes(sha256: String);
    personal_db::set_replay_notes(notes: ReplayNotes);
    personal_db::list_replay_tags();
    duplicates::find_duplicate_replays();
    duplicates::move_duplicate_replays();
    player_detail::query_player_detail(player_name: String);
    player_detail::list_players();
    player_detail::query_head_to_head(player_name: String, opponent_name: String);
    map_scout::query_map_scout(
        map_title: String,
        player_name: String,
        unit_name: String,
        max_seconds: u32,
        layer: ScoutLayer
    );
    map_details::query_map_details(map_title: String);
    map_details::list_maps();
    patch_overview::query_unit_snapshot_stats(unit_name: String);
    ability_usage::query_ability_usage();
    ability_detail::query_ability_detail(query: AbilityDetailQuery);
    race_detail::query_race_detail(race: String);
    progress::query_progress();
    progress::set_progress_goals(goals: BTreeMap<String, f64>);
}
