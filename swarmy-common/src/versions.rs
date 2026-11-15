use serde::{Deserialize, Serialize};

/// The versions of the application and the libraries it is built with.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AppVersions {
    pub swarmy: String,
    pub s2protocol: String,
    pub rerun: String,
    pub polars: String,
    pub tauri: String,
    /// The commit the backend was built from (`SWARMY_GIT_SHA`, src-tauri/build.rs), for the
    /// links to the query files; empty from a server older than it.
    #[serde(default)]
    pub git_sha: String,
    /// Whether the tree had uncommitted changes then: the links go to the development branch.
    #[serde(default)]
    pub git_dirty: bool,
}
