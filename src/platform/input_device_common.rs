//! Common input device base struct and utilities
//!
//! This module provides a shared [InputDeviceBase] struct that encapsulates
//! common state (modifier tracking, event channel, running flag) used by
//! all platform input device implementations.
//!
//! Also provides [InputDevice] generic struct for platform-specific implementations.

use crate::platform::traits::InputDeviceConfig;
use crate::types::{InputEvent, KeyState, ModifierState};
use anyhow::Result;
use std::sync::mpsc::{channel, Receiver, Sender};

#[allow(dead_code)]
impl InputDeviceConfig {
    pub fn keyboard_only() -> Self {
        Self {
            capture_keyboard: true,
            capture_mouse: false,
            block_legacy_input: true,
        }
    }

    pub fn mouse_only() -> Self {
        Self {
            capture_keyboard: false,
            capture_mouse: true,
            block_legacy_input: true,
        }
    }
}

/// Shared base state for input device implementations
///
/// Encapsulates the common fields and logic shared across all platform
/// input devices: modifier state tracking, event channel, and running flag.
#[allow(dead_code)]
pub struct InputDeviceBase {
    pub modifier_state: ModifierState,
    pub running: bool,
    pub event_receiver: Receiver<InputEvent>,
    pub event_sender: Sender<InputEvent>,
}

#[allow(dead_code)]
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

    pub fn with_channel(
        event_sender: Sender<InputEvent>,
        event_receiver: Receiver<InputEvent>,
    ) -> Self {
        Self {
            modifier_state: ModifierState::default(),
            running: false,
            event_receiver,
            event_sender,
        }
    }

    pub fn update_modifier_state(&mut self, virtual_key: u16, pressed: bool) {
        self.modifier_state
            .apply_from_virtual_key(virtual_key, pressed);
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
#[allow(dead_code)]
pub struct InputDevice<T> {
    pub base: InputDeviceBase,
    pub inner: Option<T>,
}

#[allow(dead_code)]
impl<T> InputDevice<T> {
    /// Create a new input device with default config
    pub fn new(_config: InputDeviceConfig) -> Result<Self> {
        let base = InputDeviceBase::new();
        Ok(Self {
            base,
            inner: None,
        })
    }

    /// Create an input device with custom sender
    pub fn with_sender(event_sender: Sender<InputEvent>) -> Result<Self> {
        let base = InputDeviceBase::with_sender(event_sender);
        Ok(Self {
            base,
            inner: None,
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

/// Trait for platform-specific input device operations.
///
/// Implement this trait for platform-specific inner device types
/// to enable the generic [InputDevice] to work with them.
#[allow(dead_code)]
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
    fn test_keyboard_only_config() {
        let config = InputDeviceConfig::keyboard_only();
        assert!(config.capture_keyboard);
        assert!(!config.capture_mouse);
        assert!(config.block_legacy_input);
    }

    #[test]
    fn test_mouse_only_config() {
        let config = InputDeviceConfig::mouse_only();
        assert!(!config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(config.block_legacy_input);
    }
}
