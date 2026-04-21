use super::{
    auth, read_message, security, send_message, ConnectionLimiter, IpcError, Message,
    Result,
};
use crate::constants::{
    AUTH_OPERATION_TIMEOUT_SECS, IPC_CHANNEL_CAPACITY, IPC_IDLE_TIMEOUT_SECS,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

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
                    if !security::is_allowed_ip(addr.ip()) {
                        warn!("Rejected connection from external IP: {}", addr);
                        continue;
                    }

                    // Check rate limit (Security layer 2: prevent brute force)
                    {
                        let mut limiter = self.rate_limiter.write().await;
                        if !limiter.check_rate_limit(addr.ip()) {
                            warn!("Rate limit exceeded for IP: {}", addr);
                            // Log security alert
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
        let key = {
            let key_guard = key_arc.read().await;
            key_guard.clone()
        };
        if !key.is_empty() {
            if !perform_authentication_with_timeout(&mut stream, &key).await? {
                warn!("Authentication failed for {}", addr);
                return Err(IpcError::ConnectionRefused);
            }
            debug!("Authentication successful for {}", addr);
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
    let challenge = auth::generate_challenge();

    // Send challenge to client (with timeout)
    timeout(
        Duration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.write_all(&challenge),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    // Read response (with timeout)
    let mut response = [0u8; auth::RESPONSE_SIZE];
    timeout(
        Duration::from_secs(AUTH_OPERATION_TIMEOUT_SECS),
        stream.read_exact(&mut response),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    // Verify response
    Ok(auth::verify_response(auth_key, &challenge, &response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

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
