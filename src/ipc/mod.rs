use crate::constants::{IPC_BASE_PORT, IPC_DISCOVERY_TIMEOUT_MS, IPC_MAX_MESSAGE_SIZE};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};
use tracing::debug;

pub mod client;
pub mod server;

pub use client::IpcClient;
pub use server::IpcServer;

// ==================== Authentication ====================

/// Challenge length (32 bytes)
pub const CHALLENGE_SIZE: usize = 32;

/// Response length (32 bytes, HMAC-SHA256 output)
pub const RESPONSE_SIZE: usize = 32;

/// Authentication result byte: success
pub const AUTH_RESULT_SUCCESS: u8 = 0x01;
/// Authentication result byte: failure
pub const AUTH_RESULT_FAILURE: u8 = 0x00;

/// Generate random challenge
pub fn generate_challenge() -> [u8; CHALLENGE_SIZE] {
    let mut challenge = [0u8; CHALLENGE_SIZE];
    rand::thread_rng().fill_bytes(&mut challenge);
    challenge
}

/// Compute response (HMAC-SHA256)
pub fn compute_response(auth_key: &str, challenge: &[u8]) -> [u8; RESPONSE_SIZE] {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(auth_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(challenge);

    let result = mac.finalize();
    let bytes = result.into_bytes();

    let mut response = [0u8; RESPONSE_SIZE];
    response.copy_from_slice(&bytes[..RESPONSE_SIZE]);
    response
}

/// Verify response using constant-time comparison via hmac crate
pub fn verify_response(auth_key: &str, challenge: &[u8], response: &[u8]) -> bool {
    if response.len() != RESPONSE_SIZE {
        return false;
    }

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = match HmacSha256::new_from_slice(auth_key.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(challenge);
    mac.verify_slice(response).is_ok()
}

// ==================== Instance Discovery ====================

/// Instance information
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    /// Instance ID
    pub id: u32,
    /// Bind address
    pub address: String,
    /// Whether active (connectable)
    pub active: bool,
}

/// Discover running instances
/// Scan ports 57427-57436 (max 10 instances, ID 0-9)
pub async fn discover_instances() -> Vec<InstanceInfo> {
    let mut set = JoinSet::new();

    for id in 0..10u32 {
        set.spawn(async move {
            let address = get_instance_address(id);

            let active = match timeout(
                Duration::from_millis(IPC_DISCOVERY_TIMEOUT_MS),
                TcpStream::connect(&address),
            )
            .await
            {
                Ok(Ok(_)) => {
                    debug!("Found active instance {} at {}", id, address);
                    true
                }
                _ => false,
            };

            InstanceInfo {
                id,
                address,
                active,
            }
        });
    }

    let mut instances = Vec::with_capacity(10);
    while let Some(result) = set.join_next().await {
        if let Ok(info) = result {
            instances.push(info);
        }
    }

    instances.sort_by_key(|info| info.id);
    instances
}

// ==================== Message Protocol ====================

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

// ==================== Instance Address Helpers ====================

/// Base port (re-export from constants)
pub const BASE_PORT: u16 = IPC_BASE_PORT;

/// Get instance port
pub fn get_instance_port(instance_id: u32) -> u16 {
    BASE_PORT + instance_id as u16
}

/// Get instance bind address
pub fn get_instance_address(instance_id: u32) -> String {
    format!("127.0.0.1:{}", get_instance_port(instance_id))
}

// ==================== Message I/O ====================

/// Read message from TCP stream (common implementation to eliminate duplication)
pub async fn read_message(stream: &mut TcpStream) -> Result<Message> {
    // Read length (4 bytes)
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    // Limit maximum message size
    if len > IPC_MAX_MESSAGE_SIZE {
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

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_generation() {
        let challenge1 = generate_challenge();
        let challenge2 = generate_challenge();
        assert_ne!(challenge1, challenge2);
    }

    #[test]
    fn test_response_computation() {
        let auth_key = "my-secret-key";
        let challenge = generate_challenge();

        let response1 = compute_response(auth_key, &challenge);
        let response2 = compute_response(auth_key, &challenge);
        assert_eq!(response1, response2);
    }

    #[test]
    fn test_response_different_keys() {
        let challenge = generate_challenge();
        let response1 = compute_response("key1", &challenge);
        let response2 = compute_response("key2", &challenge);
        assert_ne!(response1, response2);
    }

    #[test]
    fn test_verify_response_success() {
        let auth_key = "test-key";
        let challenge = generate_challenge();
        let response = compute_response(auth_key, &challenge);
        assert!(verify_response(auth_key, &challenge, &response));
    }

    #[test]
    fn test_verify_response_wrong_key() {
        let challenge = generate_challenge();
        let response = compute_response("correct-key", &challenge);
        assert!(!verify_response("wrong-key", &challenge, &response));
    }

    #[test]
    fn test_verify_response_wrong_challenge() {
        let auth_key = "test-key";
        let challenge1 = generate_challenge();
        let challenge2 = generate_challenge();
        let response = compute_response(auth_key, &challenge1);
        assert!(!verify_response(auth_key, &challenge2, &response));
    }

    #[tokio::test]
    async fn test_discover_instances() {
        let instances = discover_instances().await;
        assert_eq!(instances.len(), 10);
        for (i, info) in instances.iter().enumerate() {
            assert_eq!(info.id, i as u32);
            assert!(info.address.starts_with("127.0.0.1:"));
        }
    }

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
}
