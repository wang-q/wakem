use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::debug;

use crate::constants::IPC_DISCOVERY_TIMEOUT_MS;

use super::get_instance_address;

/// Default maximum instance ID to scan during discovery
/// Most users run a single instance (id=0), so the default scan range is 0-9.
/// The full range (0-255) can still be accessed by passing max_instance_id explicitly.
const DEFAULT_MAX_DISCOVERY_INSTANCE_ID: u32 = 9;
/// Absolute maximum instance ID (matches Config::validate() which allows 0-255)
const MAX_DISCOVERY_INSTANCE_ID: u32 = 255;

/// Maximum number of concurrent connection attempts during discovery
/// Limits resource usage when scanning many instance ports
const DISCOVERY_MAX_CONCURRENCY: usize = 32;

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

/// Discover active wakem instances by scanning ports.
///
/// Scans instance IDs from 0 to `max_instance_id` (inclusive), checking each
/// port for an active listener. Uses bounded concurrency to avoid exhausting
/// file descriptors.
///
/// # Arguments
/// * `max_instance_id` - Maximum instance ID to scan. Defaults to
///   `DEFAULT_MAX_DISCOVERY_INSTANCE_ID` (9) when `None` is passed. Pass a
///   smaller value (e.g., `Some(0)`) for a quick single-instance check.
pub async fn discover_instances(max_instance_id: Option<u32>) -> Vec<InstanceInfo> {
    let max_id = max_instance_id
        .unwrap_or(DEFAULT_MAX_DISCOVERY_INSTANCE_ID)
        .min(MAX_DISCOVERY_INSTANCE_ID);
    let semaphore = Arc::new(Semaphore::new(DISCOVERY_MAX_CONCURRENCY));
    let mut set = JoinSet::new();

    for id in 0..=max_id {
        let permit = semaphore.clone();
        set.spawn(async move {
            let _permit = permit.acquire().await.unwrap();
            let address = get_instance_address(id);

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

    let mut instances = Vec::with_capacity((max_id + 1) as usize);
    while let Some(result) = set.join_next().await {
        if let Ok(info) = result {
            instances.push(info);
        }
    }

    instances.sort_by_key(|info| info.id);
    instances
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_instances() {
        let instances = discover_instances(None).await;
        assert_eq!(
            instances.len(),
            (DEFAULT_MAX_DISCOVERY_INSTANCE_ID + 1) as usize
        );
        for (i, info) in instances.iter().enumerate() {
            assert_eq!(info.id, i as u32);
            assert!(info.address.starts_with("127.0.0.1:"));
        }
    }

    #[tokio::test]
    async fn test_discover_instances_full_range() {
        let instances = discover_instances(Some(255)).await;
        assert_eq!(instances.len(), 256);
    }
}
