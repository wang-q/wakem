//! Common launcher implementation shared across platforms
//!
//! This module provides a cross-platform program launcher using std::process::Command.

use crate::types::LaunchAction;
use anyhow::Result;
use std::process::Command;
use tracing::{debug, info};

/// Cross-platform program launcher
///
/// Provides program launching capabilities using the standard library's
/// Command API. Platform-specific extensions (e.g., macOS `open`) are
/// added via conditional impl blocks.
#[derive(Debug, Clone)]
pub struct Launcher;

impl Launcher {
    pub fn new() -> Self {
        Self
    }

    pub fn launch(&self, action: &LaunchAction) -> Result<()> {
        info!("Launching program: {}", action.program);
        debug!("Args: {:?}", action.args);
        debug!("Working dir: {:?}", action.working_dir);

        let program = action.program.clone();

        let mut cmd = Command::new(&program);
        cmd.args(&action.args);

        if let Some(ref dir) = action.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &action.env_vars {
            cmd.env(key, value);
        }

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

    #[test]
    fn test_parse_command_multiple_args() {
        let action = Launcher::parse_command("open -a Safari");
        assert_eq!(action.program, "open");
        assert_eq!(action.args, vec!["-a", "Safari"]);
    }

    #[test]
    fn test_parse_command_empty() {
        let action = Launcher::parse_command("");
        assert!(action.program.is_empty());
        assert!(action.args.is_empty());
    }

    #[test]
    fn test_launcher_creation() {
        let launcher = Launcher::new();
        let _cloned = launcher.clone();
    }

    #[test]
    fn test_launcher_default() {
        let _launcher = Launcher::default();
    }
}
