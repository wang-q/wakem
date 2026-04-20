use serde::{Deserialize, Serialize};

pub mod auth;
pub mod client;
pub mod discovery;
pub mod security;
pub mod server;

pub use client::IpcClient;
pub use discovery::{discover_instances, InstanceInfo};
pub use server::IpcServer;

/// IPC 消息协议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // 客户端 -> 服务端
    /// 发送配置到服务端
    SetConfig { config: crate::config::Config },
    /// 重新加载配置
    ReloadConfig,
    /// 获取当前状态
    GetStatus,
    /// 启用/禁用映射
    SetActive { active: bool },
    /// 获取下一个按键信息（用于调试）
    GetNextKeyInfo,

    // 服务端 -> 客户端
    /// 状态响应
    StatusResponse { active: bool, config_loaded: bool },
    /// 配置已加载
    ConfigLoaded,
    /// 配置加载错误
    ConfigError { error: String },
    /// 下一个按键信息（调试）
    NextKeyInfo { info: String },
    /// 错误响应
    Error { message: String },

    // 双向
    /// 心跳
    Ping,
    /// 心跳响应
    Pong,
}

/// IPC 错误类型
#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("Connection refused")]
    ConnectionRefused,
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Timeout")]
    Timeout,
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<tokio::time::error::Elapsed> for IpcError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        IpcError::Timeout
    }
}

pub type Result<T> = std::result::Result<T, IpcError>;

/// 基础端口
pub const BASE_PORT: u16 = 57427;

/// 获取实例端口
pub fn get_instance_port(instance_id: u32) -> u16 {
    BASE_PORT + instance_id as u16
}

/// 获取实例绑定地址
pub fn get_instance_address(instance_id: u32) -> String {
    format!("127.0.0.1:{}", get_instance_port(instance_id))
}
