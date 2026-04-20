use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub mod auth;
pub mod client;
pub mod discovery;
pub mod rate_limiter;
pub mod security;
pub mod server;

pub use client::IpcClient;
pub use rate_limiter::ConnectionLimiter;
pub use server::IpcServer;

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
    RegisterMessageWindow { hwnd: usize },

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

pub type Result<T> = std::result::Result<T, IpcError>;

/// Base port
pub const BASE_PORT: u16 = 57427;

/// Get instance port
pub fn get_instance_port(instance_id: u32) -> u16 {
    BASE_PORT + instance_id as u16
}

/// Get instance bind address
pub fn get_instance_address(instance_id: u32) -> String {
    format!("127.0.0.1:{}", get_instance_port(instance_id))
}

/// Read message from TCP stream (common implementation to eliminate duplication)
pub async fn read_message(stream: &mut TcpStream) -> Result<Message> {
    // Read length (4 bytes)
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    // Limit maximum message size
    if len > 1024 * 1024 {
        return Err(IpcError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Message too large",
        )));
    }

    // Read data
    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer).await?;

    // Deserialize
    let message = serde_json::from_slice(&buffer)?;
    Ok(message)
}

/// Send message to TCP stream (common implementation to eliminate duplication)
pub async fn send_message(stream: &mut TcpStream, message: &Message) -> Result<()> {
    let data = serde_json::to_vec(message)?;
    let len = data.len() as u32;

    // Send length
    stream.write_all(&len.to_be_bytes()).await?;
    // Send data
    stream.write_all(&data).await?;

    Ok(())
}
