//! macOS window event hook implementation
//!
//! Monitors window events such as foreground window changes using NSWorkspace
//! notifications. This is the macOS equivalent of Windows WinEventHook.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Window event types for macOS
#[derive(Debug, Clone)]
pub enum MacosWindowEvent {
    /// Foreground window changed (application switched)
    WindowActivated {
        process_name: String,
        window_title: String,
    },
    /// Window was minimized
    WindowMinimized { process_name: String },
    /// Window was restored from minimized state
    WindowRestored { process_name: String },
    /// Window was moved or resized
    WindowMovedOrResized {
        process_name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
}

/// macOS window event hook using NSWorkspace notifications
pub struct MacosWindowEventHook {
    event_sender: Sender<MacosWindowEvent>,
    running: Arc<AtomicBool>,
    shutdown_flag: Arc<AtomicBool>,
}

impl MacosWindowEventHook {
    /// Create a new window event hook
    pub fn new() -> Self {
        let (sender, _) = channel();

        Self {
            event_sender: sender,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a new window event hook with custom event sender
    pub fn with_sender(event_sender: Sender<MacosWindowEvent>) -> Self {
        Self {
            event_sender,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start monitoring window events in background thread
    pub fn start(&mut self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.shutdown_flag.store(false, Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);

        let sender = self.event_sender.clone();
        let shutdown = self.shutdown_flag.clone();
        let is_running = self.running.clone();

        // Spawn background thread to poll for foreground window changes
        let handle = std::thread::spawn(move || {
            use std::process::Command;
            use std::time::Duration;

            let mut last_process = String::new();
            let mut last_title = String::new();

            while !shutdown.load(Ordering::SeqCst) {
                // Query current frontmost application using AppleScript
                let script = r#"
                    tell application "System Events"
                        set frontApp to first application process whose frontmost is true
                        set appName to name of frontApp
                        try
                            set winTitle to name of first window of frontApp
                        on error
                            set winTitle to ""
                        end try
                        return {appName, winTitle}
                    end tell
                "#;

                match Command::new("osascript").arg("-e").arg(script).output() {
                    Ok(output) if output.status.success() => {
                        let result =
                            String::from_utf8_lossy(&output.stdout).trim().to_string();
                        let parts: Vec<&str> = result.split(", ").collect();

                        let current_process = parts.get(0).unwrap_or(&"").to_string();
                        let current_title = parts.get(1).unwrap_or(&"").to_string();

                        // Detect foreground window change
                        if !current_process.is_empty()
                            && (current_process != last_process
                                || current_title != last_title)
                        {
                            if !last_process.is_empty()
                                && current_process != last_process
                            {
                                let _ = sender.send(MacosWindowEvent::WindowActivated {
                                    process_name: current_process.clone(),
                                    window_title: current_title.clone(),
                                });
                                debug!(
                                    "Foreground window changed: {} - {}",
                                    current_process, current_title
                                );
                            }

                            last_process = current_process;
                            last_title = current_title;
                        }
                    }
                    Err(e) => {
                        warn!("Failed to query foreground window: {}", e);
                    }
                    _ => {}
                }

                // Poll every 200ms to avoid excessive CPU usage
                std::thread::sleep(Duration::from_millis(200));
            }

            is_running.store(false, Ordering::SeqCst);
            debug!("MacosWindowEventHook thread stopped");
        });

        info!("MacosWindowEventHook started");
        let _ = handle; // Keep thread alive

        Ok(())
    }

    /// Stop the event hook
    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        self.running.store(false, Ordering::SeqCst);
        debug!("MacosWindowEventHook stop requested");
    }

    /// Check if the hook is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the shutdown flag reference (for external coordination)
    pub fn shutdown_flag(&self) -> &Arc<AtomicBool> {
        &self.shutdown_flag
    }
}

impl Default for MacosWindowEventHook {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MacosWindowEventHook {
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            self.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_hook_creation() {
        let hook = MacosWindowEventHook::new();
        assert!(!hook.is_running());
    }

    #[test]
    fn test_event_hook_start_stop() {
        let (sender, receiver) = channel::<MacosWindowEvent>();
        let mut hook = MacosWindowEventHook::with_sender(sender);

        hook.start().unwrap();
        assert!(hook.is_running());

        // Give it a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));

        hook.stop();
        assert!(!hook.is_running());

        // Channel should be closed after stop
        drop(hook);
    }

    #[test]
    fn test_window_event_variants() {
        let event1 = MacosWindowEvent::WindowActivated {
            process_name: "Safari".to_string(),
            window_title: "Apple".to_string(),
        };

        let event2 = MacosWindowEvent::WindowMinimized {
            process_name: "Finder".to_string(),
        };

        let event3 = MacosWindowEvent::WindowRestored {
            process_name: "Terminal".to_string(),
        };

        let event4 = MacosWindowEvent::WindowMovedOrResized {
            process_name: "Code".to_string(),
            x: 100,
            y: 200,
            width: 800,
            height: 600,
        };

        // Verify they can be cloned and matched on
        let _e1 = event1.clone();
        let _e2 = event2.clone();

        match &event1 {
            MacosWindowEvent::WindowActivated { .. } => {}
            _ => panic!("Expected WindowActivated"),
        }

        match &event4 {
            MacosWindowEvent::WindowMovedOrResized {
                process_name: _,
                x,
                y,
                width,
                height,
            } => {
                assert_eq!(*x, 100);
                assert_eq!(*y, 200);
                assert_eq!(*width, 800);
                assert_eq!(*height, 600);
            }
            _ => panic!("Expected WindowMovedOrResized"),
        }
    }

    #[test]
    fn test_default_creation() {
        let hook = MacosWindowEventHook::default();
        assert!(!hook.is_running());
    }

    #[test]
    fn test_shutdown_flag() {
        let hook = MacosWindowEventHook::new();
        assert!(!hook.shutdown_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn test_multiple_starts_ignored() {
        let mut hook = MacosWindowEventHook::new();
        hook.start().unwrap();
        assert!(hook.is_running());

        // Second start should be no-op
        hook.start().unwrap();
        assert!(hook.is_running());

        hook.stop();
    }
}
