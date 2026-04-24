//! Command line argument parsing
//!
//! Unified CLI definition, serving as the single source of command line interface for the entire project.

use clap::{Parser, Subcommand};

/// wakem - Window Adjust, Keyboard Enhance, and Mouse
#[derive(Parser)]
#[command(name = "wakem")]
#[command(about = "wakem - Window/Keyboard/Mouse Enhancer")]
#[command(version)]
pub struct Cli {
    /// Instance ID (for multi-instance support)
    #[arg(short, long, default_value = "0")]
    pub instance: u32,

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start daemon
    Daemon {
        /// Instance ID
        #[arg(short, long, default_value = "0")]
        instance: u32,
    },
    /// Get server status
    Status,
    /// Reload configuration
    Reload,
    /// Save current configuration to file
    Save,
    /// Enable mapping
    Enable,
    /// Disable mapping
    Disable,
    /// Open config folder
    Config,
    /// List running instances
    Instances,
    /// Run system tray (default)
    Tray,
    /// Record macro
    Record {
        /// Macro name
        name: String,
    },
    /// Stop recording macro
    StopRecord,
    /// Play macro
    Play {
        /// Macro name
        name: String,
    },
    /// List all macros
    Macros,
    /// Bind macro to trigger key
    BindMacro {
        /// Macro name
        macro_name: String,
        /// Trigger key
        trigger: String,
    },
    /// Delete macro
    DeleteMacro {
        /// Macro name
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Test no args (should use default command Tray or None)
    #[test]
    fn test_cli_no_args() {
        let cli = Cli::try_parse_from(["wakem"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        // command should be None when no args (will use default Tray)
        assert!(cli.command.is_none());
    }

    /// Test daemon subcommand
    #[test]
    fn test_cli_daemon_command() {
        let cli = Cli::try_parse_from(["wakem", "daemon"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Daemon { .. })));
    }

    /// Test status subcommand
    #[test]
    fn test_cli_status_command() {
        let cli = Cli::try_parse_from(["wakem", "status"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Status)));
    }

    /// Test reload subcommand
    #[test]
    fn test_cli_reload_command() {
        let cli = Cli::try_parse_from(["wakem", "reload"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Reload)));
    }

    /// Test enable subcommand
    #[test]
    fn test_cli_enable_command() {
        let cli = Cli::try_parse_from(["wakem", "enable"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Enable)));
    }

    /// Test disable subcommand
    #[test]
    fn test_cli_disable_command() {
        let cli = Cli::try_parse_from(["wakem", "disable"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Disable)));
    }

    /// Test config subcommand
    #[test]
    fn test_cli_config_command() {
        let cli = Cli::try_parse_from(["wakem", "config"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Config)));
    }

    /// Test tray subcommand (explicit)
    #[test]
    fn test_cli_tray_command() {
        let cli = Cli::try_parse_from(["wakem", "tray"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Tray)));
    }

    /// Test --version flag
    #[test]
    fn test_cli_version_flag() {
        // --version causes the program to print version and exit, so we only test that it can be parsed
        // clap version behavior causes process exit, so we only verify parameters can be recognized
        let result = Cli::try_parse_from(["wakem", "--version"]);
        // This call usually triggers version display and exit, so may return Err
        // We mainly verify parameter format is correct
        assert!(result.is_err() || result.is_ok());
    }

    /// Test --help flag
    #[test]
    fn test_cli_help_flag() {
        // Similar to --version, --help displays help information
        let result = Cli::try_parse_from(["wakem", "--help"]);
        assert!(result.is_err() || result.is_ok());
    }

    /// Test invalid command error handling
    #[test]
    fn test_cli_invalid_command() {
        let result = Cli::try_parse_from(["wakem", "invalid_command"]);
        assert!(result.is_err());
    }

    /// Test subcommand case sensitivity (should be case-sensitive)
    #[test]
    fn test_cli_case_sensitivity() {
        // Uppercase Daemon should not parse
        let result = Cli::try_parse_from(["wakem", "Daemon"]);
        assert!(result.is_err());
    }

    /// Test extra arguments handling
    #[test]
    fn test_cli_extra_arguments() {
        // daemon command should not have extra arguments
        let result = Cli::try_parse_from(["wakem", "daemon", "extra_arg"]);
        // Depending on clap configuration, may error or ignore extra arguments
        // Here we only verify it does not panic
        let _ = result;
    }
}
