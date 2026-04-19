use anyhow::Result;
use tracing::info;

mod platform;
mod runtime;
mod server;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("wakemd starting...");

    // 运行服务端
    server::run_server().await?;

    info!("wakemd shutting down...");
    Ok(())
}
