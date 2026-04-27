use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::{debug, warn};

use crate::constants::{IPC_CONNECTION_TIMEOUT_SECS, IPC_REQUEST_TIMEOUT_SECS};

use super::auth::client_perform_authentication;
use super::{read_message, send_message, IpcError, Message, Result, IPC_PROTOCOL_VERSION};

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

            let mut version_bytes = [0u8; 4];
            timeout(
                TokioDuration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
                stream.read_exact(&mut version_bytes),
            )
            .await
            .map_err(|_| IpcError::Timeout)??;
            let server_version = u32::from_be_bytes(version_bytes);
            if server_version != IPC_PROTOCOL_VERSION {
                let error_msg = format!(
                    "Protocol version mismatch: server={}, client={}",
                    server_version, IPC_PROTOCOL_VERSION
                );
                warn!("{}", error_msg);
                let _ = send_message(
                    &mut stream,
                    &Message::Error {
                        message: error_msg.clone(),
                    },
                )
                .await;
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
            TokioDuration::from_secs(IPC_REQUEST_TIMEOUT_SECS),
            self.receive(),
        )
        .await?
    }
}


