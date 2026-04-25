//! macOS program launcher implementation
#![cfg(target_os = "macos")]

use crate::platform::launcher_common::CommonLauncher;
use crate::types::LaunchAction;
use anyhow::Result;
use std::process::Command;
use tracing::info;

/// Program launcher (macOS-specific wrapper with additional features)
#[derive(Debug, Clone)]
pub struct Launcher {
    inner: CommonLauncher,
}

impl Launcher {
    /// Create a new launcher
    pub fn new() -> Self {
        Self {
            inner: CommonLauncher::new(),
        }
    }

    /// Execute launch action
    pub fn launch(&self, action: &LaunchAction) -> Result<()> {
        self.inner.launch(action)
    }

    /// Create a simple launch action from string
    pub fn create_action(program: impl Into<String>) -> LaunchAction {
        CommonLauncher::create_action(program)
    }

    /// Parse from command line string (e.g., "open -a Safari")
    pub fn parse_command(command: &str) -> LaunchAction {
        CommonLauncher::parse_command(command)
    }

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

impl Default for Launcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let action = Launcher::parse_command("open -a Safari");
        assert_eq!(action.program, "open");
        assert_eq!(action.args, vec!["-a", "Safari"]);
    }

    #[test]
    fn test_parse_command_no_args() {
        let action = Launcher::parse_command("open");
        assert_eq!(action.program, "open");
        assert!(action.args.is_empty());
    }

    #[test]
    fn test_create_action() {
        let action = Launcher::create_action("Safari");
        assert_eq!(action.program, "Safari");
        assert!(action.args.is_empty());
    }
}
