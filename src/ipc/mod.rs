use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::constants::{IPC_BASE_PORT, IPC_MAX_MESSAGE_SIZE};

pub mod auth;
pub mod client;
pub mod discovery;
pub mod rate_limiter;
pub mod security;
pub mod server;

// Re-export commonly used types for convenience
pub use client::IpcClient;

// These re-exports are part of the public API for external crate usage.
// They may not be used internally but are kept for backward compatibility.
#[allow(unused_imports)]
pub use discovery::InstanceInfo;
pub use discovery::discover_instances;

#[allow(unused_imports)]
pub use rate_limiter::ConnectionLimiter;

#[allow(unused_imports)]
pub use security::{is_allowed_ip, is_private_ip};
pub use server::IpcServer;

// ==================== Message Protocol ====================

/// IPC message protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // Client -> Server
    /// Send configuration to server
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
    /// Initialize platform-specific services (e.g., notification service)
    InitializePlatform { native_handle: Option<usize> },
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

// ==================== Error Types ====================

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

// ==================== Protocol Version ====================

/// IPC protocol version for compatibility checking between client and server.
///
/// Increment this when the message format changes in a breaking way.
/// The server sends this version during the handshake; the client can
/// check it to detect version mismatches.
pub const IPC_PROTOCOL_VERSION: u32 = 1;

// ==================== Instance Address Helpers ====================

/// Base port (re-export from constants)
pub const BASE_PORT: u16 = IPC_BASE_PORT;

/// Get instance port number
///
/// Uses const-compatible checked arithmetic. Panics at compile time
/// if instance_id would cause port overflow (which shouldn't happen
/// since instance_id is validated to be 0-255 in Config::validate).
pub const fn get_instance_port(instance_id: u32) -> u16 {
    assert!(
        instance_id <= 255,
        "instance_id overflow: port would exceed u16 range"
    );
    BASE_PORT + instance_id as u16
}

/// Get instance bind address
pub fn get_instance_address(instance_id: u32) -> String {
    format!("127.0.0.1:{}", get_instance_port(instance_id))
}

// ==================== Message I/O ====================

/// Read message from TCP stream with reusable buffer
///
/// The buffer is cleared and reused across calls to avoid repeated heap
/// allocations. This is especially beneficial for high-frequency IPC
/// communication where many messages are exchanged on a single connection.
pub async fn read_message(
    stream: &mut TcpStream,
    buffer: &mut Vec<u8>,
) -> Result<Message> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    if len > IPC_MAX_MESSAGE_SIZE {
        return Err(IpcError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Message too large",
        )));
    }

    buffer.clear();
    buffer.resize(len, 0);
    stream.read_exact(buffer).await?;

    let message = serde_json::from_slice(buffer)?;
    Ok(message)
}

/// Send message to TCP stream
pub async fn send_message(stream: &mut TcpStream, message: &Message) -> Result<()> {
    let data = serde_json::to_vec(message)?;
    let len = data.len() as u32;

    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&data).await?;

    Ok(())
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_instance_port() {
        assert_eq!(get_instance_port(0), 57427);
        assert_eq!(get_instance_port(1), 57428);
        assert_eq!(get_instance_port(9), 57436);
    }

    #[tokio::test]
    async fn test_get_instance_address() {
        assert_eq!(get_instance_address(0), "127.0.0.1:57427");
        assert_eq!(get_instance_address(1), "127.0.0.1:57428");
    }

    /// Ignored: This test requires a running server on port 57427.
    /// Run manually with: `cargo test test_client_connect -- --ignored`
    #[tokio::test]
    #[ignore = "requires running server on port 57427"]
    async fn test_client_connect() {
        let mut client = IpcClient::new("127.0.0.1:57427");
        let _ = client.connect().await;
    }
}
