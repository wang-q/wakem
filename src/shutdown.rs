use tokio::sync::watch;
use tracing::{debug, info, trace};

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
    ///
    /// This is a synchronous operation since `watch::Sender::send` is non-blocking.
    pub fn shutdown(&self) {
        info!("Initiating graceful shutdown...");
        if self.sender.send(true).is_ok() {
            debug!("Shutdown signal sent to all subscribers");
        } else {
            trace!("No active subscribers for shutdown signal (all tasks already completed)");
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

    #[tokio::test]
    async fn test_basic_shutdown() {
        let shutdown = ShutdownSignal::new();
        let rx = shutdown.subscribe();

        // Initial state should be false
        assert!(!*rx.borrow());

        // Trigger shutdown
        shutdown.shutdown();

        // Now should be true
        assert!(*rx.borrow());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let shutdown = ShutdownSignal::new();
        let rx1 = shutdown.subscribe();
        let rx2 = shutdown.subscribe();

        shutdown.shutdown();

        // Both receivers should receive signal
        assert!(*rx1.borrow());
        assert!(*rx2.borrow());
    }
}
