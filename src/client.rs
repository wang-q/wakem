use anyhow::Result;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

use crate::ipc::{IpcClient, Message, TcpIpcClient};

/// 连接类型
enum ConnectionType {
    /// 本地 Named Pipe
    Pipe(IpcClient),
    /// TCP 网络连接
    Tcp(TcpIpcClient),
}

/// 守护进程客户端
pub struct DaemonClient {
    connection: Option<ConnectionType>,
}

impl DaemonClient {
    /// 创建新的客户端（不连接）
    pub fn new() -> Self {
        Self { connection: None }
    }

    /// 连接到本地服务端（Named Pipe）
    pub async fn connect(&mut self) -> Result<()> {
        debug!("Connecting to daemon via Named Pipe...");

        let mut ipc = IpcClient::new();
        match timeout(Duration::from_secs(5), ipc.connect()).await {
            Ok(result) => {
                result?;
                self.connection = Some(ConnectionType::Pipe(ipc));
                info!("Connected to daemon via Named Pipe");
                Ok(())
            }
            Err(_) => Err(anyhow::anyhow!("Connection timeout")),
        }
    }

    /// 连接到 TCP 服务端
    pub async fn connect_tcp(
        &mut self,
        address: impl Into<String>,
        auth_key: Option<String>,
    ) -> Result<()> {
        let address = address.into();
        debug!("Connecting to daemon via TCP at {}...", address);

        let mut tcp_client = TcpIpcClient::new(address);
        if let Some(key) = auth_key {
            tcp_client = tcp_client.with_auth_key(key);
        }

        match timeout(Duration::from_secs(5), tcp_client.connect()).await {
            Ok(result) => {
                result?;
                self.connection = Some(ConnectionType::Tcp(tcp_client));
                info!("Connected to daemon via TCP");
                Ok(())
            }
            Err(_) => Err(anyhow::anyhow!("Connection timeout")),
        }
    }

    /// 自动连接（先尝试 Pipe，再尝试 TCP）
    pub async fn connect_auto(
        &mut self,
        tcp_address: Option<String>,
        auth_key: Option<String>,
    ) -> Result<()> {
        // 1. 先尝试本地 Named Pipe
        if let Ok(()) = self.connect().await {
            return Ok(());
        }

        // 2. 如果提供了 TCP 地址，尝试 TCP
        if let Some(addr) = tcp_address {
            return self.connect_tcp(addr, auth_key).await;
        }

        Err(anyhow::anyhow!(
            "No connection available. Is wakemd running?"
        ))
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    /// 获取服务端状态
    pub async fn get_status(&mut self) -> Result<(bool, bool)> {
        let response = self.send_receive(&Message::GetStatus).await?;

        match response {
            Message::StatusResponse {
                active,
                config_loaded,
            } => Ok((active, config_loaded)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// 设置启用状态
    pub async fn set_active(&mut self, active: bool) -> Result<()> {
        let response = self.send_receive(&Message::SetActive { active }).await?;

        match response {
            Message::StatusResponse { .. } => Ok(()),
            Message::Error { message } => {
                Err(anyhow::anyhow!("Server error: {}", message))
            }
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// 重新加载配置
    pub async fn reload_config(&mut self) -> Result<()> {
        let response = self.send_receive(&Message::ReloadConfig).await?;

        match response {
            Message::ConfigLoaded => {
                info!("Configuration reloaded");
                Ok(())
            }
            Message::ConfigError { error } => {
                Err(anyhow::anyhow!("Config error: {}", error))
            }
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// 发送消息并等待响应
    async fn send_receive(&mut self, message: &Message) -> Result<Message> {
        if !self.is_connected() {
            return Err(anyhow::anyhow!("Not connected to daemon"));
        }

        match self.connection.as_mut().unwrap() {
            ConnectionType::Pipe(ipc) => {
                match timeout(Duration::from_secs(5), ipc.send_receive(message)).await {
                    Ok(result) => {
                        result.map_err(|e| anyhow::anyhow!("IPC error: {}", e))
                    }
                    Err(_) => Err(anyhow::anyhow!("Request timeout")),
                }
            }
            ConnectionType::Tcp(tcp) => {
                match timeout(Duration::from_secs(5), tcp.send_receive(message)).await {
                    Ok(result) => {
                        result.map_err(|e| anyhow::anyhow!("TCP error: {}", e))
                    }
                    Err(_) => Err(anyhow::anyhow!("Request timeout")),
                }
            }
        }
    }

    /// 关闭连接
    pub async fn close(self) -> Result<()> {
        match self.connection {
            Some(ConnectionType::Pipe(ipc)) => {
                ipc.close().await?;
            }
            Some(ConnectionType::Tcp(tcp)) => {
                tcp.close().await?;
            }
            None => {}
        }
        Ok(())
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}
