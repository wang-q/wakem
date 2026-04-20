use super::{auth, security, IpcError, Message, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// IPC 服务端（基于 TCP）
pub struct IpcServer {
    listener: Option<TcpListener>,
    bind_address: String,
    auth_key: Option<String>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
}

impl IpcServer {
    /// 创建新的服务端
    pub fn new(
        bind_address: impl Into<String>,
        auth_key: Option<String>,
        message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    ) -> Self {
        Self {
            listener: None,
            bind_address: bind_address.into(),
            auth_key,
            message_tx,
        }
    }

    /// 启动服务端
    pub async fn start(&mut self) -> Result<()> {
        let listener = TcpListener::bind(&self.bind_address).await?;
        info!("Server listening on {}", self.bind_address);
        self.listener = Some(listener);
        Ok(())
    }

    /// 运行服务端主循环
    pub async fn run(&mut self) -> Result<()> {
        let listener = self.listener.as_ref().ok_or(IpcError::ConnectionClosed)?;

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("New connection from {}", addr);

                    // 检查 IP 是否允许
                    if !security::is_allowed_ip(addr.ip()) {
                        warn!("Rejected connection from external IP: {}", addr);
                        continue;
                    }

                    // 克隆必要的数据
                    let auth_key = self.auth_key.clone();
                    let message_tx = self.message_tx.clone();

                    // 处理连接
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(stream, addr, auth_key, message_tx).await
                        {
                            debug!("Connection handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

/// 处理单个连接
async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
    auth_key: Option<String>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
) -> Result<()> {
    // 如果配置了认证密钥，执行挑战-响应认证
    if let Some(key) = auth_key {
        if !perform_authentication(&mut stream, &key).await? {
            warn!("Authentication failed for {}", addr);
            return Err(IpcError::ConnectionRefused);
        }
        debug!("Authentication successful for {}", addr);
    }

    // 创建响应通道
    let (response_tx, mut response_rx) = mpsc::channel(100);

    // 消息处理循环
    loop {
        tokio::select! {
            // 读取客户端消息
            result = read_message(&mut stream) => {
                match result {
                    Ok(message) => {
                        // 发送给主处理循环
                        if message_tx.send((message, response_tx.clone())).await.is_err() {
                            break;
                        }
                    }
                    Err(IpcError::Io(e))
                        if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        // 客户端断开连接
                        break;
                    }
                    Err(e) => return Err(e),
                }
            }

            // 发送响应给客户端
            Some(response) = response_rx.recv() => {
                if let Err(e) = send_message(&mut stream, &response).await {
                    debug!("Failed to send response: {}", e);
                    break;
                }
            }

            // 超时检查
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(300)) => {
                debug!("Connection timeout for {}", addr);
                break;
            }
        }
    }

    debug!("Connection closed: {}", addr);
    Ok(())
}

/// 执行挑战-响应认证
async fn perform_authentication(stream: &mut TcpStream, auth_key: &str) -> Result<bool> {
    // 生成挑战
    let challenge = auth::generate_challenge();

    // 发送挑战给客户端
    stream.write_all(&challenge).await?;

    // 读取响应
    let mut response = [0u8; auth::RESPONSE_SIZE];
    stream.read_exact(&mut response).await?;

    // 验证响应
    Ok(auth::verify_response(auth_key, &challenge, &response))
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
    use tokio::sync::mpsc;

    #[tokio::test]
    #[ignore]
    async fn test_server_start() {
        let (tx, _rx) = mpsc::channel(100);
        let mut server = IpcServer::new("127.0.0.1:57428", None, tx);
        server.start().await.unwrap();
    }
}
