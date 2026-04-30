//! Message I/O utilities for IPC.

use crate::constants::{IPC_BASE_PORT, IPC_MAX_MESSAGE_SIZE};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::messages::{Message, Result};
use super::IpcError;

/// Base port (re-export from constants)
pub const BASE_PORT: u16 = IPC_BASE_PORT;

/// Get instance port
///
/// # Panics
/// Panics if `instance_id` would cause port overflow (instance_id > 8108).
/// In practice, `Config::validate()` limits instance_id to 0-255.
pub fn get_instance_port(instance_id: u32) -> u16 {
    BASE_PORT
        .checked_add(instance_id as u16)
        .expect("instance_id overflow: port would exceed u16 range")
}

/// Get instance bind address
pub fn get_instance_address(instance_id: u32) -> String {
    format!("127.0.0.1:{}", get_instance_port(instance_id))
}

/// Read message from TCP stream
pub async fn read_message(stream: &mut TcpStream) -> Result<Message> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    if len > IPC_MAX_MESSAGE_SIZE {
        return Err(IpcError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Message too large",
        )));
    }

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer).await?;

    let message = serde_json::from_slice(&buffer)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_instance_port() {
        assert_eq!(get_instance_port(0), IPC_BASE_PORT);
        assert_eq!(get_instance_port(1), IPC_BASE_PORT + 1);
        assert_eq!(get_instance_port(255), IPC_BASE_PORT + 255);
    }

    #[test]
    fn test_get_instance_address() {
        assert_eq!(get_instance_address(0), format!("127.0.0.1:{}", IPC_BASE_PORT));
        assert_eq!(
            get_instance_address(1),
            format!("127.0.0.1:{}", IPC_BASE_PORT + 1)
        );
    }
}
