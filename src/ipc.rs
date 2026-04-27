use crate::constants::{
    AUTH_OPERATION_TIMEOUT_SECS, IPC_BASE_PORT, IPC_CHANNEL_CAPACITY,
    IPC_CONNECTION_TIMEOUT_SECS, IPC_DISCOVERY_TIMEOUT_MS, IPC_IDLE_TIMEOUT_LONG_SECS,
    IPC_IDLE_TIMEOUT_SHORT_SECS,
    IPC_MAX_MESSAGE_SIZE, RATE_LIMIT_MAX_ATTEMPTS, RATE_LIMIT_WINDOW_SECS,
};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::{debug, error, info, warn};
use zeroize::Zeroizing;

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

// ==================== IP Security ====================

/// Check if IP address is private (RFC 1918) or loopback
///
/// IPv6 addresses are intentionally rejected by design:
/// - The IPC server binds to 127.0.0.1 (IPv4 loopback only)
/// - IPv6 is not used in this project to keep the networking layer simple
/// - Any IPv6 connection attempt would not reach the server anyway
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 10
                || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                || (o[0] == 192 && o[1] == 168)
                || o[0] == 127
                || (o[0] == 169 && o[1] == 254)
        }
        IpAddr::V6(_) => false,
    }
}

/// Check if IP address is allowed to connect
///
/// Only private IPv4 addresses are allowed (RFC 1918, loopback, link-local).
/// IPv6 is intentionally not supported - see `is_private_ip` for rationale.
pub fn is_allowed_ip(ip: IpAddr) -> bool {
    is_private_ip(ip)
}

// ==================== Rate Limiter ====================

/// Maximum number of IPs to track (prevents memory exhaustion)
const MAX_TRACKED_IPS: usize = 1000;
/// Cleanup threshold - when exceeded, remove oldest entries
const CLEANUP_THRESHOLD: usize = 900;
/// Maximum instance ID to scan during discovery
/// Matches Config::validate() which allows instance_id 0-255
const MAX_DISCOVERY_INSTANCE_ID: u32 = 255;

/// Connection rate limiter
///
/// Used to prevent brute force and denial of service attacks
/// Features:
/// - IP-based rate limiting
/// - Configurable max attempts and time window
/// - Automatic cleanup of expired records
/// - Memory limit protection (max 1000 tracked IPs)
pub struct ConnectionLimiter {
    attempts: HashMap<IpAddr, Vec<Instant>>,
    /// Maximum allowed attempts
    pub max_attempts: u32,
    /// Time window (seconds)
    pub window_seconds: u64,
}

impl ConnectionLimiter {
    /// Create a new rate limiter
    pub fn new(max_attempts: u32, window_seconds: u64) -> Self {
        Self {
            attempts: HashMap::new(),
            max_attempts,
            window_seconds,
        }
    }

    /// Create rate limiter with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RATE_LIMIT_MAX_ATTEMPTS, RATE_LIMIT_WINDOW_SECS)
    }

    /// Check if connection is allowed
    pub fn check_rate_limit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_seconds);

        if self.attempts.len() >= MAX_TRACKED_IPS {
            self.cleanup_oldest_entries(MAX_TRACKED_IPS - CLEANUP_THRESHOLD);
        }

        let attempt_times = self.attempts.entry(ip).or_default();
        attempt_times.retain(|&time| now.duration_since(time) < window);

        let allowed = attempt_times.len() < self.max_attempts as usize;
        if allowed {
            attempt_times.push(now);
        }

        allowed
    }

    fn cleanup_oldest_entries(&mut self, count: usize) {
        let mut ip_ages: Vec<(IpAddr, Instant)> = self
            .attempts
            .iter()
            .filter_map(|(ip, times)| times.first().copied().map(|t| (*ip, t)))
            .collect();

        let count = count.min(ip_ages.len());
        if count > 0 {
            ip_ages.select_nth_unstable_by(count - 1, |a, b| a.1.cmp(&b.1));
            for (ip, _) in ip_ages.iter().take(count) {
                self.attempts.remove(ip);
            }
        }
    }

    /// Reset limit count for specified IP
    #[cfg(test)]
    pub fn reset(&mut self, ip: &IpAddr) {
        self.attempts.remove(ip);
    }
}

impl Default for ConnectionLimiter {
    fn default() -> Self {
        Self::with_defaults()
    }
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

/// Maximum number of concurrent connection attempts during discovery
/// Limits resource usage when scanning many instance ports
const DISCOVERY_MAX_CONCURRENCY: usize = 32;

/// Discover running instances
/// Scan ports based on MAX_DISCOVERY_INSTANCE_ID
/// Uses bounded concurrency to avoid overwhelming the system with simultaneous
/// TCP connection attempts (max 32 concurrent).
pub async fn discover_instances() -> Vec<InstanceInfo> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(DISCOVERY_MAX_CONCURRENCY));
    let mut set = JoinSet::new();

    for id in 0..=MAX_DISCOVERY_INSTANCE_ID {
        let permit = semaphore.clone();
        set.spawn(async move {
            let _permit = permit.acquire().await.unwrap();
            let address = get_instance_address(id);

            let active = match timeout(
                TokioDuration::from_millis(IPC_DISCOVERY_TIMEOUT_MS),
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

    let mut instances = Vec::with_capacity((MAX_DISCOVERY_INSTANCE_ID + 1) as usize);
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

/// Get instance port
///
/// # Panics
/// Panics if `instance_id` would cause port overflow (instance_id > 8108).
/// In practice, `Config::validate()` limits instance_id to 0-255.
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

/// Read message from TCP stream
/// Read message from TCP stream with reusable buffer
///
/// The buffer is cleared and reused across calls to avoid repeated heap
/// allocations. This is especially beneficial for high-frequency IPC
/// communication where many messages are exchanged on a single connection.
pub async fn read_message(stream: &mut TcpStream, buffer: &mut Vec<u8>) -> Result<Message> {
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

// ==================== IPC Client ====================

/// IPC client (based on TCP)
pub struct IpcClient {
    stream: Option<TcpStream>,
    address: String,
    auth_key: Option<String>,
    /// Reusable buffer for reading messages to avoid per-message allocation
    read_buffer: Vec<u8>,
}

impl IpcClient {
    /// Create new client
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            stream: None,
            address: address.into(),
            auth_key: None,
            read_buffer: Vec::new(),
        }
    }

    /// Set authentication key
    pub fn with_auth_key(mut self, auth_key: impl Into<String>) -> Self {
        self.auth_key = Some(auth_key.into());
        self
    }

    /// Connect to server
    pub async fn connect(&mut self) -> Result<()> {
        debug!("Connecting to server at {}", self.address);

        let mut stream = timeout(
            TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
            TcpStream::connect(&self.address),
        )
        .await
        .map_err(|_| IpcError::Timeout)??;

        debug!("Connection established");

        if let Some(ref key) = self.auth_key {
            if !client_perform_authentication(&mut stream, key).await? {
                return Err(IpcError::ConnectionRefused);
            }
            debug!("Authentication successful");

            // Read and verify protocol version from server
            let mut version_bytes = [0u8; 4];
            timeout(
                TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
                stream.read_exact(&mut version_bytes),
            )
            .await
            .map_err(|_| IpcError::Timeout)??;
            let server_version = u32::from_le_bytes(version_bytes);
            if server_version != IPC_PROTOCOL_VERSION {
                warn!(
                    "Protocol version mismatch: server={}, client={}",
                    server_version, IPC_PROTOCOL_VERSION
                );
                return Err(IpcError::ConnectionRefused);
            }
            debug!("Protocol version {} verified", server_version);
        }

        self.stream = Some(stream);
        Ok(())
    }

    /// Send message
    pub async fn send(&mut self, message: &Message) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(IpcError::ConnectionClosed)?;
        send_message(stream, message).await
    }

    /// Receive message
    pub async fn receive(&mut self) -> Result<Message> {
        let stream = self.stream.as_mut().ok_or(IpcError::ConnectionClosed)?;
        read_message(stream, &mut self.read_buffer).await
    }

    /// Send message and wait for response
    pub async fn send_receive(&mut self, message: &Message) -> Result<Message> {
        self.send(message).await?;
        timeout(
            TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
            self.receive(),
        )
        .await?
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new("127.0.0.1:57427")
    }
}

/// Perform challenge-response authentication (client side)
async fn client_perform_authentication(
    stream: &mut TcpStream,
    auth_key: &str,
) -> Result<bool> {
    let mut challenge = [0u8; CHALLENGE_SIZE];

    timeout(
        TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
        stream.read_exact(&mut challenge),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    let response = compute_response(auth_key, &challenge);

    timeout(
        TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
        stream.write_all(&response),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    let mut result = [0u8; 1];
    timeout(
        TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
        stream.read_exact(&mut result),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    Ok(result[0] == AUTH_RESULT_SUCCESS)
}

// ==================== IPC Server ====================

/// IPC server (based on TCP)
///
/// Security features:
/// - IP whitelist (only allow local connections)
/// - Challenge-response authentication mechanism
/// - Connection rate limiting (prevent brute force attacks)
pub struct IpcServer {
    listener: Option<TcpListener>,
    bind_address: String,
    /// Authentication key (Zeroizing ensures old key data is wiped on update)
    auth_key: Option<Arc<RwLock<Zeroizing<String>>>>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    /// Connection rate limiter (prevent brute force attacks)
    rate_limiter: Arc<RwLock<ConnectionLimiter>>,
}

impl IpcServer {
    /// Create new server (with dynamic key)
    pub fn new_with_dynamic_key(
        bind_address: impl Into<String>,
        auth_key: Arc<RwLock<Zeroizing<String>>>,
        message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    ) -> Self {
        Self {
            listener: None,
            bind_address: bind_address.into(),
            auth_key: Some(auth_key),
            message_tx,
            rate_limiter: Arc::new(RwLock::new(ConnectionLimiter::with_defaults())),
        }
    }

    /// Start server
    pub async fn start(&mut self) -> Result<()> {
        let listener = TcpListener::bind(&self.bind_address).await?;
        info!("Server listening on {}", self.bind_address);
        self.listener = Some(listener);
        Ok(())
    }

    /// Run server main loop
    pub async fn run(&mut self) -> Result<()> {
        let listener = self.listener.as_ref().ok_or(IpcError::ConnectionClosed)?;

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("New connection from {}", addr);

                    if !is_allowed_ip(addr.ip()) {
                        warn!("Rejected connection from external IP: {}", addr);
                        continue;
                    }

                    {
                        let mut limiter = self.rate_limiter.write().await;
                        if !limiter.check_rate_limit(addr.ip()) {
                            warn!("Rate limit exceeded for IP: {}", addr);
                            error!(
                                "Security alert: Possible brute force attack from {}",
                                addr
                            );
                            continue;
                        }
                    }

                    let auth_key = self.auth_key.clone();
                    let message_tx = self.message_tx.clone();

                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, addr, auth_key, message_tx).await
                        {
                            debug!("Connection handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
}

/// Handle a single connection
async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    auth_key: Option<Arc<RwLock<Zeroizing<String>>>>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
) -> Result<()> {
    if let Some(key_arc) = auth_key {
        let key = {
            let key_guard = key_arc.read().await;
            key_guard.clone()
        };
        if !key.is_empty() {
            let auth_result = server_perform_authentication(&mut stream, &key).await;
            if !auth_result? {
                warn!("Authentication failed for {}", addr);
                return Err(IpcError::ConnectionRefused);
            }
            debug!("Authentication successful for {}", addr);

            // Send protocol version after successful authentication
            let version_bytes = IPC_PROTOCOL_VERSION.to_le_bytes();
            stream.write_all(&version_bytes).await?;
            debug!(
                "Sent protocol version {} to {}",
                IPC_PROTOCOL_VERSION, addr
            );
        } else {
            // Zeroizing<String> automatically zeroes on drop
        }
    }

    let (response_tx, mut response_rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
    let mut read_buffer = Vec::new();
    let mut idle_timeout = IPC_IDLE_TIMEOUT_SHORT_SECS;

    loop {
        tokio::select! {
            result = read_message(&mut stream, &mut read_buffer) => {
                match result {
                    Ok(message) => {
                        // Upgrade to long timeout for persistent connections.
                        // InitializePlatform is used by tray clients that maintain
                        // a long-lived connection for status updates.
                        if matches!(message, Message::InitializePlatform { .. }) {
                            idle_timeout = IPC_IDLE_TIMEOUT_LONG_SECS;
                        }
                        if message_tx.send((message, response_tx.clone())).await.is_err() {
                            break;
                        }
                    }
                    Err(IpcError::Io(e))
                        if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        break;
                    }
                    Err(e) => return Err(e),
                }
            }

            Some(response) = response_rx.recv() => {
                if let Err(e) = send_message(&mut stream, &response).await {
                    debug!("Failed to send response: {}", e);
                    break;
                }
            }

            _ = tokio::time::sleep(Duration::from_secs(idle_timeout)) => {
                debug!("Connection idle timeout ({})s for {}", idle_timeout, addr);
                break;
            }
        }
    }

    debug!("Connection closed: {}", addr);
    Ok(())
}

/// Perform challenge-response authentication (server side, with timeout)
async fn server_perform_authentication(
    stream: &mut TcpStream,
    auth_key: &str,
) -> Result<bool> {
    let challenge = generate_challenge();

    timeout(
        TokioDuration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.write_all(&challenge),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    let mut response = [0u8; RESPONSE_SIZE];
    timeout(
        TokioDuration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.read_exact(&mut response),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    let auth_ok = verify_response(auth_key, &challenge, &response);

    let result_byte = if auth_ok {
        AUTH_RESULT_SUCCESS
    } else {
        AUTH_RESULT_FAILURE
    };
    timeout(
        TokioDuration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.write_all(&[result_byte]),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    Ok(auth_ok)
}

/// Zero out a String's memory contents using zeroize crate
///
/// This is used to clear sensitive data (e.g., authentication keys) from memory
/// after use, preventing key material from lingering in heap memory where it
/// could potentially be exposed through memory dumps or core dumps.
///
/// Uses the zeroize crate which provides secure memory clearing that is not
/// optimized away by the compiler.
// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tokio::sync::mpsc;

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

    #[test]
    fn test_private_ip_10() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
    }

    #[test]
    fn test_private_ip_172() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 15, 0, 1))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));
    }

    #[test]
    fn test_private_ip_192() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 167, 0, 1))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 169, 0, 1))));
    }

    #[test]
    fn test_localhost() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))));
    }

    #[test]
    fn test_public_ip() {
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1))));
    }

    #[test]
    fn test_basic_rate_limiting() {
        let mut limiter = ConnectionLimiter::new(3, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(!limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_different_ips() {
        let mut limiter = ConnectionLimiter::new(2, 60);
        let ip1 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

        assert!(limiter.check_rate_limit(ip1));
        assert!(limiter.check_rate_limit(ip1));
        assert!(!limiter.check_rate_limit(ip1));

        assert!(limiter.check_rate_limit(ip2));
        assert!(limiter.check_rate_limit(ip2));
    }

    #[test]
    fn test_reset() {
        let mut limiter = ConnectionLimiter::new(2, 60);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
        assert!(!limiter.check_rate_limit(ip));

        limiter.reset(&ip);
        assert!(limiter.check_rate_limit(ip));
        assert!(limiter.check_rate_limit(ip));
    }

    #[test]
    fn test_default_config() {
        let limiter = ConnectionLimiter::default();
        assert_eq!(limiter.max_attempts, RATE_LIMIT_MAX_ATTEMPTS);
        assert_eq!(limiter.window_seconds, RATE_LIMIT_WINDOW_SECS);
    }

    #[tokio::test]
    async fn test_discover_instances() {
        let instances = discover_instances().await;
        assert_eq!(instances.len(), 256);
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

    /// Ignored: This test requires a running server on port 57427.
    /// Run manually with: `cargo test test_client_connect -- --ignored`
    #[tokio::test]
    #[ignore = "requires running server on port 57427"]
    async fn test_client_connect() {
        let mut client = IpcClient::new("127.0.0.1:57427");
        let _ = client.connect().await;
    }

    /// Ignored: This test binds to a real port and may conflict with running instances.
    /// Run manually with: `cargo test test_server_start -- --ignored`
    #[tokio::test]
    #[ignore = "binds to real port, may conflict with running instances"]
    async fn test_server_start() {
        let (tx, _rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
        let auth_key = Arc::new(RwLock::new(Zeroizing::new("test-key".to_string())));
        let mut server =
            IpcServer::new_with_dynamic_key("127.0.0.1:57428", auth_key, tx);
        server.start().await.unwrap();
    }

    /// Ignored: This test binds to a real port and may conflict with running instances.
    /// Run manually with: `cargo test test_server_start_with_dynamic_key -- --ignored`
    #[tokio::test]
    #[ignore = "binds to real port, may conflict with running instances"]
    async fn test_server_start_with_dynamic_key() {
        let (tx, _rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
        let auth_key = Arc::new(RwLock::new(Zeroizing::new("test-key".to_string())));
        let mut server =
            IpcServer::new_with_dynamic_key("127.0.0.1:57429", auth_key, tx);
        server.start().await.unwrap();
    }
}
