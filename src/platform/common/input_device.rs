//! Common input device base struct and utilities
//!
//! This module provides a shared [InputDeviceBase] struct that encapsulates
//! common state (modifier tracking, event channel, running flag) used by
//! all platform input device implementations.
//!
//! Also provides [InputDevice] generic struct for platform-specific implementations.

use crate::platform::types::InputDeviceConfig;
use crate::types::{InputEvent, KeyState, ModifierState};
use anyhow::Result;
use std::sync::mpsc::{channel, Receiver, Sender};
use tracing::debug;

/// Shared base state for input device implementations
///
/// Encapsulates the common fields and logic shared across all platform
/// input devices: modifier state tracking, event channel, and running flag.
pub struct InputDeviceBase {
    pub modifier_state: ModifierState,
    pub running: bool,
    pub event_receiver: Receiver<InputEvent>,
    pub event_sender: Sender<InputEvent>,
}

impl InputDeviceBase {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            modifier_state: ModifierState::default(),
            running: false,
            event_receiver: receiver,
            event_sender: sender,
        }
    }

    pub fn with_sender(event_sender: Sender<InputEvent>) -> Self {
        Self {
            modifier_state: ModifierState::default(),
            running: false,
            event_receiver: channel().1,
            event_sender,
        }
    }

    pub fn update_modifier_state(&mut self, virtual_key: u16, pressed: bool) {
        self.modifier_state
            .apply_from_internal_vk(virtual_key, pressed);
    }

    pub fn try_recv_event(&mut self) -> Option<InputEvent> {
        match self.event_receiver.try_recv() {
            Ok(event) => {
                if let InputEvent::Key(key_event) = &event {
                    self.update_modifier_state(
                        key_event.virtual_key,
                        key_event.state == KeyState::Pressed,
                    );
                }
                Some(event)
            }
            Err(_) => None,
        }
    }

    pub fn sender(&self) -> Sender<InputEvent> {
        self.event_sender.clone()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn stop(&mut self) {
        self.running = false;
    }
}

impl Default for InputDeviceBase {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic input device wrapper for platform-specific implementations.
///
/// This struct combines [InputDeviceBase] with a platform-specific inner device
/// to provide a unified input device interface.
pub struct InputDevice<T> {
    pub base: InputDeviceBase,
    pub inner: Option<T>,
    pub config: InputDeviceConfig,
}

impl<T> InputDevice<T> {
    /// Create a new input device with default config
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        let base = InputDeviceBase::new();
        Ok(Self {
            base,
            inner: None,
            config,
        })
    }

    /// Create an input device with custom sender
    pub fn with_sender(event_sender: Sender<InputEvent>) -> Result<Self> {
        let base = InputDeviceBase::with_sender(event_sender);
        Ok(Self {
            base,
            inner: None,
            config: InputDeviceConfig::default(),
        })
    }

    /// Get the event sender
    pub fn get_sender(&self) -> Sender<InputEvent> {
        self.base.sender()
    }

    /// Check if the device is running
    pub fn is_running(&self) -> bool {
        self.base.is_running()
    }

    /// Stop the device
    pub fn stop(&mut self) {
        self.base.stop();
        self.inner = None;
    }
}

impl<T: PlatformInputDevice> InputDevice<T> {
    /// Default unregister implementation shared by all platforms
    pub fn default_unregister(&mut self, device_name: &str) {
        debug!("Unregistering {}", device_name);
        self.base.running = false;
        if let Some(ref mut inner) = self.inner.take() {
            inner.stop();
        }
    }

    /// Default run_once implementation shared by all platforms
    pub fn default_run_once(&mut self) -> Result<bool> {
        if let Some(ref mut inner) = self.inner {
            inner.run_once()
        } else {
            Ok(true)
        }
    }
}

/// Trait for platform-specific input device operations.
///
/// Implement this trait for platform-specific inner device types
/// to enable the generic [InputDevice] to work with them.
pub trait PlatformInputDevice: Sized + Send {
    /// Create the platform-specific device with a sender
    fn create(sender: Sender<InputEvent>) -> Result<Self>;

    /// Run one iteration of the input processing loop
    /// Returns Ok(true) if should continue, Ok(false) if shutdown requested
    fn run_once(&mut self) -> Result<bool>;

    /// Stop the device
    fn stop(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_device_base_creation() {
        let base = InputDeviceBase::new();
        assert!(!base.is_running());
    }

    #[test]
    fn test_input_device_base_with_sender() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let base = InputDeviceBase::with_sender(tx);
        assert!(!base.is_running());
    }
}
