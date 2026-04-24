use tokio::sync::watch;
use tracing::{debug, info};

/// Error type for shutdown-related operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownError {
    /// Shutdown signal was received, operation cancelled
    Cancelled,
    /// The operation itself failed
    OperationFailed,
}

impl std::fmt::Display for ShutdownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShutdownError::Cancelled => {
                write!(f, "Operation cancelled due to shutdown signal")
            }
            ShutdownError::OperationFailed => write!(f, "Operation failed"),
        }
    }
}

impl std::error::Error for ShutdownError {}

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

    /// Wait for shutdown signal
    ///
    /// # Returns
    /// * `true` - Shutdown signal received
    /// * `false` - Channel closed without shutdown signal
    #[allow(dead_code)]
    pub async fn wait_for_shutdown(&mut self) -> bool {
        self.receiver.changed().await.is_ok() && *self.receiver.borrow()
    }

    /// Check if shutdown signal has been received (non-blocking)
    ///
    /// # Returns
    /// * `true` - Shutdown signal has been received
    /// * `false` - Shutdown signal not yet received
    #[allow(dead_code)]
    pub fn is_shutdown_requested(&self) -> bool {
        *self.receiver.borrow()
    }

    /// Wait for shutdown signal or complete operation
    ///
    /// # Arguments
    /// * `operation` - Operation to execute (Future)
    ///
    /// # Returns
    /// * `Ok(T)` - Operation completed successfully
    /// * `Err(ShutdownError::Cancelled)` - Shutdown signal received, operation cancelled
    /// * `Err(ShutdownError::OperationFailed)` - Operation failed
    #[allow(dead_code)]
    pub async fn run_until_shutdown<F, T, E>(
        &mut self,
        operation: F,
    ) -> std::result::Result<T, ShutdownError>
    where
        F: std::future::Future<Output = std::result::Result<T, E>>,
    {
        tokio::select! {
            result = operation => match result {
                Ok(value) => Ok(value),
                Err(_) => Err(ShutdownError::OperationFailed),
            },
            _ = self.receiver.changed() => {
                info!("Operation cancelled due to shutdown signal");
                Err(ShutdownError::Cancelled)
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
        let rx = shutdown.subscribe();

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
        let rx1 = shutdown.subscribe();
        let rx2 = shutdown.subscribe();

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
        assert_eq!(result.unwrap(), Err(ShutdownError::Cancelled)); // Should return Cancelled error
    }

    #[tokio::test]
    async fn test_is_shutdown_requested_non_blocking() {
        let shutdown = ShutdownSignal::new();

        // Initially should be false
        assert!(!shutdown.is_shutdown_requested());

        // Trigger shutdown
        shutdown.shutdown().await;

        // Now should be true (non-blocking check)
        assert!(shutdown.is_shutdown_requested());
    }

    #[tokio::test]
    async fn test_wait_for_shutdown() {
        use std::sync::Arc;
        let shutdown = Arc::new(ShutdownSignal::new());
        let shutdown_for_trigger = Arc::clone(&shutdown);

        // Trigger shutdown in background
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            shutdown_for_trigger.shutdown().await;
        });

        let mut shutdown_clone = (*shutdown).clone();
        // Wait for shutdown signal
        let result = timeout(
            Duration::from_millis(100),
            shutdown_clone.wait_for_shutdown(),
        )
        .await;

        assert!(result.is_ok()); // Should return before timeout
        assert!(result.unwrap()); // Should return true (shutdown requested)
    }

    #[tokio::test]
    async fn test_run_until_shutdown_operation_failed() {
        let mut shutdown = ShutdownSignal::new();

        // Operation failure should return OperationFailed error
        let result: Result<i32, ShutdownError> = shutdown
            .run_until_shutdown(async { Err::<i32, &str>("operation failed") })
            .await;
        assert_eq!(result, Err(ShutdownError::OperationFailed));
    }
}
