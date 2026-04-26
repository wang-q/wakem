//! macOS window event hook implementation
//!
//! Monitors window events such as foreground window changes using Core Graphics
//! and Accessibility APIs. This is the macOS equivalent of Windows WinEventHook.
//!
//! Performance: < 2ms per poll (vs 100-200ms with AppleScript)

use crate::platform::traits::PlatformWindowEvent;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tracing::{debug, info, trace};

/// Window event hook manager
pub struct WindowEventHook {
    event_sender: Sender<PlatformWindowEvent>,
    running: Arc<AtomicBool>,
    shutdown_flag: Arc<AtomicBool>,
    poll_interval_ms: u64,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl WindowEventHook {
    /// Create new window event hook
    pub fn new(event_sender: Sender<PlatformWindowEvent>) -> Self {
        Self {
            event_sender,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            poll_interval_ms: 200,
            thread_handle: None,
        }
    }

    /// Start window event monitoring
    pub fn start(&mut self) -> Result<()> {
        self.start_with_shutdown(self.shutdown_flag.clone())
    }

    /// Start window event monitoring with shutdown flag for graceful exit
    pub fn start_with_shutdown(&mut self, shutdown_flag: Arc<AtomicBool>) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.shutdown_flag = shutdown_flag;
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
            let mut last_window_count: usize = 0;
            let mut initialized = false;

            while !shutdown.load(Ordering::SeqCst) {
                match get_frontmost_app_info() {
                    Ok((current_process, current_title, current_window_count)) => {
                        if !current_process.is_empty() {
                            if !initialized {
                                last_process = current_process;
                                last_title = current_title;
                                last_window_count = current_window_count;
                                initialized = true;
                            } else if current_process != last_process
                                || current_title != last_title
                            {
                                let _ =
                                    sender.send(PlatformWindowEvent::WindowActivated {
                                        process_name: current_process.clone(),
                                        window_title: current_title.clone(),
                                        window_id: 0,
                                    });
                                debug!(
                                    "Foreground window changed: {} - {}",
                                    current_process, current_title
                                );

                                last_process = current_process;
                                last_title = current_title;
                            }

                            if current_window_count != last_window_count && initialized {
                                if current_window_count > last_window_count {
                                    let _ = sender.send(
                                        PlatformWindowEvent::WindowCreated {
                                            process_name: last_process.clone(),
                                            window_title: last_title.clone(),
                                        },
                                    );
                                    debug!("Window created in {}", last_process);
                                } else if current_window_count < last_window_count {
                                    let _ =
                                        sender.send(PlatformWindowEvent::WindowClosed {
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
            debug!("WindowEventHook thread stopped");
        });

        self.thread_handle = Some(handle);
        info!("WindowEventHook started (using native APIs)");

        Ok(())
    }

    /// Stop window event monitoring
    pub fn stop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
            debug!("WindowEventHook thread joined");
        }

        self.running.store(false, Ordering::SeqCst);
        debug!("WindowEventHook stopped");
    }

    /// Get shutdown flag reference
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }
}

impl Drop for WindowEventHook {
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            self.stop();
        }
    }
}

fn get_frontmost_app_info() -> Result<(String, String, usize)> {
    use crate::platform::macos::native_api::cg_window::get_on_screen_windows;

    let windows = get_on_screen_windows()
        .map_err(|e| anyhow::anyhow!("Failed to get window list: {}", e))?;

    let frontmost = windows
        .iter()
        .rfind(|w| w.layer == 0 && !w.owner_name.is_empty());

    if let Some(window) = frontmost {
        let process_name = window.owner_name.clone();
        let window_title = window.name.clone();

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_window_event_creation() {
        let (tx, _rx) = channel::<PlatformWindowEvent>();
        let hook = WindowEventHook::new(tx);
        drop(hook);
    }

    #[test]
    fn test_event_hook_start_stop() {
        let (sender, receiver) = channel::<PlatformWindowEvent>();
        let mut hook = WindowEventHook::new(sender);

        hook.start().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(100));

        hook.stop();

        drop(hook);
        drop(receiver);
    }

    #[test]
    fn test_shutdown_flag() {
        let (sender, _receiver) = channel::<PlatformWindowEvent>();
        let hook = WindowEventHook::new(sender);
        assert!(!hook.shutdown_flag().load(Ordering::SeqCst));
    }

    #[test]
    fn test_get_frontmost_app_info() {
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
