//! macOS application control
#![cfg(target_os = "macos")]

use crate::platform::traits::ApplicationControl;
use anyhow::Result;

impl ApplicationControl for super::platform_utils::MacosPlatform {
    fn detach_console() {}

    fn terminate_application() {
        <Self as crate::platform::traits::TrayLifecycle>::stop_tray()
    }

    fn open_folder(path: &std::path::Path) -> Result<()> {
        std::process::Command::new("open").arg(path).spawn()?;
        Ok(())
    }

    fn force_kill_instance(instance_id: u32) -> Result<()> {
        use std::process::{Command, Stdio};

        let process_name = if instance_id == 0 {
            "wakemd".to_string()
        } else {
            format!("wakemd-instance{}", instance_id)
        };

        let output = Command::new("pkill")
            .args(["-f", &process_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(result) if result.status.success() => {
                tracing::info!("Successfully killed daemon instance {}", instance_id);
                Ok(())
            }
            _ => {
                anyhow::bail!("Failed to kill daemon instance {}", instance_id)
            }
        }
    }
}
