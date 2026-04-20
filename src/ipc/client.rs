use super::{auth, IpcError, Message, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::debug;

/// IPC 客户端（基于 TCP）
pub struct IpcClient {
    stream: Option<TcpStream>,
    address: String,
    auth_key: Option<String>,
}

impl IpcClient {
    /// 创建新的客户端
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            stream: None,
            address: address.into(),
            auth_key: None,
        }
    }

    /// 设置认证密钥
    pub fn with_auth_key(mut self, auth_key: impl Into<String>) -> Self {
        self.auth_key = Some(auth_key.into());
        self
    }

    /// 连接到服务端
    pub async fn connect(&mut self) -> Result<()> {
        debug!("Connecting to server at {}", self.address);

        // 连接超时 5 秒
        let mut stream =
            timeout(Duration::from_secs(5), TcpStream::connect(&self.address))
                .await
                .map_err(|_| IpcError::Timeout)??;

        debug!("Connection established");

        // 如果配置了认证密钥，执行认证
        if let Some(ref key) = self.auth_key {
            if !perform_authentication(&mut stream, key).await? {
                return Err(IpcError::ConnectionRefused);
            }
            debug!("Authentication successful");
        }

        self.stream = Some(stream);
        Ok(())
    }

    /// 发送消息
    pub async fn send(&mut self, message: &Message) -> Result<()> {
        let stream = self.stream.as_mut().ok_or(IpcError::ConnectionClosed)?;
        send_message(stream, message).await
    }

    /// 接收消息
    pub async fn receive(&mut self) -> Result<Message> {
        let stream = self.stream.as_mut().ok_or(IpcError::ConnectionClosed)?;
        read_message(stream).await
    }

    /// 发送消息并等待响应
    pub async fn send_receive(&mut self, message: &Message) -> Result<Message> {
        self.send(message).await?;
        timeout(Duration::from_secs(5), self.receive()).await?
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new("127.0.0.1:57427")
    }
}

/// 执行挑战-响应认证
async fn perform_authentication(stream: &mut TcpStream, auth_key: &str) -> Result<bool> {
    // 读取挑战
    let mut challenge = [0u8; auth::CHALLENGE_SIZE];

    timeout(Duration::from_secs(5), stream.read_exact(&mut challenge))
        .await
        .map_err(|_| IpcError::Timeout)??;

    // 计算响应
    let response = auth::compute_response(auth_key, &challenge);

    // 发送响应
    timeout(Duration::from_secs(5), stream.write_all(&response))
        .await
        .map_err(|_| IpcError::Timeout)??;

    Ok(true)
}

/// 读取消息
async fn read_message(stream: &mut TcpStream) -> Result<Message> {
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

/// 发送消息
async fn send_message(stream: &mut TcpStream, message: &Message) -> Result<()> {
    let data = serde_json::to_vec(message)?;
    let len = data.len() as u32;

    // 发送长度
    stream.write_all(&len.to_be_bytes()).await?;
    // 发送数据
    stream.write_all(&data).await?;

    Ok(())
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
