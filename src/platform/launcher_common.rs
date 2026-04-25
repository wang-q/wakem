//! Common launcher implementation shared across platforms
//!
//! This module provides a cross-platform program launcher using std::process::Command.

use crate::types::LaunchAction;
use anyhow::Result;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

/// Common program launcher
///
/// This struct provides cross-platform program launching capabilities
/// using the standard library's Command API.
#[derive(Debug, Clone)]
pub struct CommonLauncher;

impl CommonLauncher {
    /// Create a new launcher
    pub fn new() -> Self {
        Self
    }

    /// Execute launch action
    pub fn launch(&self, action: &LaunchAction) -> Result<()> {
        info!("Launching program: {}", action.program);
        debug!("Args: {:?}", action.args);
        debug!("Working dir: {:?}", action.working_dir);

        // Check program path
        let program = if Path::new(&action.program).exists() {
            action.program.clone()
        } else {
            // Try to find in PATH
            action.program.clone()
        };

        // Use std::process::Command to launch program
        let mut cmd = Command::new(&program);
        cmd.args(&action.args);

        if let Some(ref dir) = action.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &action.env_vars {
            cmd.env(key, value);
        }

        // Launch asynchronously, don't wait
        match cmd.spawn() {
            Ok(child) => {
                info!(
                    "Program launched successfully: {} (pid: {:?})",
                    action.program,
                    child.id()
                );
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!(
                "Failed to launch program {}: {}",
                action.program,
                e
            )),
        }
    }

    /// Create a simple launch action from string
    pub fn create_action(program: impl Into<String>) -> LaunchAction {
        LaunchAction {
            program: program.into(),
            args: Vec::new(),
            working_dir: None,
            env_vars: Vec::new(),
        }
    }

    /// Parse from command line string (e.g., "notepad.exe file.txt")
    pub fn parse_command(command: &str) -> LaunchAction {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return LaunchAction {
                program: String::new(),
                args: Vec::new(),
                working_dir: None,
                env_vars: Vec::new(),
            };
        }

        LaunchAction {
            program: parts[0].to_string(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
            working_dir: None,
            env_vars: Vec::new(),
        }
    }
}

impl Default for CommonLauncher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let action = CommonLauncher::parse_command("notepad.exe file.txt");
        assert_eq!(action.program, "notepad.exe");
        assert_eq!(action.args, vec!["file.txt"]);
    }

    #[test]
    fn test_parse_command_no_args() {
        let action = CommonLauncher::parse_command("calc.exe");
        assert_eq!(action.program, "calc.exe");
        assert!(action.args.is_empty());
    }

    #[test]
    fn test_create_action() {
        let action = CommonLauncher::create_action("Safari");
        assert_eq!(action.program, "Safari");
        assert!(action.args.is_empty());
    }

    #[test]
    fn test_parse_command_multiple_args() {
        let action = CommonLauncher::parse_command("open -a Safari");
        assert_eq!(action.program, "open");
        assert_eq!(action.args, vec!["-a", "Safari"]);
    }

    #[test]
    fn test_parse_command_empty() {
        let action = CommonLauncher::parse_command("");
        assert!(action.program.is_empty());
        assert!(action.args.is_empty());
    }

    #[test]
    fn test_launcher_creation() {
        let launcher = CommonLauncher::new();
        let _cloned = launcher.clone();
    }

    #[test]
    fn test_launcher_default() {
        let _launcher = CommonLauncher::default();
    }
}
