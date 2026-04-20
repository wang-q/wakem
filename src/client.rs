use anyhow::Result;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

use crate::ipc::{get_instance_address, IpcClient, Message};

/// 守护进程客户端
pub struct DaemonClient {
    client: Option<IpcClient>,
}

impl DaemonClient {
    /// 创建新的客户端（不连接）
    pub fn new() -> Self {
        Self { client: None }
    }

    /// 连接到指定实例
    pub async fn connect_to_instance(&mut self, instance_id: u32) -> Result<()> {
        let address = get_instance_address(instance_id);
        self.connect(&address, None).await
    }

    /// 连接到指定地址
    pub async fn connect(
        &mut self,
        address: impl Into<String>,
        auth_key: Option<String>,
    ) -> Result<()> {
        let address = address.into();
        debug!("Connecting to daemon at {}...", address);

        let mut client = IpcClient::new(address);
        if let Some(key) = auth_key {
            client = client.with_auth_key(key);
        }

        match timeout(Duration::from_secs(5), client.connect()).await {
            Ok(result) => {
                result.map_err(|e| anyhow::anyhow!("Connection failed: {}", e))?;
                self.client = Some(client);
                info!("Connected to daemon");
                Ok(())
            }
            Err(_) => Err(anyhow::anyhow!("Connection timeout")),
        }
    }

    /// 检查是否已连接
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
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

    /// 保存配置到文件
    pub async fn save_config(&mut self) -> Result<()> {
        let response = self.send_receive(&Message::SaveConfig).await?;

        match response {
            Message::ConfigLoaded => {
                info!("Configuration saved");
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
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to daemon"))?;

        match timeout(Duration::from_secs(5), client.send_receive(message)).await {
            Ok(result) => result.map_err(|e| anyhow::anyhow!("IPC error: {}", e)),
            Err(_) => Err(anyhow::anyhow!("Request timeout")),
        }
    }

    /// 关闭连接
    #[allow(dead_code)]
    pub async fn close(self) -> Result<()> {
        if let Some(client) = self.client {
            let _ = client.close().await;
        }
        Ok(())
    }

    /// 开始录制宏
    pub async fn start_macro_recording(&mut self, name: &str) -> Result<()> {
        let response = self
            .send_receive(&Message::StartMacroRecording {
                name: name.to_string(),
            })
            .await?;

        match response {
            Message::Success => Ok(()),
            Message::Error { message } => Err(anyhow::anyhow!("{}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// 停止录制宏
    pub async fn stop_macro_recording(&mut self) -> Result<(String, usize)> {
        let response = self.send_receive(&Message::StopMacroRecording).await?;

        match response {
            Message::MacroRecordingResult { name, action_count } => {
                Ok((name, action_count))
            }
            Message::Error { message } => Err(anyhow::anyhow!("{}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// 播放宏
    pub async fn play_macro(&mut self, name: &str) -> Result<()> {
        let response = self
            .send_receive(&Message::PlayMacro {
                name: name.to_string(),
            })
            .await?;

        match response {
            Message::Success => Ok(()),
            Message::Error { message } => Err(anyhow::anyhow!("{}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// 获取宏列表
    pub async fn get_macros(&mut self) -> Result<Vec<String>> {
        let response = self.send_receive(&Message::GetMacros).await?;

        match response {
            Message::MacrosList { macros } => Ok(macros),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// 删除宏
    pub async fn delete_macro(&mut self, name: &str) -> Result<()> {
        let response = self
            .send_receive(&Message::DeleteMacro {
                name: name.to_string(),
            })
            .await?;

        match response {
            Message::Success => Ok(()),
            Message::Error { message } => Err(anyhow::anyhow!("{}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// 绑定宏到触发键
    pub async fn bind_macro(&mut self, macro_name: &str, trigger: &str) -> Result<()> {
        let response = self
            .send_receive(&Message::BindMacro {
                macro_name: macro_name.to_string(),
                trigger: trigger.to_string(),
            })
            .await?;

        match response {
            Message::Success => Ok(()),
            Message::Error { message } => Err(anyhow::anyhow!("{}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}
