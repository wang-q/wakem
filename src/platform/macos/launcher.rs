//! macOS program launcher implementation
#![cfg(target_os = "macos")]

use crate::platform::launcher_common::Launcher;
use anyhow::Result;
use std::process::Command;
use tracing::info;

impl Launcher {
    /// Open a file or URL using the default application (macOS-specific)
    pub fn open(&self, path: &str) -> Result<()> {
        let mut cmd = Command::new("open");
        cmd.arg(path);

        match cmd.spawn() {
            Ok(_) => {
                info!("Opened: {}", path);
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to open {}: {}", path, e)),
        }
    }
}

pub use crate::platform::launcher_common::Launcher;
