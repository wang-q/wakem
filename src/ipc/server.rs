use super::{IpcError, Message, Result, DEFAULT_PIPE_NAME};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::sync::mpsc;

/// IPC 服务端
pub struct IpcServer {
    pipe_name: String,
    server: Option<NamedPipeServer>,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
}

impl IpcServer {
    /// 创建新的 IPC 服务端
    pub fn new(message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>) -> Self {
        Self {
            pipe_name: DEFAULT_PIPE_NAME.to_string(),
            server: None,
            message_tx,
        }
    }

    /// 使用自定义管道名称
    pub fn with_pipe_name(mut self, name: impl Into<String>) -> Self {
        self.pipe_name = name.into();
        self
    }

    /// 启动服务端
    pub async fn start(&mut self) -> Result<()> {
        let server = ServerOptions::new().create(&self.pipe_name)?;
        self.server = Some(server);
        Ok(())
    }

    /// 接受新的客户端连接
    pub async fn accept(&mut self) -> Result<ClientConnection> {
        let server = self.server.as_mut().ok_or(IpcError::ConnectionClosed)?;

        // 等待连接
        server.connect().await?;

        // 创建新的服务端实例以接受更多连接
        let new_server = ServerOptions::new().create(&self.pipe_name)?;
        let current_server = std::mem::replace(server, new_server);

        let (response_tx, response_rx) = mpsc::channel(100);

        Ok(ClientConnection {
            stream: current_server,
            message_tx: self.message_tx.clone(),
            response_rx,
            response_tx,
        })
    }

    /// 停止服务端
    pub fn stop(&mut self) {
        self.server = None;
    }
}

/// 客户端连接
pub struct ClientConnection {
    stream: NamedPipeServer,
    message_tx: mpsc::Sender<(Message, mpsc::Sender<Message>)>,
    response_rx: mpsc::Receiver<Message>,
    response_tx: mpsc::Sender<Message>,
}

impl ClientConnection {
    /// 处理客户端消息
    pub async fn handle(&mut self) -> Result<()> {
        loop {
            // 读取消息
            let message = match self.read_message().await {
                Ok(msg) => msg,
                Err(IpcError::Io(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    // 客户端断开连接
                    break;
                }
                Err(e) => return Err(e),
            };

            // 发送给主处理循环
            let _ = self
                .message_tx
                .send((message, self.response_tx.clone()))
                .await;

            // 等待响应
            if let Some(response) = self.response_rx.recv().await {
                self.send_message(&response).await?;
            }
        }

        Ok(())
    }

    /// 读取消息
    async fn read_message(&mut self) -> Result<Message> {
        // 读取长度（4字节）
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes).await?;
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
        self.stream.read_exact(&mut buffer).await?;

        // 反序列化
        let message = serde_json::from_slice(&buffer)?;
        Ok(message)
    }

    /// 发送消息
    async fn send_message(&mut self, message: &Message) -> Result<()> {
        let data = serde_json::to_vec(message)?;
        let len = data.len() as u32;

        // 发送长度
        self.stream.write_all(&len.to_be_bytes()).await?;
        // 发送数据
        self.stream.write_all(&data).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    #[ignore]
    async fn test_server_start() {
        let (tx, _rx) = mpsc::channel(100);
        let mut server = IpcServer::new(tx);
        server.start().await.unwrap();
    }
}
