//! Common input device base struct and utilities
//!
//! This module provides a shared [InputDeviceBase] struct that encapsulates
//! common state (modifier tracking, event channel, running flag) used by
//! all platform input device implementations.

use crate::platform::traits::InputDeviceConfig;
use crate::types::{InputEvent, KeyState, ModifierState};
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
