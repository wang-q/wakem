use tokio::sync::watch;
use tracing::{debug, info};

/// 优雅关闭信号
///
/// 用于协调所有长时间运行的任务在收到关闭信号时安全退出
/// 特性：
/// - 基于 tokio::watch channel 实现
/// - 支持多个接收者同时监听
/// - 线程安全的广播机制
#[derive(Clone)]
pub struct ShutdownSignal {
    sender: watch::Sender<bool>,
    receiver: watch::Receiver<bool>,
}

impl ShutdownSignal {
    /// 创建新的关闭信号
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(false);
        Self { sender, receiver }
    }

    /// 获取新的接收者（用于传递给子任务）
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.receiver.clone()
    }

    /// 触发关闭信号
    pub async fn shutdown(&self) {
        info!("Initiating graceful shutdown...");
        if self.sender.send(true).is_ok() {
            debug!("Shutdown signal sent to all subscribers");
        }
    }

    /// 检查是否已收到关闭信号
    ///
    /// # 返回值
    /// * `true` - 已收到关闭信号
    /// * `false` - 未收到关闭信号
    pub async fn is_shutdown_requested(&mut self) -> bool {
        self.receiver.changed().await.is_ok() && *self.receiver.borrow()
    }

    /// 等待关闭信号或完成操作
    ///
    /// # 参数
    /// * `operation` - 要执行的操作（Future）
    ///
    /// # 返回值
    /// * `Ok(T)` - 操作成功完成
    /// * `Err(())` - 收到关闭信号，操作被取消
    pub async fn run_until_shutdown<F, T, E>(
        &mut self,
        operation: F,
    ) -> std::result::Result<T, ()>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
    {
        tokio::select! {
            result = operation => match result {
                Ok(value) => Ok(value),
                Err(_) => Err(()),
            },
            _ = self.receiver.changed() => {
                info!("Operation cancelled due to shutdown signal");
                Err(())
            },
        }
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_basic_shutdown() {
        let shutdown = ShutdownSignal::new();
        let mut rx = shutdown.subscribe();

        // 初始状态应该是 false
        assert!(!*rx.borrow());

        // 触发关闭
        shutdown.shutdown().await;

        // 现在应该是 true
        assert!(*rx.borrow());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let shutdown = ShutdownSignal::new();
        let mut rx1 = shutdown.subscribe();
        let mut rx2 = shutdown.subscribe();

        shutdown.shutdown().await;

        // 两个接收者都应该收到信号
        assert!(*rx1.borrow());
        assert!(*rx2.borrow());
    }

    #[tokio::test]
    async fn test_run_until_shutdown_completion() {
        let mut shutdown = ShutdownSignal::new();

        // 正常完成的操作应该返回 Ok
        let result = shutdown.run_until_shutdown(async { Ok::<_, ()>(42) }).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_run_until_shutdown_cancellation() {
        use std::sync::Arc;
        let shutdown = Arc::new(ShutdownSignal::new());
        let shutdown_for_trigger = Arc::clone(&shutdown);

        // 在后台触发关闭
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            shutdown_for_trigger.shutdown().await;
        });

        let mut shutdown_clone = (*shutdown).clone();
        // 长时间运行的操作应该被取消
        let result = timeout(
            Duration::from_millis(100),
            shutdown_clone.run_until_shutdown(async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok::<_, ()>(())
            }),
        )
        .await;

        assert!(result.is_ok()); // 应该在超时前返回
        assert!(result.unwrap().is_err()); // 应该返回错误（被取消）
    }
}
