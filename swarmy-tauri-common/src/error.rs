use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum SwarmyTauriError {
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Store Error")]
    TauriPluginStore(#[from] tauri_plugin_store::Error),
    #[error("StdError")]
    StdErr(#[from] Box<dyn std::error::Error>),
    #[error("S2proto Error")]
    S2ProtoErr(#[from] s2protocol::error::S2ProtocolError),
    #[error(transparent)]
    StdIo(#[from] std::io::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Polars Error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    #[error("UTF8 Error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Serde Error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Serde Wasm Bindgen Error: {0}")]
    SerdeWasmBindgen(#[from] serde_wasm_bindgen::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Reqwest Error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Tokio JoinError: {0}")]
    TokioJoin(#[from] JoinError),

    #[error("Other Error: {0}")]
    Other(String),
}

impl From<SwarmyTauriError> for String {
    fn from(err: SwarmyTauriError) -> Self {
        match err {
            #[cfg(not(target_arch = "wasm32"))]
            SwarmyTauriError::TauriPluginStore(e) => format!("TauriPluginStore Error: {}", e),
            SwarmyTauriError::StdErr(e) => format!("StdError: {}", e),
            SwarmyTauriError::StdIo(e) => format!("StdIoError: {}", e),
            SwarmyTauriError::S2ProtoErr(e) => format!("S2proto Error: {}", e),

            #[cfg(not(target_arch = "wasm32"))]
            SwarmyTauriError::Polars(e) => format!("Polars Error: {}", e),

            SwarmyTauriError::Utf8(e) => format!("UTF8 Error: {}", e),
            SwarmyTauriError::SerdeJson(e) => format!("Serde Error: {}", e),
            SwarmyTauriError::SerdeWasmBindgen(e) => format!("Serde Wasm Bindgen Error: {}", e),

            #[cfg(not(target_arch = "wasm32"))]
            SwarmyTauriError::Reqwest(e) => format!("Reqwest Error: {}", e),
            #[cfg(not(target_arch = "wasm32"))]
            SwarmyTauriError::TokioJoin(e) => format!("Tokio JoinError: {}", e),
            SwarmyTauriError::Other(e) => format!("Other Error: {}", e),
        }
    }
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
enum ErrorKind {
    #[cfg(not(target_arch = "wasm32"))]
    SwarmyTauriStore(String),
    StdErr(String),
    Io(String),
    Utf8(String),
    S2Proto(String),

    #[cfg(not(target_arch = "wasm32"))]
    Polars(String),

    Serde(String),
    SerdeWasmBindgen(String),

    #[cfg(not(target_arch = "wasm32"))]
    Reqwest(String),

    #[cfg(not(target_arch = "wasm32"))]
    TokioJoin(String),

    Other(String),
}

impl serde::Serialize for SwarmyTauriError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let error_message = self.to_string();
        let error_kind = match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::TauriPluginStore(_) => ErrorKind::SwarmyTauriStore(error_message),
            Self::StdErr(_) => ErrorKind::StdErr(error_message),
            Self::StdIo(_) => ErrorKind::Io(error_message),
            Self::Utf8(_) => ErrorKind::Utf8(error_message),
            Self::S2ProtoErr(_) => ErrorKind::S2Proto(error_message),

            #[cfg(not(target_arch = "wasm32"))]
            Self::Polars(_) => ErrorKind::Polars(error_message),

            Self::SerdeJson(_) => ErrorKind::Serde(error_message),
            Self::SerdeWasmBindgen(_) => ErrorKind::SerdeWasmBindgen(error_message),

            #[cfg(not(target_arch = "wasm32"))]
            Self::Reqwest(_) => ErrorKind::Reqwest(error_message),

            #[cfg(not(target_arch = "wasm32"))]
            Self::TokioJoin(_) => ErrorKind::TokioJoin(error_message),

            Self::Other(_) => ErrorKind::Other(error_message),
        };
        error_kind.serialize(serializer)
    }
}
