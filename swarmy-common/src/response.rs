use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::SwarmyError;

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
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiResponseBuilder {
    status: bool,
    init_time: Instant,
    message: Option<String>,
}

impl ApiResponseBuilder {
    pub fn new() -> Self {
        Self {
            status: false,
            init_time: Instant::now(),
            message: None,
        }
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
