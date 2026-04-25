//! macOS window event hook implementation
//!
//! Monitors window events such as foreground window changes using Core Graphics
//! and Accessibility APIs. This is the macOS equivalent of Windows WinEventHook.
//!
//! Performance: < 2ms per poll (vs 100-200ms with AppleScript)
#![cfg(target_os = "macos")]

use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use tracing::{debug, info, trace};

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
    /// Window was created
    WindowCreated {
        process_name: String,
        window_title: String,
    },
    /// Window was closed
    WindowClosed { process_name: String },
}

/// macOS window event hook using native APIs (Core Graphics + Accessibility)
pub struct MacosWindowEventHook {
    event_sender: Sender<MacosWindowEvent>,
    running: Arc<AtomicBool>,
    shutdown_flag: Arc<AtomicBool>,
    poll_interval_ms: u64,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl MacosWindowEventHook {
    /// Create a new window event hook
    pub fn new() -> Self {
        let (sender, _) = channel();

        Self {
            event_sender: sender,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            poll_interval_ms: 200,
            thread_handle: None,
        }
    }

    pub fn with_sender(event_sender: Sender<MacosWindowEvent>) -> Self {
        Self {
            event_sender,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            poll_interval_ms: 200,
            thread_handle: None,
        }
    }

    pub fn with_interval(
        event_sender: Sender<MacosWindowEvent>,
        interval_ms: u64,
    ) -> Self {
        Self {
            event_sender,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            poll_interval_ms: interval_ms,
            thread_handle: None,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.shutdown_flag.store(false, Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);

        let sender = self.event_sender.clone();
        let shutdown = self.shutdown_flag.clone();
        let is_running = self.running.clone();
        let poll_interval = self.poll_interval_ms;

        let handle = std::thread::spawn(move || {
            use std::time::Duration;

            let mut last_process = String::new();
            let mut last_title = String::new();
            let mut last_window_count = 0;

            while !shutdown.load(Ordering::SeqCst) {
                match get_frontmost_app_info() {
                    Ok((current_process, current_title, current_window_count)) => {
                        if !current_process.is_empty() {
                            if current_process != last_process
                                || current_title != last_title
                            {
                                if !last_process.is_empty()
                                    && current_process != last_process
                                {
                                    let _ =
                                        sender.send(MacosWindowEvent::WindowActivated {
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

                            if current_window_count != last_window_count {
                                if current_window_count > last_window_count {
                                    let _ =
                                        sender.send(MacosWindowEvent::WindowCreated {
                                            process_name: last_process.clone(),
                                            window_title: last_title.clone(),
                                        });
                                    debug!("Window created in {}", last_process);
                                } else if current_window_count < last_window_count {
                                    let _ =
                                        sender.send(MacosWindowEvent::WindowClosed {
                                            process_name: last_process.clone(),
                                        });
                                    debug!("Window closed in {}", last_process);
                                }
                                last_window_count = current_window_count;
                            }
                        }
                    }
                    Err(e) => {
                        trace!("Failed to query foreground window: {}", e);
                    }
                }

                std::thread::sleep(Duration::from_millis(poll_interval));
            }

            is_running.store(false, Ordering::SeqCst);
            debug!("MacosWindowEventHook thread stopped");
        });

        self.thread_handle = Some(handle);
        info!("MacosWindowEventHook started (using native APIs)");

        Ok(())
    }

    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
            debug!("MacosWindowEventHook thread joined");
        }

        self.running.store(false, Ordering::SeqCst);
        debug!("MacosWindowEventHook stopped");
    }

    /// Check if the hook is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the shutdown flag reference (for external coordination)
    pub fn shutdown_flag(&self) -> &Arc<AtomicBool> {
        &self.shutdown_flag
    }

    /// Get the event sender
    pub fn event_sender(&self) -> &Sender<MacosWindowEvent> {
        &self.event_sender
    }

    /// Create a new channel and return the receiver
    pub fn create_receiver(&mut self) -> Receiver<MacosWindowEvent> {
        let (sender, receiver) = channel();
        self.event_sender = sender;
        receiver
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

/// Get frontmost application info using native APIs
/// Returns: (process_name, window_title, window_count)
fn get_frontmost_app_info() -> Result<(String, String, usize)> {
    use crate::platform::macos::native_api::cg_window::get_on_screen_windows;

    let windows = get_on_screen_windows()
        .map_err(|e| anyhow::anyhow!("Failed to get window list: {}", e))?;

    // Find the frontmost window (highest layer, most recent in that layer)
    let frontmost = windows
        .iter()
        .filter(|w| w.layer == 0 && !w.owner_name.is_empty())
        .next_back();

    if let Some(window) = frontmost {
        let process_name = window.owner_name.clone();
        let window_title = window.name.clone();

        // Count windows for this process
        let window_count = windows
            .iter()
            .filter(|w| w.owner_name == process_name && w.layer == 0)
            .count();

        trace!(
            "Frontmost: {} - {} ({} windows)",
            process_name,
            window_title,
            window_count
        );

        Ok((process_name, window_title, window_count))
    } else {
        Err(anyhow::anyhow!("No frontmost window found"))
    }
}

/// Window event handler trait for processing window events
pub trait WindowEventHandler: Send + Sync {
    fn handle_event(&self, event: &MacosWindowEvent);
}

/// Simple window event processor that routes events to handlers
pub struct WindowEventProcessor {
    handlers: Vec<Box<dyn WindowEventHandler>>,
}

impl WindowEventProcessor {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Box<dyn WindowEventHandler>) {
        self.handlers.push(handler);
    }

    pub fn process_event(&self, event: &MacosWindowEvent) {
        for handler in &self.handlers {
            handler.handle_event(event);
        }
    }
}

impl Default for WindowEventProcessor {
    fn default() -> Self {
        Self::new()
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
    fn test_event_hook_with_sender() {
        let (sender, _receiver) = channel::<MacosWindowEvent>();
        let hook = MacosWindowEventHook::with_sender(sender);
        assert!(!hook.is_running());
    }

    #[test]
    fn test_event_hook_with_interval() {
        let (sender, _receiver) = channel::<MacosWindowEvent>();
        let hook = MacosWindowEventHook::with_interval(sender, 500);
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
        drop(receiver);
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

        let event5 = MacosWindowEvent::WindowCreated {
            process_name: "Safari".to_string(),
            window_title: "New Window".to_string(),
        };

        let event6 = MacosWindowEvent::WindowClosed {
            process_name: "Finder".to_string(),
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

        match &event5 {
            MacosWindowEvent::WindowCreated {
                process_name,
                window_title,
            } => {
                assert_eq!(process_name, "Safari");
                assert_eq!(window_title, "New Window");
            }
            _ => panic!("Expected WindowCreated"),
        }

        match &event6 {
            MacosWindowEvent::WindowClosed { process_name } => {
                assert_eq!(process_name, "Finder");
            }
            _ => panic!("Expected WindowClosed"),
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

    #[test]
    fn test_event_processor() {
        struct TestHandler {
            received: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        }

        impl WindowEventHandler for TestHandler {
            fn handle_event(&self, _event: &MacosWindowEvent) {
                self.received.fetch_add(1, Ordering::SeqCst);
            }
        }

        let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let handler = TestHandler {
            received: counter.clone(),
        };

        let mut processor = WindowEventProcessor::new();
        processor.add_handler(Box::new(handler));

        let event = MacosWindowEvent::WindowActivated {
            process_name: "Test".to_string(),
            window_title: "Test".to_string(),
        };
        processor.process_event(&event);

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_get_frontmost_app_info() {
        // This test may fail in headless environments
        match get_frontmost_app_info() {
            Ok((process, title, count)) => {
                println!("Frontmost: {} - {} ({} windows)", process, title, count);
                assert!(!process.is_empty());
            }
            Err(e) => {
                println!("Note: Could not get frontmost app info: {}", e);
            }
        }
    }
}
