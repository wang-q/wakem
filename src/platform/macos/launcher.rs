//! macOS program launcher
//!
//! This module is cross-platform compatible using std::process::Command.
#![cfg(target_os = "macos")]

use crate::types::LaunchAction;
use anyhow::Result;
use std::path::Path;
use std::process::Command;
use tracing::{debug, info};

/// Program launcher
#[derive(Debug, Clone)]
pub struct Launcher;

impl Launcher {
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

    /// Parse from command line string (e.g., "open -a Safari")
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

    /// Open a file or URL using the default application
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
