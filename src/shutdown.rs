use tokio::sync::watch;
use tracing::{debug, info};

/// Graceful shutdown signal
///
/// Used to coordinate all long-running tasks to exit safely when shutdown signal is received
/// Features:
/// - Based on tokio::watch channel
/// - Supports multiple receivers listening simultaneously
/// - Thread-safe broadcast mechanism
#[derive(Clone)]
pub struct ShutdownSignal {
    sender: watch::Sender<bool>,
    receiver: watch::Receiver<bool>,
}

impl ShutdownSignal {
    /// Create new shutdown signal
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(false);
        Self { sender, receiver }
    }

    /// Get new receiver (for passing to child tasks)
    pub fn subscribe(&self) -> watch::Receiver<bool> {
        self.receiver.clone()
    }

    /// Trigger shutdown signal
    pub async fn shutdown(&self) {
        info!("Initiating graceful shutdown...");
        if self.sender.send(true).is_ok() {
            debug!("Shutdown signal sent to all subscribers");
        }
    }

    /// Check if shutdown signal has been received
    ///
    /// # Returns
    /// * `true` - Shutdown signal received
    /// * `false` - Shutdown signal not received
    #[allow(dead_code)]
    pub async fn is_shutdown_requested(&mut self) -> bool {
        self.receiver.changed().await.is_ok() && *self.receiver.borrow()
    }

    /// Wait for shutdown signal or complete operation
    ///
    /// # Arguments
    /// * `operation` - Operation to execute (Future)
    ///
    /// # Returns
    /// * `Ok(T)` - Operation completed successfully
    /// * `Err(())` - Shutdown signal received, operation cancelled
    #[allow(dead_code)]
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

        // Initial state should be false
        assert!(!*rx.borrow());

        // Trigger shutdown
        shutdown.shutdown().await;

        // Now should be true
        assert!(*rx.borrow());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let shutdown = ShutdownSignal::new();
        let mut rx1 = shutdown.subscribe();
        let mut rx2 = shutdown.subscribe();

        shutdown.shutdown().await;

        // Both receivers should receive signal
        assert!(*rx1.borrow());
        assert!(*rx2.borrow());
    }

    #[tokio::test]
    async fn test_run_until_shutdown_completion() {
        let mut shutdown = ShutdownSignal::new();

        // Normal completion should return Ok
        let result = shutdown.run_until_shutdown(async { Ok::<_, ()>(42) }).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_run_until_shutdown_cancellation() {
        use std::sync::Arc;
        let shutdown = Arc::new(ShutdownSignal::new());
        let shutdown_for_trigger = Arc::clone(&shutdown);

        // Trigger shutdown in background
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            shutdown_for_trigger.shutdown().await;
        });

        let mut shutdown_clone = (*shutdown).clone();
        // Long-running operation should be cancelled
        let result = timeout(
            Duration::from_millis(100),
            shutdown_clone.run_until_shutdown(async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok::<_, ()>(())
            }),
        )
        .await;

        assert!(result.is_ok()); // Should return before timeout
        assert!(result.unwrap().is_err()); // Should return error (cancelled)
    }
}
