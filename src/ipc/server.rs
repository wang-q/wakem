use super::{
    generate_challenge, read_message, send_message, verify_response, AUTH_RESULT_FAILURE,
    AUTH_RESULT_SUCCESS, IpcError, Message, RESPONSE_SIZE, Result,
};
use crate::constants::{
    AUTH_OPERATION_TIMEOUT_SECS, IPC_CHANNEL_CAPACITY, IPC_IDLE_TIMEOUT_SECS,
    RATE_LIMIT_MAX_ATTEMPTS, RATE_LIMIT_WINDOW_SECS,
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

// ==================== IP Security ====================

/// Check if IP address is private (RFC 1918) or loopback
/// Only IPv4 addresses are supported; IPv6 addresses are rejected
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            // 10.0.0.0/8
            o[0] == 10
                // 172.16.0.0/12
                || (o[0] == 172 && o[1] >= 16 && o[1] <= 31)
                // 192.168.0.0/16
                || (o[0] == 192 && o[1] == 168)
                // 127.0.0.0/8 (localhost)
                || o[0] == 127
                // 169.254.0.0/16 (link-local)
                || (o[0] == 169 && o[1] == 254)
        }
        IpAddr::V6(_) => {
            // IPv6 is not supported, reject all IPv6 addresses
            false
        }
    }
}

/// Check if IP address is allowed to connect
/// Only private IPv4 addresses are allowed
pub fn is_allowed_ip(ip: IpAddr) -> bool {
    is_private_ip(ip)
}

// ==================== Rate Limiter ====================

/// Maximum number of IPs to track (prevents memory exhaustion)
const MAX_TRACKED_IPS: usize = 1000;
/// Cleanup threshold - when exceeded, remove oldest entries
const CLEANUP_THRESHOLD: usize = 900;

/// Connection rate limiter
///
/// Used to prevent brute force and denial of service attacks
/// Features:
/// - IP-based rate limiting
/// - Configurable max attempts and time window
/// - Automatic cleanup of expired records
/// - Memory limit protection (max 1000 tracked IPs)
pub struct ConnectionLimiter {
    /// Attempt records for each IP
    attempts: HashMap<IpAddr, Vec<Instant>>,
    /// Maximum allowed attempts
    pub max_attempts: u32,
    /// Time window (seconds)
    pub window_seconds: u64,
}

impl ConnectionLimiter {
    /// Create a new rate limiter
    ///
    /// # Parameters
    /// * `max_attempts` - Maximum allowed attempts within the time window
    /// * `window_seconds` - Time window size (seconds)
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
    ///
    /// # Returns
    /// * `true` - Connection allowed
    /// * `false` - Rate limit exceeded, connection denied
    pub fn check_rate_limit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_seconds);

        // Memory protection: cleanup if too many IPs tracked
        if self.attempts.len() >= MAX_TRACKED_IPS {
            self.cleanup_oldest_entries(MAX_TRACKED_IPS - CLEANUP_THRESHOLD);
        }

        // Get or create record for this IP
        let attempt_times = self.attempts.entry(ip).or_default();

        // Cleanup expired records for this IP
        attempt_times.retain(|&time| now.duration_since(time) < window);

        // Record this attempt (always, even if rate limited)
        attempt_times.push(now);

        // Check if limit exceeded
        attempt_times.len() <= self.max_attempts as usize
    }

    /// Cleanup oldest entries when memory limit is reached
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
    #[allow(dead_code)]
    pub fn reset(&mut self, ip: &IpAddr) {
        self.attempts.remove(ip);
    }

    /// Clear all records
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.attempts.clear();
    }

    /// Get current number of tracked IPs
    #[allow(dead_code)]
    pub fn tracked_count(&self) -> usize {
        self.attempts.len()
    }
}

impl Default for ConnectionLimiter {
    fn default() -> Self {
        Self::with_defaults()
    }
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
    /// Authentication key (using Arc<RwLock> for dynamic updates)
    auth_key: Option<Arc<RwLock<String>>>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    /// Connection rate limiter (prevent brute force attacks)
    rate_limiter: Arc<RwLock<ConnectionLimiter>>,
}

impl IpcServer {
    /// Create new server (with dynamic key)
    pub fn new_with_dynamic_key(
        bind_address: impl Into<String>,
        auth_key: Arc<RwLock<String>>,
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

    /// Create new server (static key, backward compatible)
    #[allow(dead_code)]
    pub fn new(
        bind_address: impl Into<String>,
        auth_key: Option<String>,
        message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    ) -> Self {
        Self {
            listener: None,
            bind_address: bind_address.into(),
            auth_key: auth_key.map(|k| Arc::new(RwLock::new(k))),
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

                    // Check if IP is allowed (Security layer 1: IP whitelist)
                    if !is_allowed_ip(addr.ip()) {
                        warn!("Rejected connection from external IP: {}", addr);
                        continue;
                    }

                    // Check rate limit (Security layer 2: prevent brute force)
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

                    // Clone necessary data (Arc<RwLock> clone is cheap)
                    let auth_key = self.auth_key.clone();
                    let message_tx = self.message_tx.clone();

                    // Handle connection
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
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

/// Handle a single connection
async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    auth_key: Option<Arc<RwLock<String>>>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
) -> Result<()> {
    // If auth key is configured, perform challenge-response authentication (dynamically read latest key)
    if let Some(key_arc) = auth_key {
        // Clone the key immediately to release the lock before authentication
        let mut key = {
            let key_guard = key_arc.read().await;
            key_guard.clone()
        };
        if !key.is_empty() {
            let auth_result = perform_authentication_with_timeout(&mut stream, &key).await;
            // Zero sensitive key data after use
            zero_string(&mut key);
            if !auth_result? {
                warn!("Authentication failed for {}", addr);
                return Err(IpcError::ConnectionRefused);
            }
            debug!("Authentication successful for {}", addr);
        } else {
            zero_string(&mut key);
        }
    }

    // Create response channel
    let (response_tx, mut response_rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);

    // Message processing loop
    loop {
        tokio::select! {
            // Read client message
            result = read_message(&mut stream) => {
                match result {
                    Ok(message) => {
                        // Send to main processing loop
                        if message_tx.send((message, response_tx.clone())).await.is_err() {
                            break;
                        }
                    }
                    Err(IpcError::Io(e))
                        if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        // Client disconnected
                        break;
                    }
                    Err(e) => return Err(e),
                }
            }

            // Send response to client
            Some(response) = response_rx.recv() => {
                if let Err(e) = send_message(&mut stream, &response).await {
                    debug!("Failed to send response: {}", e);
                    break;
                }
            }

            // Timeout check (idle timeout, balancing resource usage and user experience)
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(IPC_IDLE_TIMEOUT_SECS)) => {
                debug!("Connection timeout for {}", addr);
                break;
            }
        }
    }

    debug!("Connection closed: {}", addr);
    Ok(())
}

/// Perform challenge-response authentication (with timeout)
async fn perform_authentication_with_timeout(
    stream: &mut TcpStream,
    auth_key: &str,
) -> Result<bool> {
    use tokio::time::{timeout, Duration};

    // Generate challenge
    let challenge = generate_challenge();

    // Send challenge to client (with timeout)
    timeout(
        Duration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.write_all(&challenge),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    // Read response (with timeout)
    let mut response = [0u8; RESPONSE_SIZE];
    timeout(
        Duration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.read_exact(&mut response),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    // Verify response
    let auth_ok = verify_response(auth_key, &challenge, &response);

    // Send authentication result to client
    let result_byte = if auth_ok {
        AUTH_RESULT_SUCCESS
    } else {
        AUTH_RESULT_FAILURE
    };
    timeout(
        Duration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.write_all(&[result_byte]),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    Ok(auth_ok)
}

/// Zero sensitive string data in memory
fn zero_string(s: &mut String) {
    unsafe {
        let bytes = s.as_bytes_mut();
        bytes.iter_mut().for_each(|b| *b = 0);
    }
    s.clear();
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use tokio::sync::mpsc;

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
    #[ignore]
    async fn test_server_start() {
        let (tx, _rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
        let mut server = IpcServer::new("127.0.0.1:57428", None, tx);
        server.start().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_server_start_with_dynamic_key() {
        let (tx, _rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
        let auth_key = Arc::new(RwLock::new("test-key".to_string()));
        let mut server =
            IpcServer::new_with_dynamic_key("127.0.0.1:57429", auth_key, tx);
        server.start().await.unwrap();
    }
}
