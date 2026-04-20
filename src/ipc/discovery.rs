use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::debug;

use super::{get_instance_address, get_instance_port, Message};

/// 实例信息
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    /// 实例ID
    pub id: u32,
    /// 端口号
    pub port: u16,
    /// 绑定地址
    pub address: String,
    /// 是否活跃（可连接）
    pub active: bool,
}

/// 发现运行中的实例
/// 扫描端口 57427-57436（最多10个实例，ID 0-9）
pub async fn discover_instances() -> Vec<InstanceInfo> {
    let mut instances = Vec::new();

    for id in 0..10 {
        let address = get_instance_address(id);
        let port = get_instance_port(id);

        // 尝试连接，超时100ms
        let active = match timeout(
            Duration::from_millis(100),
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
            port,
            address,
            active,
        });
    }

    instances
}

/// 获取第一个活跃的实例
pub async fn find_first_active_instance() -> Option<InstanceInfo> {
    let instances = discover_instances().await;
    instances.into_iter().find(|i| i.active)
}

/// 检查指定实例是否活跃
pub async fn is_instance_active(instance_id: u32) -> bool {
    let address = get_instance_address(instance_id);
    matches!(
        timeout(Duration::from_millis(100), TcpStream::connect(&address)).await,
        Ok(Ok(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_instances() {
        let instances = discover_instances().await;
        assert_eq!(instances.len(), 10);

        // 验证端口计算
        for (i, info) in instances.iter().enumerate() {
            assert_eq!(info.id, i as u32);
            assert_eq!(info.port, 57427 + i as u16);
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
