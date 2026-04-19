use anyhow::Result;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("wakem client starting...");

    // TODO: 实现客户端主逻辑
    // 1. 连接到服务端
    // 2. 初始化系统托盘
    // 3. 启动消息循环

    info!("wakem client started successfully");

    // 保持运行
    tokio::signal::ctrl_c().await?;

    info!("wakem client shutting down...");
    Ok(())
}
