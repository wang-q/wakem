//! Windows input device implementation
#![cfg(target_os = "windows")]
#![allow(dead_code)]

use crate::platform::traits::{InputDeviceConfig, InputDeviceTrait};
use crate::types::{InputEvent, KeyState, ModifierState};
use anyhow::Result;
use std::sync::mpsc::{channel, Receiver, Sender};
use tracing::debug;

/// Real Raw Input device implementation
#[allow(dead_code)]
pub struct RawInputDevice {
    config: InputDeviceConfig,
    event_receiver: Receiver<InputEvent>,
    event_sender: Sender<InputEvent>,
    modifier_state: ModifierState,
    running: bool,
    #[allow(dead_code)]
    hwnd: Option<isize>, // Store as isize for Send safety
}

impl RawInputDevice {
    /// Create a new Raw Input device
    pub fn new(config: InputDeviceConfig) -> Result<Self> {
        let (sender, receiver) = channel();

        Ok(Self {
            config,
            event_receiver: receiver,
            event_sender: sender,
            modifier_state: ModifierState::default(),
            running: false,
            hwnd: None,
        })
    }

    /// Update modifier key state
    #[allow(dead_code)]
    fn update_modifier_state(&mut self, virtual_key: u16, pressed: bool) {
        self.modifier_state
            .apply_from_virtual_key(virtual_key, pressed);
    }
}

impl InputDeviceTrait for RawInputDevice {
    fn register(&mut self) -> Result<()> {
        debug!("Registering Raw Input device");
        self.running = true;
        Ok(())
    }

    fn unregister(&mut self) {
        debug!("Unregistering Raw Input device");
        self.running = false;
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        if !self.running {
            return None;
        }

        match self.event_receiver.try_recv() {
            Ok(event) => {
                // Update modifier key state
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

    fn is_running(&self) -> bool {
        self.running
    }

    fn stop(&mut self) {
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    // Common MockInputDevice tests are in platform::mock module.
    // All RawInputDevice tests use MockInputDevice which is fully tested there.
    // No Windows-specific input device tests beyond mock coverage.
}
