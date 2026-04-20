use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub mod auth;
pub mod client;
pub mod discovery;
pub mod rate_limiter;
pub mod security;
pub mod server;

pub use client::IpcClient;
pub use rate_limiter::ConnectionLimiter;
pub use server::IpcServer;

/// IPC 消息协议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    // 客户端 -> 服务端
    /// 发送配置到服务端
    SetConfig { config: crate::config::Config },
    /// 重新加载配置
    ReloadConfig,
    /// 保存当前配置到文件
    SaveConfig,
    /// 获取当前状态
    GetStatus,
    /// 启用/禁用映射
    SetActive { active: bool },
    /// 获取下一个按键信息（用于调试）
    GetNextKeyInfo,
    /// 开始录制宏
    StartMacroRecording { name: String },
    /// 停止录制宏
    StopMacroRecording,
    /// 播放宏
    PlayMacro { name: String },
    /// 获取宏列表
    GetMacros,
    /// 删除宏
    DeleteMacro { name: String },
    /// 绑定宏到触发键
    BindMacro { macro_name: String, trigger: String },
    /// 注册消息窗口句柄（用于发送通知）
    RegisterMessageWindow { hwnd: usize },

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
    /// 宏录制结果
    MacroRecordingResult { name: String, action_count: usize },
    /// 宏列表响应
    MacrosList { macros: Vec<String> },
    /// 成功响应
    Success,

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

/// 从 TCP 流读取消息（公共实现，消除重复）
pub async fn read_message(stream: &mut TcpStream) -> Result<Message> {
    // 读取长度（4字节）
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    // 限制最大消息大小
    if len > 1024 * 1024 {
        return Err(IpcError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Message too large",
        )));
    }

    // 读取数据
    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer).await?;

    // 反序列化
    let message = serde_json::from_slice(&buffer)?;
    Ok(message)
}

/// 发送消息到 TCP 流（公共实现，消除重复）
pub async fn send_message(stream: &mut TcpStream, message: &Message) -> Result<()> {
    let data = serde_json::to_vec(message)?;
    let len = data.len() as u32;

    // 发送长度
    stream.write_all(&len.to_be_bytes()).await?;
    // 发送数据
    stream.write_all(&data).await?;

    Ok(())
}
