//! IPC server implementation.

use crate::constants::{
    AUTH_OPERATION_TIMEOUT_SECS, IPC_CHANNEL_CAPACITY, IPC_IDLE_TIMEOUT_SECS,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::{debug, error, info, warn};

use super::auth::{
    generate_challenge, verify_response, zero_string, AUTH_RESULT_FAILURE,
    AUTH_RESULT_SUCCESS, RESPONSE_SIZE,
};
use super::io::{read_message, send_message};
use super::messages::{Message, Result};
use super::rate_limiter::ConnectionLimiter;
use super::security::is_allowed_ip;
use super::IpcError;

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
    auth_key: Option<Arc<RwLock<String>>>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
) -> Result<()> {
    if let Some(key_arc) = auth_key {
        let mut key = {
            let key_guard = key_arc.read().await;
            key_guard.clone()
        };
        if !key.is_empty() {
            let auth_result = server_perform_authentication(&mut stream, &key).await;
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

    let (response_tx, mut response_rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);

    loop {
        tokio::select! {
            result = read_message(&mut stream) => {
                match result {
                    Ok(message) => {
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

            _ = tokio::time::sleep(Duration::from_secs(IPC_IDLE_TIMEOUT_SECS)) => {
                debug!("Connection timeout for {}", addr);
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
