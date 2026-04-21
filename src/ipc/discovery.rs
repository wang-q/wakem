use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::debug;

use crate::constants::IPC_DISCOVERY_TIMEOUT_MS;

use super::{get_instance_address, get_instance_port};

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
    let mut instances = Vec::new();

    for id in 0..10 {
        let address = get_instance_address(id);
        let _port = get_instance_port(id);

        // Try to connect, timeout
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

        instances.push(InstanceInfo {
            id,
            address,
            active,
        });
    }

    instances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_instances() {
        let instances = discover_instances().await;
        assert_eq!(instances.len(), 10);

        // Verify instance ID and address format
        for (i, info) in instances.iter().enumerate() {
            assert_eq!(info.id, i as u32);
            // Address format should be 127.0.0.1:PORT
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
