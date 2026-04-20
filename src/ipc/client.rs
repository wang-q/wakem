use super::{IpcError, Message, Result, DEFAULT_PIPE_NAME};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::time::{timeout, Duration};

/// IPC 客户端
pub struct IpcClient {
    stream: Option<tokio::net::windows::named_pipe::NamedPipeClient>,
    pipe_name: String,
}

impl IpcClient {
    /// 创建新的 IPC 客户端
    pub fn new() -> Self {
        Self {
            stream: None,
            pipe_name: DEFAULT_PIPE_NAME.to_string(),
        }
    }

    /// 使用自定义管道名称
    pub fn with_pipe_name(mut self, name: impl Into<String>) -> Self {
        self.pipe_name = name.into();
        self
    }

    /// 连接到服务端
    pub async fn connect(&mut self) -> Result<()> {
        let stream = ClientOptions::new().open(&self.pipe_name)?;
        self.stream = Some(stream);
        Ok(())
    }

    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// 发送消息
    pub async fn send(&mut self, message: &Message) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(IpcError::ConnectionClosed)?;

        // 序列化消息
        let data = serde_json::to_vec(message)?;
        let len = data.len() as u32;

        // 发送长度（4字节，大端序）
        stream.write_all(&len.to_be_bytes()).await?;
        // 发送数据
        stream.write_all(&data).await?;

        Ok(())
    }

    /// 接收消息
    pub async fn receive(&mut self) -> Result<Message> {
        let stream = self.stream.as_mut().ok_or(IpcError::ConnectionClosed)?;

        // 读取长度（4字节）
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        // 限制最大消息大小
        if len > 1024 * 1024 {
            // 1MB
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

    /// 发送消息并等待响应
    pub async fn send_receive(&mut self, message: &Message) -> Result<Message> {
        self.send(message).await?;
        timeout(Duration::from_secs(5), self.receive()).await?
    }

    /// 关闭连接
    pub async fn close(mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            stream.shutdown().await?;
        }
        Ok(())
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：这些测试需要服务端运行
    #[tokio::test]
    #[ignore]
    async fn test_client_connect() {
        let mut client = IpcClient::new();
        // 如果没有服务端，这里会失败
        let _ = client.connect().await;
    }
}
