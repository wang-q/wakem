//! IPC message protocol and error types.

use serde::{Deserialize, Serialize};

/// IPC message protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Client -> Server
    /// Send configuration to server
    #[allow(clippy::large_enum_variant)]
    SetConfig { config: Box<crate::config::Config> },
    /// Reload configuration
    ReloadConfig,
    /// Save current configuration to file
    SaveConfig,
    /// Get current status
    GetStatus,
    /// Enable/disable mapping
    SetActive { active: bool },
    /// Get next key info (for debugging)
    GetNextKeyInfo,
    /// Start macro recording
    StartMacroRecording { name: String },
    /// Stop macro recording
    StopMacroRecording,
    /// Play macro
    PlayMacro { name: String },
    /// Get macro list
    GetMacros,
    /// Delete macro
    DeleteMacro { name: String },
    /// Bind macro to trigger key
    BindMacro { macro_name: String, trigger: String },
    /// Register message window handle (for sending notifications)
    RegisterNativeHandle { handle: usize },
    /// Shutdown the daemon
    Shutdown,

    // Server -> Client
    /// Status response
    StatusResponse { active: bool, config_loaded: bool },
    /// Configuration loaded
    ConfigLoaded,
    /// Configuration load error
    ConfigError { error: String },
    /// Next key info (debug)
    NextKeyInfo { info: String },
    /// Error response
    Error { message: String },
    /// Macro recording result
    MacroRecordingResult { name: String, action_count: usize },
    /// Macro list response
    MacrosList { macros: Vec<String> },
    /// Success response
    Success,

    // Bidirectional
    /// Heartbeat
    Ping,
    /// Heartbeat response
    Pong,
}

/// IPC error type
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("Connection refused")]
    ConnectionRefused,
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Timeout")]
    Timeout,
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<tokio::time::error::Elapsed> for IpcError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        IpcError::Timeout
    }
}

/// IPC result type
pub type Result<T> = std::result::Result<T, IpcError>;
