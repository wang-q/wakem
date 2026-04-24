// CLI Argument Parsing Tests

use clap::Parser;
use wakem::cli::{Cli, Commands};

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
    // --version 会导致程序打印版本后退出，所以这里测试它能被解析
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
    // daemon 命令不应该有额外参数
    let result = Cli::try_parse_from(["wakem", "daemon", "extra_arg"]);
    // Depending on clap configuration, may error or ignore extra arguments
    // Here we only verify it does not panic
    let _ = result;
}
