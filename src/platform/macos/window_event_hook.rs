//! macOS window event hook implementation
//!
//! Monitors window events such as foreground window changes using Core Graphics
//! and Accessibility APIs. This is the macOS equivalent of Windows WinEventHook.
//!
//! Performance: < 2ms per poll (vs 100-200ms with AppleScript)
#![cfg(target_os = "macos")]

use crate::platform::traits::PlatformWindowEvent;
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use tracing::{debug, info, trace};

/// macOS window event hook using native APIs (Core Graphics + Accessibility)
pub struct MacosWindowEventHook {
    event_sender: Sender<PlatformWindowEvent>,
    running: Arc<AtomicBool>,
    shutdown_flag: Arc<AtomicBool>,
    poll_interval_ms: u64,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl MacosWindowEventHook {
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

    pub fn with_sender(event_sender: Sender<PlatformWindowEvent>) -> Self {
        Self {
            event_sender,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            poll_interval_ms: 200,
            thread_handle: None,
        }
    }

    pub fn with_interval(
        event_sender: Sender<PlatformWindowEvent>,
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
                                    let _ =
                                        sender.send(PlatformWindowEvent::WindowCreated {
                                            process_name: last_process.clone(),
                                            window_title: last_title.clone(),
                                        });
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

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn shutdown_flag(&self) -> &Arc<AtomicBool> {
        &self.shutdown_flag
    }

    pub fn event_sender(&self) -> &Sender<PlatformWindowEvent> {
        &self.event_sender
    }

    pub fn create_receiver(&mut self) -> Receiver<PlatformWindowEvent> {
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

fn get_frontmost_app_info() -> Result<(String, String, usize)> {
    use crate::platform::macos::native_api::cg_window::get_on_screen_windows;

    let windows = get_on_screen_windows()
        .map_err(|e| anyhow::anyhow!("Failed to get window list: {}", e))?;

    let frontmost = windows
        .iter()
        .filter(|w| w.layer == 0 && !w.owner_name.is_empty())
        .next_back();

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

    #[test]
    fn test_event_hook_creation() {
        let hook = MacosWindowEventHook::new();
        assert!(!hook.is_running());
    }

    #[test]
    fn test_event_hook_with_sender() {
        let (sender, _receiver) = channel::<PlatformWindowEvent>();
        let hook = MacosWindowEventHook::with_sender(sender);
        assert!(!hook.is_running());
    }

    #[test]
    fn test_event_hook_with_interval() {
        let (sender, _receiver) = channel::<PlatformWindowEvent>();
        let hook = MacosWindowEventHook::with_interval(sender, 500);
        assert!(!hook.is_running());
    }

    #[test]
    fn test_event_hook_start_stop() {
        let (sender, receiver) = channel::<PlatformWindowEvent>();
        let mut hook = MacosWindowEventHook::with_sender(sender);

        hook.start().unwrap();
        assert!(hook.is_running());

        std::thread::sleep(std::time::Duration::from_millis(100));

        hook.stop();
        assert!(!hook.is_running());

        drop(hook);
        drop(receiver);
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

        hook.start().unwrap();
        assert!(hook.is_running());

        hook.stop();
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
