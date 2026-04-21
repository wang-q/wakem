use super::{
    auth, read_message, security, send_message, ConnectionLimiter, IpcError, Message,
    Result,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// IPC server (based on TCP)
///
/// Security features:
/// - IP whitelist (only allow local connections)
/// - Challenge-response authentication mechanism
/// - Connection rate limiting (prevent brute force attacks)
pub struct IpcServer {
    listener: Option<TcpListener>,
    bind_address: String,
    /// Authentication key (using Arc<RwLock> for dynamic updates)
    auth_key: Option<Arc<RwLock<String>>>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    /// Connection rate limiter (prevent brute force attacks)
    rate_limiter: Arc<RwLock<ConnectionLimiter>>,
}

impl IpcServer {
    /// Create new server (with dynamic key)
    pub fn new_with_dynamic_key(
        bind_address: impl Into<String>,
        auth_key: Arc<RwLock<String>>,
        message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    ) -> Self {
        Self {
            listener: None,
            bind_address: bind_address.into(),
            auth_key: Some(auth_key),
            message_tx,
            rate_limiter: Arc::new(RwLock::new(ConnectionLimiter::with_defaults())),
        }
    }

    /// Create new server (static key, backward compatible)
    pub fn new(
        bind_address: impl Into<String>,
        auth_key: Option<String>,
        message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    ) -> Self {
        Self {
            listener: None,
            bind_address: bind_address.into(),
            auth_key: auth_key.map(|k| Arc::new(RwLock::new(k))),
            message_tx,
            rate_limiter: Arc::new(RwLock::new(ConnectionLimiter::with_defaults())),
        }
    }

    /// Start server
    pub async fn start(&mut self) -> Result<()> {
        let listener = TcpListener::bind(&self.bind_address).await?;
        info!("Server listening on {}", self.bind_address);
        self.listener = Some(listener);
        Ok(())
    }

    /// Run server main loop
    pub async fn run(&mut self) -> Result<()> {
        let listener = self.listener.as_ref().ok_or(IpcError::ConnectionClosed)?;

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("New connection from {}", addr);

                    // Check if IP is allowed (Security layer 1: IP whitelist)
                    if !security::is_allowed_ip(addr.ip()) {
                        warn!("Rejected connection from external IP: {}", addr);
                        continue;
                    }

                    // 检查速率限制（安全层2：防暴力破解）
                    {
                        let mut limiter = self.rate_limiter.write().await;
                        if !limiter.check_rate_limit(addr.ip()) {
                            warn!("Rate limit exceeded for IP: {}", addr);
                            // 记录安全告警
                            error!(
                                "Security alert: Possible brute force attack from {}",
                                addr
                            );
                            continue;
                        }
                    }

                    // 克隆必要的数据（Arc<RwLock> 的 clone 很便宜）
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
    auth_key: Option<Arc<RwLock<String>>>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
) -> Result<()> {
    // 如果配置了认证密钥，执行挑战-响应认证（动态读取最新密钥）
    if let Some(key_arc) = auth_key {
        let key = key_arc.read().await;
        if !key.is_empty() {
            if !perform_authentication_with_timeout(&mut stream, &key).await? {
                warn!("Authentication failed for {}", addr);
                return Err(IpcError::ConnectionRefused);
            }
            debug!("Authentication successful for {}", addr);
        }
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

            // 超时检查（2分钟空闲超时，平衡资源使用和用户体验）
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(120)) => {
                debug!("Connection timeout for {}", addr);
                break;
            }
        }
    }

    debug!("Connection closed: {}", addr);
    Ok(())
}

/// 执行挑战-响应认证（带超时控制）
async fn perform_authentication_with_timeout(
    stream: &mut TcpStream,
    auth_key: &str,
) -> Result<bool> {
    use tokio::time::{timeout, Duration};

    // 生成挑战
    let challenge = auth::generate_challenge();

    // 发送挑战给客户端（带5秒超时）
    timeout(Duration::from_secs(5), stream.write_all(&challenge))
        .await
        .map_err(|_| IpcError::Timeout)??;

    // 读取响应（带5秒超时）
    let mut response = [0u8; auth::RESPONSE_SIZE];
    timeout(Duration::from_secs(5), stream.read_exact(&mut response))
        .await
        .map_err(|_| IpcError::Timeout)??;

    // 验证响应
    Ok(auth::verify_response(auth_key, &challenge, &response))
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

    #[tokio::test]
    #[ignore]
    async fn test_server_start_with_dynamic_key() {
        let (tx, _rx) = mpsc::channel(100);
        let auth_key = Arc::new(RwLock::new("test-key".to_string()));
        let mut server =
            IpcServer::new_with_dynamic_key("127.0.0.1:57429", auth_key, tx);
        server.start().await.unwrap();
    }
}
