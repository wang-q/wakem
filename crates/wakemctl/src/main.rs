use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "wakemctl")]
#[command(about = "Control utility for wakem")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check service status
    Status,
    /// Reload configuration
    Reload,
    /// Stop the service
    Stop,
    /// Start the service
    Start,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => {
            println!("Checking wakemd status...");
            // TODO: 实现状态检查
        }
        Commands::Reload => {
            println!("Reloading configuration...");
            // TODO: 实现配置重载
        }
        Commands::Stop => {
            println!("Stopping wakemd...");
            // TODO: 实现停止服务
        }
        Commands::Start => {
            println!("Starting wakemd...");
            // TODO: 实现启动服务
        }
    }

    Ok(())
}
