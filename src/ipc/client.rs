use super::{auth, read_message, send_message, IpcError, Message, Result};
use crate::constants::IPC_CONNECTION_TIMEOUT_SECS;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::debug;

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

        // Connection timeout
        let mut stream = timeout(
            Duration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
            TcpStream::connect(&self.address),
        )
        .await
        .map_err(|_| IpcError::Timeout)??;

        debug!("Connection established");

        // If authentication key is configured, perform authentication
        if let Some(ref key) = self.auth_key {
            if !perform_authentication(&mut stream, key).await? {
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
            Duration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
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

/// Perform challenge-response authentication
async fn perform_authentication(stream: &mut TcpStream, auth_key: &str) -> Result<bool> {
    // Read challenge
    let mut challenge = [0u8; auth::CHALLENGE_SIZE];

    timeout(
        Duration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
        stream.read_exact(&mut challenge),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    // Compute response
    let response = auth::compute_response(auth_key, &challenge);

    // Send response
    timeout(
        Duration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
        stream.write_all(&response),
    )
    .await
    .map_err(|_| IpcError::Timeout)??;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_client_connect() {
        let mut client = IpcClient::new("127.0.0.1:57427");
        let _ = client.connect().await;
    }
}
