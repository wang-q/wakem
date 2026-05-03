//! Windows application control
#![cfg(target_os = "windows")]

use crate::platform::traits::ApplicationControl;
use anyhow::Result;

impl ApplicationControl for super::platform_utils::WindowsPlatform {
    fn detach_console() {
        use windows::Win32::System::Console::FreeConsole;
        unsafe {
            let _ = FreeConsole();
        }
    }

    fn terminate_application() {
        super::tray::stop_tray()
    }

    fn open_folder(path: &std::path::Path) -> Result<()> {
        std::process::Command::new("explorer").arg(path).spawn()?;
        Ok(())
    }

    fn force_kill_instance(instance_id: u32) -> Result<()> {
        use std::process::{Command, Stdio};

        let window_title = if instance_id == 0 {
            "wakemd".to_string()
        } else {
            format!("wakemd-instance{}", instance_id)
        };

        let output = Command::new("taskkill")
            .args(["/F", "/FI", &format!("WINDOWTITLE eq {}", window_title)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(result) if result.status.success() => {
                tracing::info!("Successfully killed daemon instance {}", instance_id);
                return Ok(());
            }
            _ => {
                tracing::debug!("Could not kill by window title, trying PowerShell");
            }
        }

        let ps_script = if instance_id == 0 {
            r#"Get-Process wakem -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -notmatch '--instance' } | Stop-Process -Force"#.to_string()
        } else {
            format!(
                r#"Get-Process wakem -ErrorAction SilentlyContinue | Where-Object {{ $_.CommandLine -match '--instance {}' }} | Stop-Process -Force"#,
                instance_id
            )
        };

        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        match output {
            Ok(result) if result.status.success() => {
                tracing::info!(
                    "Successfully killed daemon instance {} via PowerShell",
                    instance_id
                );
                Ok(())
            }
            _ => {
                anyhow::bail!("Failed to kill daemon instance {}", instance_id)
            }
        }
    }
}
