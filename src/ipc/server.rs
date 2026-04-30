//! IPC server implementation.

use crate::constants::{
    IPC_CHANNEL_CAPACITY, IPC_IDLE_TIMEOUT_LONG_SECS, IPC_IDLE_TIMEOUT_SHORT_SECS,
    IPC_PROTOCOL_VERSION,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use zeroize::Zeroizing;

use super::auth::server_perform_authentication;
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
    auth_key: Option<Arc<RwLock<Zeroizing<String>>>>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
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

    /// Create new server (static key, backward compatible)
    pub fn new(
        bind_address: impl Into<String>,
        auth_key: Option<String>,
        message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    ) -> Self {
        Self {
            listener: None,
            bind_address: bind_address.into(),
            auth_key: auth_key.map(|k| Arc::new(RwLock::new(Zeroizing::new(k)))),
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

    /// Run server main loop with shutdown support
    pub async fn run(
        &mut self,
        mut shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<()> {
        let listener = self.listener.as_ref().ok_or(IpcError::ConnectionClosed)?;

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
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
                _ = shutdown.changed() => {
                    info!("IPC server received shutdown signal");
                    break;
                }
            }
        }
        Ok(())
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
        let key_guard = key_arc.read().await;
        if !key_guard.is_empty() {
            let auth_result =
                server_perform_authentication(&mut stream, key_guard.as_str()).await;
            if !auth_result? {
                warn!("Authentication failed for {}", addr);
                return Err(IpcError::ConnectionRefused);
            }
            debug!("Authentication successful for {}", addr);

            let version_bytes = IPC_PROTOCOL_VERSION.to_be_bytes();
            stream.write_all(&version_bytes).await?;
            debug!("Sent protocol version {} to {}", IPC_PROTOCOL_VERSION, addr);
        }
        drop(key_guard);
    }

    let (response_tx, mut response_rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
    let mut idle_timeout = IPC_IDLE_TIMEOUT_SHORT_SECS;

    loop {
        tokio::select! {
            result = read_message(&mut stream) => {
                match result {
                    Ok(message) => {
                        if matches!(message, Message::RegisterMessageWindow { .. }) {
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
                debug!("Connection idle timeout ({}s) for {}", idle_timeout, addr);
                break;
            }
        }
    }

    debug!("Connection closed: {}", addr);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    #[ignore = "binds to real port, may conflict with running instances"]
    async fn test_server_start() {
        let (tx, _rx) = mpsc::channel(IPC_CHANNEL_CAPACITY);
        let auth_key = Arc::new(RwLock::new(Zeroizing::new("test-key".to_string())));
        let mut server =
            IpcServer::new_with_dynamic_key("127.0.0.1:57428", auth_key, tx);
        server.start().await.unwrap();
    }

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
