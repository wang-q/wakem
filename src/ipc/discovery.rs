//! Instance discovery for IPC.

use crate::constants::IPC_DISCOVERY_TIMEOUT_MS;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::debug;

/// Maximum instance ID to scan during discovery
/// Matches Config::validate() which allows instance_id 0-255
const MAX_DISCOVERY_INSTANCE_ID: u32 = 255;

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
/// Scan ports based on MAX_DISCOVERY_INSTANCE_ID
pub async fn discover_instances() -> Vec<InstanceInfo> {
    let mut set = JoinSet::new();

    for id in 0..=MAX_DISCOVERY_INSTANCE_ID {
        set.spawn(async move {
            let address = super::get_instance_address(id);

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
