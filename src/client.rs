use anyhow::Result;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

use crate::constants::IPC_CONNECTION_TIMEOUT_SECS;
use crate::ipc::{get_instance_address, IpcClient, Message};

/// Daemon client
pub struct DaemonClient {
    client: Option<IpcClient>,
}

impl DaemonClient {
    /// Create new client (not connected)
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Connect to specified instance
    pub async fn connect_to_instance(&mut self, instance_id: u32) -> Result<()> {
        let address = get_instance_address(instance_id);
        self.connect(&address, None).await
    }

    /// Connect to specified address
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

        match timeout(
            Duration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
            client.connect(),
        )
        .await
        {
            Ok(result) => {
                result.map_err(|e| anyhow::anyhow!("Connection failed: {}", e))?;
                self.client = Some(client);
                info!("Connected to daemon");
                Ok(())
            }
            Err(_) => Err(anyhow::anyhow!("Connection timeout")),
        }
    }

    /// Check if connected
    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    /// Get server status
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

    /// Set active state
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

    /// Reload configuration
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

    /// Save configuration to file
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

    /// Send message and wait for response
    async fn send_receive(&mut self, message: &Message) -> Result<Message> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to daemon"))?;

        match timeout(
            Duration::from_secs(IPC_CONNECTION_TIMEOUT_SECS),
            client.send_receive(message),
        )
        .await
        {
            Ok(result) => result.map_err(|e| anyhow::anyhow!("IPC error: {}", e)),
            Err(_) => Err(anyhow::anyhow!("Request timeout")),
        }
    }

    /// Start recording macro
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

    /// Stop recording macro
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

    /// Play macro
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

    /// Get list of macros
    pub async fn get_macros(&mut self) -> Result<Vec<String>> {
        let response = self.send_receive(&Message::GetMacros).await?;

        match response {
            Message::MacrosList { macros } => Ok(macros),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// Delete macro
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

    /// Bind macro to trigger key
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

    /// Register message window handle
    pub async fn register_message_window(&mut self, hwnd: usize) -> Result<()> {
        let response = self
            .send_receive(&Message::RegisterMessageWindow { hwnd })
            .await?;

        match response {
            Message::Success => Ok(()),
            Message::Error { message } => Err(anyhow::anyhow!("{}", message)),
            _ => Err(anyhow::anyhow!("Unexpected response")),
        }
    }

    /// Shutdown the daemon
    pub async fn shutdown(&mut self) -> Result<()> {
        let response = self.send_receive(&Message::Shutdown).await?;

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
