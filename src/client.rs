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
                result.map_err(|e| anyhow::anyhow!("Connection failed: {e}"))?;
                self.client = Some(client);
                info!("Connected to daemon");
                Ok(())
            }
            Err(_) => Err(anyhow::anyhow!("Connection timeout")),
        }
    }

    /// Get server status
    pub async fn get_status(&mut self) -> Result<(bool, bool)> {
        let response = self.send_receive(&Message::GetStatus).await?;

        match response {
            Message::StatusResponse {
                active,
                config_loaded,
            } => Ok((active, config_loaded)),
            other => Err(anyhow::anyhow!(
                "Unexpected response: expected StatusResponse, got {:?}",
                other
            )),
        }
    }

    /// Set active state
    pub async fn set_active(&mut self, active: bool) -> Result<()> {
        let response = self.send_receive(&Message::SetActive { active }).await?;
        Self::expect_success(response, "SetActive")
    }

    /// Reload configuration
    pub async fn reload_config(&mut self) -> Result<()> {
        let response = self.send_receive(&Message::ReloadConfig).await?;
        Self::handle_config_response(response, "Configuration reloaded")
    }

    /// Save configuration to file
    pub async fn save_config(&mut self) -> Result<()> {
        let response = self.send_receive(&Message::SaveConfig).await?;
        Self::handle_config_response(response, "Configuration saved")
    }

    /// Handle config operation response (shared by reload and save)
    fn handle_config_response(response: Message, success_msg: &str) -> Result<()> {
        match response {
            Message::ConfigLoaded => {
                info!("{}", success_msg);
                Ok(())
            }
            Message::ConfigError { error } => {
                Err(anyhow::anyhow!("Config error: {error}"))
            }
            other => Err(anyhow::anyhow!(
                "Unexpected response: expected ConfigLoaded or ConfigError, got {:?}",
                other
            )),
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

    fn expect_success(response: Message, context: &str) -> Result<()> {
        match response {
            Message::Success => Ok(()),
            Message::Error { message } => Err(anyhow::anyhow!("{message}")),
            other => Err(anyhow::anyhow!(
                "Unexpected response for {}: expected Success or Error, got {:?}",
                context,
                other
            )),
        }
    }

    /// Start recording macro
    pub async fn start_macro_recording(&mut self, name: &str) -> Result<()> {
        let response = self
            .send_receive(&Message::StartMacroRecording {
                name: name.to_string(),
            })
            .await?;
        Self::expect_success(response, "StartMacroRecording")
    }

    /// Stop recording macro
    pub async fn stop_macro_recording(&mut self) -> Result<(String, usize)> {
        let response = self.send_receive(&Message::StopMacroRecording).await?;

        match response {
            Message::MacroRecordingResult { name, action_count } => {
                Ok((name, action_count))
            }
            Message::Error { message } => Err(anyhow::anyhow!("{message}")),
            other => Err(anyhow::anyhow!(
                "Unexpected response for StopMacroRecording: expected MacroRecordingResult or Error, got {:?}",
                other
            )),
        }
    }

    /// Play macro
    pub async fn play_macro(&mut self, name: &str) -> Result<()> {
        let response = self
            .send_receive(&Message::PlayMacro {
                name: name.to_string(),
            })
            .await?;
        Self::expect_success(response, "PlayMacro")
    }

    /// Get list of macros
    pub async fn get_macros(&mut self) -> Result<Vec<String>> {
        let response = self.send_receive(&Message::GetMacros).await?;

        match response {
            Message::MacrosList { macros } => Ok(macros),
            other => Err(anyhow::anyhow!(
                "Unexpected response for GetMacros: expected MacrosList, got {:?}",
                other
            )),
        }
    }

    /// Delete macro
    pub async fn delete_macro(&mut self, name: &str) -> Result<()> {
        let response = self
            .send_receive(&Message::DeleteMacro {
                name: name.to_string(),
            })
            .await?;
        Self::expect_success(response, "DeleteMacro")
    }

    /// Bind macro to trigger key
    pub async fn bind_macro(&mut self, macro_name: &str, trigger: &str) -> Result<()> {
        let response = self
            .send_receive(&Message::BindMacro {
                macro_name: macro_name.to_string(),
                trigger: trigger.to_string(),
            })
            .await?;
        Self::expect_success(response, "BindMacro")
    }

    /// Initialize platform-specific services
    #[allow(dead_code)]
    pub async fn initialize_platform(
        &mut self,
        native_handle: Option<usize>,
    ) -> Result<()> {
        let response = self
            .send_receive(&Message::InitializePlatform { native_handle })
            .await?;
        Self::expect_success(response, "InitializePlatform")
    }

    /// Shutdown the daemon
    pub async fn shutdown(&mut self) -> Result<()> {
        let response = self.send_receive(&Message::Shutdown).await?;
        Self::expect_success(response, "Shutdown")
    }
}

impl Default for DaemonClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::{get_instance_address, get_instance_port, Message};

    // ==================== DaemonClient initialization ====================

    // ==================== Error handling in disconnected state ====================

    /// Test get_status should return error when not connected
    #[tokio::test]
    async fn test_get_status_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.get_status().await;
        assert!(
            result.is_err(),
            "get_status should return error when not connected"
        );
    }

    /// Test set_active should return error when not connected
    #[tokio::test]
    async fn test_set_active_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.set_active(true).await;
        assert!(
            result.is_err(),
            "set_active should return error when not connected"
        );
    }

    /// Test reload_config should return error when not connected
    #[tokio::test]
    async fn test_reload_config_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.reload_config().await;
        assert!(
            result.is_err(),
            "reload_config should return error when not connected"
        );
    }

    /// Test save_config should return error when not connected
    #[tokio::test]
    async fn test_save_config_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.save_config().await;
        assert!(
            result.is_err(),
            "save_config should return error when not connected"
        );
    }

    /// Test start_macro_recording should return error when not connected
    #[tokio::test]
    async fn test_start_macro_recording_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.start_macro_recording("test").await;
        assert!(
            result.is_err(),
            "start_macro_recording should return error when not connected"
        );
    }

    /// Test stop_macro_recording should return error when not connected
    #[tokio::test]
    async fn test_stop_macro_recording_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.stop_macro_recording().await;
        assert!(
            result.is_err(),
            "stop_macro_recording should return error when not connected"
        );
    }

    /// Test play_macro should return error when not connected
    #[tokio::test]
    async fn test_play_macro_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.play_macro("test").await;
        assert!(
            result.is_err(),
            "play_macro should return error when not connected"
        );
    }

    /// Test get_macros should return error when not connected
    #[tokio::test]
    async fn test_get_macros_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.get_macros().await;
        assert!(
            result.is_err(),
            "get_macros should return error when not connected"
        );
    }

    /// Test delete_macro should return error when not connected
    #[tokio::test]
    async fn test_delete_macro_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.delete_macro("test").await;
        assert!(
            result.is_err(),
            "delete_macro should return error when not connected"
        );
    }

    /// Test bind_macro should return error when not connected
    #[tokio::test]
    async fn test_bind_macro_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.bind_macro("test", "F5").await;
        assert!(
            result.is_err(),
            "bind_macro should return error when not connected"
        );
    }

    /// Test initialize_platform should return error when not connected
    #[tokio::test]
    async fn test_initialize_platform_not_connected() {
        let mut client = DaemonClient::new();
        let result = client.initialize_platform(Some(12345)).await;
        assert!(
            result.is_err(),
            "initialize_platform should return error when not connected"
        );
    }

    // ==================== IPC message serialization validation ====================

    /// Verify that message types used by the client can be correctly serialized/deserialized
    #[test]
    fn test_client_message_serialization() {
        // GetStatus message
        let msg = Message::GetStatus;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::GetStatus));

        // SetActive message
        let msg = Message::SetActive { active: true };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        if let Message::SetActive { active } = deserialized {
            assert!(active);
        } else {
            panic!("Expected SetActive message");
        }

        // ReloadConfig message
        let msg = Message::ReloadConfig;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::ReloadConfig));

        // SaveConfig message
        let msg = Message::SaveConfig;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::SaveConfig));

        // StartMacroRecording message
        let msg = Message::StartMacroRecording {
            name: "test".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        if let Message::StartMacroRecording { name } = deserialized {
            assert_eq!(name, "test");
        } else {
            panic!("Expected StartMacroRecording message");
        }

        // StopMacroRecording message
        let msg = Message::StopMacroRecording;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::StopMacroRecording));

        // PlayMacro message
        let msg = Message::PlayMacro {
            name: "macro1".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        if let Message::PlayMacro { name } = deserialized {
            assert_eq!(name, "macro1");
        } else {
            panic!("Expected PlayMacro message");
        }

        // GetMacros message
        let msg = Message::GetMacros;
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, Message::GetMacros));

        // DeleteMacro message
        let msg = Message::DeleteMacro {
            name: "old_macro".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        if let Message::DeleteMacro { name } = deserialized {
            assert_eq!(name, "old_macro");
        } else {
            panic!("Expected DeleteMacro message");
        }

        // BindMacro message
        let msg = Message::BindMacro {
            macro_name: "my_macro".to_string(),
            trigger: "F5".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        if let Message::BindMacro {
            macro_name,
            trigger,
        } = deserialized
        {
            assert_eq!(macro_name, "my_macro");
            assert_eq!(trigger, "F5");
        } else {
            panic!("Expected BindMacro message");
        }

        // InitializePlatform message
        let msg = Message::InitializePlatform {
            native_handle: Some(12345),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        if let Message::InitializePlatform { native_handle } = deserialized {
            assert_eq!(native_handle, Some(12345));
        } else {
            panic!("Expected InitializePlatform message");
        }
    }

    // ==================== Response message parsing validation ====================

    /// Verify parsing of StatusResponse response
    #[test]
    fn test_status_response_parsing() {
        let response = Message::StatusResponse {
            active: true,
            config_loaded: false,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        match parsed {
            Message::StatusResponse {
                active,
                config_loaded,
            } => {
                assert!(active);
                assert!(!config_loaded);
            }
            _ => panic!("Expected StatusResponse"),
        }
    }

    /// Verify parsing of MacrosList response
    #[test]
    fn test_macros_list_response_parsing() {
        let response = Message::MacrosList {
            macros: vec![
                "macro1".to_string(),
                "macro2".to_string(),
                "macro3".to_string(),
            ],
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        match parsed {
            Message::MacrosList { macros } => {
                assert_eq!(macros.len(), 3);
                assert!(macros.contains(&"macro1".to_string()));
                assert!(macros.contains(&"macro2".to_string()));
                assert!(macros.contains(&"macro3".to_string()));
            }
            _ => panic!("Expected MacrosList"),
        }
    }

    /// Verify parsing of MacroRecordingResult response
    #[test]
    fn test_macro_recording_result_parsing() {
        let response = Message::MacroRecordingResult {
            name: "test_macro".to_string(),
            action_count: 10,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: Message = serde_json::from_str(&json).unwrap();

        match parsed {
            Message::MacroRecordingResult { name, action_count } => {
                assert_eq!(name, "test_macro");
                assert_eq!(action_count, 10);
            }
            _ => panic!("Expected MacroRecordingResult"),
        }
    }

    /// Verify parsing of Error response
    #[test]
    fn test_error_response_parsing() {
        let error_messages = vec![
            "Connection failed",
            "Config not found",
            "Unknown error",
            "",
            "Special character test 🎉",
        ];

        for msg in error_messages {
            let response = Message::Error {
                message: msg.to_string(),
            };
            let json = serde_json::to_string(&response).unwrap();
            let parsed: Message = serde_json::from_str(&json).unwrap();

            if let Message::Error { message } = parsed {
                assert_eq!(message, msg);
            } else {
                panic!("Expected Error response");
            }
        }
    }

    // ==================== Connection parameter validation ====================

    /// Verify instance address generation logic
    #[test]
    fn test_instance_address_generation() {
        // Instance 0
        assert_eq!(get_instance_port(0), 57427);
        assert_eq!(get_instance_address(0), "127.0.0.1:57427");

        // Instance 1
        assert_eq!(get_instance_port(1), 57428);
        assert_eq!(get_instance_address(1), "127.0.0.1:57428");

        // Instance 5
        assert_eq!(get_instance_port(5), 57432);
        assert_eq!(get_instance_address(5), "127.0.0.1:57432");

        // Large instance ID
        assert_eq!(get_instance_port(100), 57527);
        assert_eq!(get_instance_address(100), "127.0.0.1:57527");
    }
}
