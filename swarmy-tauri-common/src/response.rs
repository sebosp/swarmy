use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub success: bool,
    pub duration_ms: u64,
    pub is_complete: bool,
}

impl ResponseMeta {
    // Creates a ResponseMeta of incomplete status.
    // Useful for the backend to display indicators.
    pub fn incomplete() -> Self {
        Self {
            success: false,
            duration_ms: 0,
            is_complete: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResponseMetaBuilder {
    pub success: bool,
    pub duration_ms: Option<u64>,
}

impl ResponseMetaBuilder {
    pub fn new(success: bool) -> Self {
        Self {
            success,
            duration_ms: None,
        }
    }

    pub fn duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn build(self) -> ResponseMeta {
        ResponseMeta {
            success: self.success,
            duration_ms: self.duration_ms.unwrap_or(0),
            is_complete: true,
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
