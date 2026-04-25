//! Common input device factory, base struct, and utilities
//!
//! This module provides platform-agnostic input device factory implementations
//! and a shared [InputDeviceBase] struct that encapsulates common state
//! (modifier tracking, event channel, running flag) used by all platform
//! input device implementations.

use crate::platform::traits::InputDeviceConfig;
use crate::types::{InputEvent, KeyState, ModifierState};
use std::sync::mpsc::{channel, Receiver, Sender};

/// Input device factory for creating configured input devices
///
/// This is a platform-agnostic factory that works with any platform's
/// input device implementation.
pub struct InputDeviceFactory;

impl InputDeviceFactory {
    /// Create default input device configuration
    pub fn default_config() -> InputDeviceConfig {
        InputDeviceConfig::default()
    }

    /// Create keyboard-only input device configuration
    pub fn keyboard_only_config() -> InputDeviceConfig {
        InputDeviceConfig {
            capture_keyboard: true,
            capture_mouse: false,
            block_legacy_input: true,
        }
    }

    /// Create mouse-only input device configuration
    pub fn mouse_only_config() -> InputDeviceConfig {
        InputDeviceConfig {
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
pub struct InputDeviceBase {
    pub modifier_state: ModifierState,
    pub running: bool,
    pub event_receiver: Receiver<InputEvent>,
    pub event_sender: Sender<InputEvent>,
}

impl InputDeviceBase {
    /// Create a new base with a fresh event channel
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            modifier_state: ModifierState::default(),
            running: false,
            event_receiver: receiver,
            event_sender: sender,
        }
    }

    /// Create a base with a pre-existing sender.
    ///
    /// Note: The internal `event_receiver` is disconnected (its sender is
    /// immediately dropped), so `try_recv_event()` will always return `None`.
    /// Use this when the caller manages their own receiver and reads events
    /// directly from the channel paired with `event_sender`.
    pub fn with_sender(event_sender: Sender<InputEvent>) -> Self {
        Self {
            modifier_state: ModifierState::default(),
            running: false,
            event_receiver: channel().1,
            event_sender,
        }
    }

    /// Create a base with a matched sender/receiver pair.
    ///
    /// Both `try_recv_event()` and the external receiver will receive
    /// events sent through `event_sender`.
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

    /// Update modifier key state based on a virtual key event
    pub fn update_modifier_state(&mut self, virtual_key: u16, pressed: bool) {
        self.modifier_state
            .apply_from_virtual_key(virtual_key, pressed);
    }

    /// Try to receive an event from the channel (non-blocking)
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

    /// Get a clone of the event sender
    pub fn sender(&self) -> Sender<InputEvent> {
        self.event_sender.clone()
    }

    /// Check if the device is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Stop the device
    pub fn stop(&mut self) {
        self.running = false;
    }
}

impl Default for InputDeviceBase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = InputDeviceFactory::default_config();
        assert!(config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(!config.block_legacy_input);
    }

    #[test]
    fn test_keyboard_only_config() {
        let config = InputDeviceFactory::keyboard_only_config();
        assert!(config.capture_keyboard);
        assert!(!config.capture_mouse);
        assert!(config.block_legacy_input);
    }

    #[test]
    fn test_mouse_only_config() {
        let config = InputDeviceFactory::mouse_only_config();
        assert!(!config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(config.block_legacy_input);
    }
}
