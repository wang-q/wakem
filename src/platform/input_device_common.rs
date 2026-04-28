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

/// Shared base state for input device implementations
///
/// Encapsulates the common fields and logic shared across all platform
/// input devices: modifier state tracking, event channel, and running flag.
pub struct InputDeviceBase {
    pub modifier_state: ModifierState,
    pub running: bool,
    pub event_receiver: Receiver<InputEvent>,
    pub event_sender: Sender<InputEvent>,
    /// Platform-specific input configuration (keyboard/mouse capture, legacy input blocking).
    /// Used by platform device implementations during registration.
    pub config: InputDeviceConfig,
}

impl InputDeviceBase {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            modifier_state: ModifierState::default(),
            running: false,
            event_receiver: receiver,
            event_sender: sender,
            config: InputDeviceConfig::default(),
        }
    }

    pub fn with_sender(event_sender: Sender<InputEvent>) -> Self {
        // Create a dummy channel for the receiver - the sender will be dropped
        // but we need a receiver to satisfy the type system
        let (_dummy_sender, receiver) = channel();
        // Explicitly drop the dummy sender to avoid resource waste
        drop(_dummy_sender);

        Self {
            modifier_state: ModifierState::default(),
            running: false,
            event_receiver: receiver,
            event_sender,
            config: InputDeviceConfig::default(),
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
pub struct InputDevice<T> {
    pub base: InputDeviceBase,
    pub inner: Option<T>,
}

impl<T> InputDevice<T> {
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        let mut base = InputDeviceBase::new();
        base.config = config;
        Ok(Self { base, inner: None })
    }

    pub fn with_sender(event_sender: Sender<InputEvent>) -> Result<Self> {
        let mut base = InputDeviceBase::with_sender(event_sender);
        base.config = InputDeviceConfig::default();
        Ok(Self { base, inner: None })
    }
}

/// Trait for platform-specific input device operations.
///
/// Implement this trait for platform-specific inner device types
/// to enable the generic [InputDevice] to work with them.
pub trait PlatformInputDevice: Sized + Send {
    /// Create the platform-specific device with a sender
    fn create(sender: Sender<InputEvent>) -> Result<Self>;

    /// Register/start the device
    /// This is called when the input device is registered
    fn register(&mut self) -> Result<()>;

    /// Run one iteration of the input processing loop
    /// Returns Ok(true) if should continue, Ok(false) if shutdown requested
    fn run_once(&mut self) -> Result<bool>;

    /// Stop the device
    fn stop(&mut self);
}

impl<T: PlatformInputDevice> InputDevice<T> {
    /// Register the input device
    /// This creates the inner device and starts it
    pub fn register_inner(&mut self) -> Result<()> {
        let sender = self.base.sender();
        let mut inner = T::create(sender)?;
        inner.register()?;
        self.inner = Some(inner);
        self.base.running = true;
        Ok(())
    }

    pub fn poll_event_inner(&mut self) -> Option<InputEvent> {
        if !self.base.running {
            return None;
        }

        if let Some(ref mut inner) = self.inner {
            let _ = inner.run_once();
        }

        self.base.try_recv_event()
    }

    pub fn unregister_inner(&mut self) {
        self.stop_inner();
    }

    pub fn stop_inner(&mut self) {
        self.base.stop();
        if let Some(mut inner) = self.inner.take() {
            inner.stop();
        }
    }
}

/// Macro implementing the three trivially-identical `InputDeviceTrait` methods
/// shared by all platform input devices: `poll_event`, `is_running`, `stop`.
#[macro_export]
macro_rules! impl_input_device_trait_common {
    () => {
        fn poll_event(&mut self) -> Option<$crate::types::InputEvent> {
            self.poll_event_inner()
        }

        fn is_running(&self) -> bool {
            self.base.is_running()
        }

        fn stop(&mut self) {
            self.stop_inner();
        }
    };
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
