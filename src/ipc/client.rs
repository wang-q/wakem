//! IPC client implementation.

use crate::constants::IPC_CONNECTION_TIMEOUT_SECS;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::debug;

use super::auth::{compute_response, AUTH_RESULT_SUCCESS, CHALLENGE_SIZE};
use super::io::{read_message, send_message};
use super::messages::{Message, Result};
use super::IpcError;

/// IPC client (based on TCP)
pub struct IpcClient {
    stream: Option<TcpStream>,
    address: String,
    auth_key: Option<String>,
}

impl IpcClient {
    /// Create new client
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            stream: None,
            address: address.into(),
            auth_key: None,
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
        read_message(stream).await
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
