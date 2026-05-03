//! macOS application control
#![cfg(target_os = "macos")]

use crate::platform::common::app_control;
use crate::platform::traits::ApplicationControl;
use anyhow::Result;

impl ApplicationControl for super::platform_utils::MacosPlatform {
    fn detach_console() {}

    fn open_folder(path: &std::path::Path) -> Result<()> {
        app_control::open_folder(path)
    }

    fn force_kill_instance(instance_id: u32) -> Result<()> {
        use std::process::{Command, Stdio};

        let process_name = app_control::daemon_process_name(instance_id);

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
