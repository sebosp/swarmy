use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::SwarmyError;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub success: bool,
    pub duration_ms: u64,
    pub is_complete: bool,
    /// The queries that computed the answer, for the "Show Polars Debug Information" links of
    /// the views; empty for the commands that are not queries of the snapshot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<QuerySource>,
}

/// Where a query of the snapshot lives in the repository, from the `META` of its file
/// (`query_meta!()` in the backend): the module path, the file and the line of `META`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuerySource {
    /// The module path without the crate, i.e. `map_details::recent_games`.
    pub id: String,
    /// The file in the repository, i.e. `src-tauri/src/map_details/recent_games.rs`.
    pub file: String,
    pub line: u32,
}

impl ResponseMeta {
    // Creates a ResponseMeta of incomplete status.
    // Useful for the backend to display indicators.
    pub fn incomplete() -> Self {
        Self {
            success: false,
            duration_ms: 0,
            is_complete: false,
            sources: vec![],
        }
    }

    /// TODO: move duration_ms to Interval to avoid carrying around counters.
    pub fn complete_with_success(&mut self) {
        self.success = true;
        self.is_complete = true;
    }

    pub fn complete_with_failure(&mut self) {
        self.success = false;
        self.is_complete = true;
    }
}

#[derive(Debug, Clone)]
pub struct ResponseMetaBuilder {
    pub success: bool,
    pub duration_ms: Option<u64>,
    init_time: Instant,
}

impl Default for ResponseMetaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseMetaBuilder {
    pub fn new() -> Self {
        Self {
            success: false,
            duration_ms: None,
            init_time: Instant::now(),
        }
    }

    pub fn with_success(mut self) -> Self {
        self.success = true;
        self
    }

    pub fn with_failure(mut self) -> Self {
        self.success = false;
        self
    }

    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn build(self) -> ResponseMeta {
        ResponseMeta {
            success: self.success,
            duration_ms: self.init_time.elapsed().as_millis() as u64,
            is_complete: true,
            sources: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiResponseBuilder {
    status: bool,
    init_time: Instant,
    message: Option<String>,
    sources: Vec<QuerySource>,
}

impl Default for ApiResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiResponseBuilder {
    pub fn new() -> Self {
        Self {
            status: false,
            init_time: Instant::now(),
            message: None,
            sources: vec![],
        }
    }

    /// The queries that compute the answer, see [`QuerySource`].
    pub fn with_sources(mut self, sources: impl IntoIterator<Item = QuerySource>) -> Self {
        self.sources = sources.into_iter().collect();
        self
    }

    pub fn with_message(mut self, msg: String) -> Self {
        self.message = Some(msg);
        self
    }

    pub fn with_success(mut self) -> Self {
        self.status = true;
        self
    }

    pub fn with_failure(mut self) -> Self {
        self.status = false;
        self
    }

    pub fn with_status(mut self, status: bool) -> Self {
        self.status = status;
        self
    }

    pub fn process_result(mut self, input: Result<impl Serialize, SwarmyError>) -> ApiResponse {
        self.status = input.is_ok();
        self.message = match input {
            Ok(v) => match serde_json::to_string(&v) {
                Ok(val) => Some(val),
                Err(err) => {
                    tracing::error!("Error serializing {:?}", err);
                    None
                }
            },
            Err(err) => {
                tracing::error!("{:?}", err);
                Some(err.to_string())
            }
        };
        self.build()
    }

    pub fn build(self) -> ApiResponse {
        ApiResponse {
            meta: ResponseMeta {
                success: self.status,
                duration_ms: self.init_time.elapsed().as_millis() as u64,
                is_complete: true,
                sources: self.sources,
            },
            message: self.message.unwrap_or(String::from("")),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub meta: ResponseMeta,
    pub message: String,
}

impl ApiResponse {
    pub fn new(meta: ResponseMeta, message: String) -> Self {
        Self { meta, message }
    }

    /// Creates an ApiResponse from client side to indicate an incomplete/pending state.
    pub fn new_incomplete() -> Self {
        Self {
            meta: ResponseMeta::incomplete(),
            message: String::new(),
        }
    }
}
