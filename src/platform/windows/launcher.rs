//! Windows program launcher implementation
#![cfg(target_os = "windows")]

use crate::platform::launcher_common::CommonLauncher;
use crate::types::LaunchAction;
use anyhow::Result;

/// Program launcher (Windows-specific wrapper)
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Launcher {
    inner: CommonLauncher,
}

#[allow(dead_code)]
impl Launcher {
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

    /// Parse from command line string (e.g., "notepad.exe file.txt")
    pub fn parse_command(command: &str) -> LaunchAction {
        CommonLauncher::parse_command(command)
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
        let action = Launcher::parse_command("notepad.exe file.txt");
        assert_eq!(action.program, "notepad.exe");
        assert_eq!(action.args, vec!["file.txt"]);
    }

    #[test]
    fn test_parse_command_no_args() {
        let action = Launcher::parse_command("calc.exe");
        assert_eq!(action.program, "calc.exe");
        assert!(action.args.is_empty());
    }
}
