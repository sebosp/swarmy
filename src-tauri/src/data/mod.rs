use swarmy_common::*;

/// Sanitcizes the replay path, this should be stored locally in the AppConfig.
pub fn sanitize_replay_path(replay_path: &str) -> Result<String, SwarmyTauriError> {
    if replay_path.is_empty() {
        return Err(SwarmyTauriError::Other(
            "Replay path is not set, please set it in Scan tab.".to_string(),
        ));
    }
    Ok(replay_path.trim_end_matches('/').to_string())
}

/// Builds the IPC path for the given replay path, the path must exist, done via the Optimize
/// functionality in the Scan tab.
pub fn build_ipc_path(replay_path: &str) -> Result<String, SwarmyTauriError> {
    let replay_path = format!("{}/{}", replay_path, IPC_DIR);
    let ipc_path = std::path::Path::new(&replay_path);
    if !ipc_path.exists() {
        return Err(SwarmyTauriError::Other(format!(
            "Directory {} not Optimized yet, go to Scan first.",
            replay_path
        )));
    }
    Ok(replay_path)
}
