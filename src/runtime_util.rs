use anyhow::Result;
use std::future::Future;

use crate::client::DaemonClient;

/// Execute a closure with a cached single-threaded tokio runtime.
///
/// Using a thread-local runtime avoids the overhead of creating and destroying
/// a runtime for every CLI command. This is especially beneficial when multiple
/// commands are issued in quick succession.
///
/// The runtime reference is only valid within the closure, avoiding the need
/// for unsafe lifetime extension.
///
/// # Limitations
///
/// - The runtime is stored in thread-local storage, so it will be destroyed
///   when the thread exits. This function is designed for short-lived CLI
///   commands and should not be used in long-running threads.
/// - Each thread gets its own runtime instance, so this does not share
///   runtimes across threads.
pub fn with_runtime<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&tokio::runtime::Runtime) -> Result<R>,
{
    thread_local! {
        static RUNTIME: tokio::runtime::Runtime = {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime")
        };
    }

    RUNTIME.with(f)
}

/// Execute an async operation with a daemon client connection
/// Reuses a cached runtime to avoid repeated creation/destruction overhead.
/// The closure receives ownership of the client to avoid async lifetime issues.
pub fn run_with_client<F, Fut>(instance_id: u32, op: F) -> Result<()>
where
    F: FnOnce(DaemonClient) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    with_runtime(|rt| {
        rt.block_on(async {
            let mut client = DaemonClient::new();
            client.connect_to_instance(instance_id).await?;
            op(client).await
        })
    })
}

/// Execute an async operation with a cached tokio runtime
/// Similar to `run_with_client` but without connecting to a daemon instance.
pub fn run_async<F, Fut>(op: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<()>>,
{
    with_runtime(|rt| rt.block_on(op()))
}

/// Run the daemon server with a multi-threaded tokio runtime
pub fn run_daemon(
    instance_id: u32,
    preloaded_config: Option<crate::config::Config>,
    config_path: Option<std::path::PathBuf>,
) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        tracing::info!("Starting wakemd (instance {})...", instance_id);
        crate::daemon::run_server_with_config(instance_id, preloaded_config, config_path)
            .await
    })
}

/// Check if a daemon instance is currently running
pub fn is_daemon_running(instance_id: u32) -> bool {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(_) => return false,
    };

    rt.block_on(async {
        let mut client = DaemonClient::new();
        client.connect_to_instance(instance_id).await.is_ok()
    })
}
