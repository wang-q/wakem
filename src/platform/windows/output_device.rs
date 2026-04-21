//! Windows output device implementation
#![cfg(target_os = "windows")]

use crate::constants::WHEEL_DELTA;
use crate::types::{KeyAction, MouseAction, MouseButton, SystemAction};
use anyhow::Result;
#[allow(unused_imports)]
use std::cell::RefCell;
#[allow(unused_imports)]
use tracing::trace;

/// Output event types
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum OutputEvent {
    /// Key event
    Key {
        scan_code: u16,
        virtual_key: u16,
        release: bool,
    },
    /// Mouse move
    MouseMove { x: i32, y: i32, relative: bool },
    /// Mouse button
    MouseButton { button: MouseButton, release: bool },
    /// Mouse wheel
    MouseWheel { delta: i32, horizontal: bool },
}

/// Output device abstract interface
#[allow(dead_code)]
pub trait OutputDevice {
    /// Send key action
    fn send_key_action(&self, action: &KeyAction) -> Result<()>;
    /// Send single key
    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()>;
    /// Send mouse action
    fn send_mouse_action(&self, action: &MouseAction) -> Result<()>;
    /// Send mouse move
    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()>;
    /// Send mouse button
    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()>;
    /// Send mouse wheel
    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()>;
    /// Send system action
    fn send_system_action(&self, action: &SystemAction) -> Result<()>;
}

/// Real SendInput implementation
#[allow(dead_code)]
pub struct SendInputDevice;

#[allow(dead_code)]
impl SendInputDevice {
    /// Create new SendInput device
    pub fn new() -> Self {
        Self
    }
}

impl Default for SendInputDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputDevice for SendInputDevice {
    fn send_key_action(&self, action: &KeyAction) -> Result<()> {
        match action {
            KeyAction::Press {
                scan_code,
                virtual_key,
            } => {
                self.send_key(*scan_code, *virtual_key, false)?;
            }
            KeyAction::Release {
                scan_code,
                virtual_key,
            } => {
                self.send_key(*scan_code, *virtual_key, true)?;
            }
            KeyAction::Click {
                scan_code,
                virtual_key,
            } => {
                self.send_key(*scan_code, *virtual_key, false)?;
                self.send_key(*scan_code, *virtual_key, true)?;
            }
            KeyAction::TypeText(text) => {
                self.send_text(text)?;
            }
            KeyAction::Combo { modifiers, key } => {
                self.send_combo(modifiers, key.0, key.1)?;
            }
            KeyAction::None => {}
        }
        Ok(())
    }

    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
            KEYEVENTF_SCANCODE,
        };

        let mut input = INPUT {
            r#type: INPUT_KEYBOARD,
            ..Default::default()
        };

        unsafe {
            input.Anonymous.ki.wScan = scan_code;
            input.Anonymous.ki.dwFlags = KEYEVENTF_SCANCODE;

            if release {
                input.Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
            }

            // If extended key, add flag
            if virtual_key >= 0xE000 {
                input.Anonymous.ki.dwFlags |= KEYEVENTF_EXTENDEDKEY;
            }

            input.Anonymous.ki.time = 0;
            input.Anonymous.ki.dwExtraInfo = 0;
        }

        let result = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if result == 0 {
            return Err(anyhow::anyhow!("SendInput failed"));
        }

        trace!(
            "Sent key: scan_code={:04X}, vk={:04X}, release={}",
            scan_code,
            virtual_key,
            release
        );

        Ok(())
    }

    fn send_mouse_action(&self, action: &MouseAction) -> Result<()> {
        match action {
            MouseAction::Move { x, y, relative } => {
                self.send_mouse_move(*x, *y, *relative)?;
            }
            MouseAction::ButtonDown { button } => {
                self.send_mouse_button(*button, false)?;
            }
            MouseAction::ButtonUp { button } => {
                self.send_mouse_button(*button, true)?;
            }
            MouseAction::ButtonClick { button } => {
                self.send_mouse_button(*button, false)?;
                self.send_mouse_button(*button, true)?;
            }
            MouseAction::Wheel { delta } => {
                self.send_mouse_wheel(*delta, false)?;
            }
            MouseAction::HWheel { delta } => {
                self.send_mouse_wheel(*delta, true)?;
            }
            MouseAction::None => {}
        }
        Ok(())
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
        };

        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            ..Default::default()
        };

        unsafe {
            input.Anonymous.mi.dx = x;
            input.Anonymous.mi.dy = y;
            input.Anonymous.mi.dwFlags = MOUSEEVENTF_MOVE;

            if !relative {
                // Absolute coordinates (need to normalize to 0-65535)
                input.Anonymous.mi.dwFlags |= MOUSEEVENTF_ABSOLUTE;
            }
        }

        let result = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if result == 0 {
            return Err(anyhow::anyhow!("SendInput failed"));
        }

        trace!("Sent mouse move: x={}, y={}, relative={}", x, y, relative);

        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
            MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN,
            MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP,
        };

        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            ..Default::default()
        };

        input.Anonymous.mi.dwFlags = match button {
            MouseButton::Left => {
                if release {
                    MOUSEEVENTF_LEFTUP
                } else {
                    MOUSEEVENTF_LEFTDOWN
                }
            }
            MouseButton::Right => {
                if release {
                    MOUSEEVENTF_RIGHTUP
                } else {
                    MOUSEEVENTF_RIGHTDOWN
                }
            }
            MouseButton::Middle => {
                if release {
                    MOUSEEVENTF_MIDDLEUP
                } else {
                    MOUSEEVENTF_MIDDLEDOWN
                }
            }
            MouseButton::X1 => {
                input.Anonymous.mi.mouseData = 0x0001;
                if release {
                    MOUSEEVENTF_XUP
                } else {
                    MOUSEEVENTF_XDOWN
                }
            }
            MouseButton::X2 => {
                input.Anonymous.mi.mouseData = 0x0002;
                if release {
                    MOUSEEVENTF_XUP
                } else {
                    MOUSEEVENTF_XDOWN
                }
            }
        };

        let result = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if result == 0 {
            return Err(anyhow::anyhow!("SendInput failed"));
        }

        trace!("Sent mouse button: {:?}, release={}", button, release);

        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_WHEEL,
        };

        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            ..Default::default()
        };

        input.Anonymous.mi.mouseData = (delta * WHEEL_DELTA) as u32;
        input.Anonymous.mi.dwFlags = if horizontal {
            MOUSEEVENTF_HWHEEL
        } else {
            MOUSEEVENTF_WHEEL
        };

        let result = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if result == 0 {
            return Err(anyhow::anyhow!("SendInput failed"));
        }

        trace!(
            "Sent mouse wheel: delta={}, horizontal={}",
            delta,
            horizontal
        );

        Ok(())
    }

    fn send_system_action(&self, action: &SystemAction) -> Result<()> {
        match action {
            SystemAction::VolumeUp => {
                // VK_VOLUME_UP = 0xAF
                self.send_key(0, 0xAF, false)?;
                self.send_key(0, 0xAF, true)?;
            }
            SystemAction::VolumeDown => {
                // VK_VOLUME_DOWN = 0xAE
                self.send_key(0, 0xAE, false)?;
                self.send_key(0, 0xAE, true)?;
            }
            SystemAction::VolumeMute => {
                // VK_VOLUME_MUTE = 0xAD
                self.send_key(0, 0xAD, false)?;
                self.send_key(0, 0xAD, true)?;
            }
            SystemAction::BrightnessUp => {
                trace!("Brightness up not yet implemented");
            }
            SystemAction::BrightnessDown => {
                trace!("Brightness down not yet implemented");
            }
        }

        trace!("Sent system action: {:?}", action);
        Ok(())
    }
}

impl SendInputDevice {
    /// Send text (internal method)
    #[allow(dead_code)]
    fn send_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            if let Some(vk) = char_to_vk(ch) {
                self.send_key(0, vk, false)?;
                self.send_key(0, vk, true)?;
            }
        }
        Ok(())
    }

    /// Send key combo (internal method)
    #[allow(dead_code)]
    fn send_combo(
        &self,
        modifiers: &crate::types::ModifierState,
        scan_code: u16,
        virtual_key: u16,
    ) -> Result<()> {
        // Press modifier keys
        if modifiers.shift {
            self.send_key(0x2A, 0xA0, false)?; // LShift
        }
        if modifiers.ctrl {
            self.send_key(0x1D, 0xA2, false)?; // LCtrl
        }
        if modifiers.alt {
            self.send_key(0x38, 0xA4, false)?; // LAlt
        }
        if modifiers.meta {
            self.send_key(0xE05B, 0x5B, false)?; // LWin
        }

        // Press target key
        self.send_key(scan_code, virtual_key, false)?;

        // Release target key
        self.send_key(scan_code, virtual_key, true)?;

        // Release modifier keys (reverse order)
        if modifiers.meta {
            self.send_key(0xE05B, 0x5B, true)?;
        }
        if modifiers.alt {
            self.send_key(0x38, 0xA4, true)?;
        }
        if modifiers.ctrl {
            self.send_key(0x1D, 0xA2, true)?;
        }
        if modifiers.shift {
            self.send_key(0x2A, 0xA0, true)?;
        }

        Ok(())
    }
}

/// Mock output device for testing
#[cfg(test)]
pub struct MockOutputDevice {
    events: RefCell<Vec<OutputEvent>>,
}

#[cfg(test)]
impl MockOutputDevice {
    /// Create new Mock output device
    pub fn new() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
        }
    }

    /// Get all output events
    pub fn get_events(&self) -> Vec<OutputEvent> {
        self.events.borrow().clone()
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events.borrow().len()
    }

    /// Clear events
    pub fn clear(&self) {
        self.events.borrow_mut().clear();
    }

    /// Get last event
    pub fn last_event(&self) -> Option<OutputEvent> {
        self.events.borrow().last().cloned()
    }

    fn record_event(&self, event: OutputEvent) {
        self.events.borrow_mut().push(event);
    }
}

#[cfg(test)]
impl Default for MockOutputDevice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl OutputDevice for MockOutputDevice {
    fn send_key_action(&self, action: &KeyAction) -> Result<()> {
        match action {
            KeyAction::Press {
                scan_code,
                virtual_key,
            } => {
                self.record_event(OutputEvent::Key {
                    scan_code: *scan_code,
                    virtual_key: *virtual_key,
                    release: false,
                });
            }
            KeyAction::Release {
                scan_code,
                virtual_key,
            } => {
                self.record_event(OutputEvent::Key {
                    scan_code: *scan_code,
                    virtual_key: *virtual_key,
                    release: true,
                });
            }
            KeyAction::Click {
                scan_code,
                virtual_key,
            } => {
                self.record_event(OutputEvent::Key {
                    scan_code: *scan_code,
                    virtual_key: *virtual_key,
                    release: false,
                });
                self.record_event(OutputEvent::Key {
                    scan_code: *scan_code,
                    virtual_key: *virtual_key,
                    release: true,
                });
            }
            KeyAction::TypeText(text) => {
                for ch in text.chars() {
                    if let Some(vk) = char_to_vk(ch) {
                        self.record_event(OutputEvent::Key {
                            scan_code: 0,
                            virtual_key: vk,
                            release: false,
                        });
                        self.record_event(OutputEvent::Key {
                            scan_code: 0,
                            virtual_key: vk,
                            release: true,
                        });
                    }
                }
            }
            KeyAction::Combo { modifiers, key } => {
                // Press modifier keys
                if modifiers.shift {
                    self.record_event(OutputEvent::Key {
                        scan_code: 0x2A,
                        virtual_key: 0xA0,
                        release: false,
                    });
                }
                if modifiers.ctrl {
                    self.record_event(OutputEvent::Key {
                        scan_code: 0x1D,
                        virtual_key: 0xA2,
                        release: false,
                    });
                }
                if modifiers.alt {
                    self.record_event(OutputEvent::Key {
                        scan_code: 0x38,
                        virtual_key: 0xA4,
                        release: false,
                    });
                }
                if modifiers.meta {
                    self.record_event(OutputEvent::Key {
                        scan_code: 0xE05B,
                        virtual_key: 0x5B,
                        release: false,
                    });
                }
                // Target key
                self.record_event(OutputEvent::Key {
                    scan_code: key.0,
                    virtual_key: key.1,
                    release: false,
                });
                self.record_event(OutputEvent::Key {
                    scan_code: key.0,
                    virtual_key: key.1,
                    release: true,
                });
                // Release modifier keys
                if modifiers.meta {
                    self.record_event(OutputEvent::Key {
                        scan_code: 0xE05B,
                        virtual_key: 0x5B,
                        release: true,
                    });
                }
                if modifiers.alt {
                    self.record_event(OutputEvent::Key {
                        scan_code: 0x38,
                        virtual_key: 0xA4,
                        release: true,
                    });
                }
                if modifiers.ctrl {
                    self.record_event(OutputEvent::Key {
                        scan_code: 0x1D,
                        virtual_key: 0xA2,
                        release: true,
                    });
                }
                if modifiers.shift {
                    self.record_event(OutputEvent::Key {
                        scan_code: 0x2A,
                        virtual_key: 0xA0,
                        release: true,
                    });
                }
            }
            KeyAction::None => {}
        }
        Ok(())
    }

    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        self.record_event(OutputEvent::Key {
            scan_code,
            virtual_key,
            release,
        });
        Ok(())
    }

    fn send_mouse_action(&self, action: &MouseAction) -> Result<()> {
        match action {
            MouseAction::Move { x, y, relative } => {
                self.record_event(OutputEvent::MouseMove {
                    x: *x,
                    y: *y,
                    relative: *relative,
                });
            }
            MouseAction::ButtonDown { button } => {
                self.record_event(OutputEvent::MouseButton {
                    button: *button,
                    release: false,
                });
            }
            MouseAction::ButtonUp { button } => {
                self.record_event(OutputEvent::MouseButton {
                    button: *button,
                    release: true,
                });
            }
            MouseAction::ButtonClick { button } => {
                self.record_event(OutputEvent::MouseButton {
                    button: *button,
                    release: false,
                });
                self.record_event(OutputEvent::MouseButton {
                    button: *button,
                    release: true,
                });
            }
            MouseAction::Wheel { delta } => {
                self.record_event(OutputEvent::MouseWheel {
                    delta: *delta,
                    horizontal: false,
                });
            }
            MouseAction::HWheel { delta } => {
                self.record_event(OutputEvent::MouseWheel {
                    delta: *delta,
                    horizontal: true,
                });
            }
            MouseAction::None => {}
        }
        Ok(())
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        self.record_event(OutputEvent::MouseMove { x, y, relative });
        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        self.record_event(OutputEvent::MouseButton { button, release });
        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        self.record_event(OutputEvent::MouseWheel { delta, horizontal });
        Ok(())
    }

    fn send_system_action(&self, _action: &SystemAction) -> Result<()> {
        // Mock implementation doesn't record system actions
        Ok(())
    }
}

/// Convert character to virtual key code (simplified)
#[allow(dead_code)]
fn char_to_vk(ch: char) -> Option<u16> {
    match ch {
        'a'..='z' => Some(ch as u16 - 'a' as u16 + 0x41),
        'A'..='Z' => Some(ch as u16 - 'A' as u16 + 0x41),
        '0'..='9' => Some(ch as u16 - '0' as u16 + 0x30),
        ' ' => Some(0x20),
        '\n' => Some(0x0D),
        '\t' => Some(0x09),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ModifierState;

    #[test]
    fn test_mock_output_device_creation() {
        let device = MockOutputDevice::new();
        assert_eq!(device.event_count(), 0);
    }

    #[test]
    fn test_mock_send_key() {
        let device = MockOutputDevice::new();

        device.send_key(0x1E, 0x41, false).unwrap();
        device.send_key(0x1E, 0x41, true).unwrap();

        assert_eq!(device.event_count(), 2);

        let events = device.get_events();
        assert_eq!(
            events[0],
            OutputEvent::Key {
                scan_code: 0x1E,
                virtual_key: 0x41,
                release: false
            }
        );
        assert_eq!(
            events[1],
            OutputEvent::Key {
                scan_code: 0x1E,
                virtual_key: 0x41,
                release: true
            }
        );
    }

    #[test]
    fn test_mock_send_key_action_press() {
        let device = MockOutputDevice::new();

        let action = KeyAction::Press {
            scan_code: 0x1E,
            virtual_key: 0x41,
        };
        device.send_key_action(&action).unwrap();

        assert_eq!(device.event_count(), 1);
        assert_eq!(
            device.last_event(),
            Some(OutputEvent::Key {
                scan_code: 0x1E,
                virtual_key: 0x41,
                release: false
            })
        );
    }

    #[test]
    fn test_mock_send_key_action_click() {
        let device = MockOutputDevice::new();

        let action = KeyAction::Click {
            scan_code: 0x1E,
            virtual_key: 0x41,
        };
        device.send_key_action(&action).unwrap();

        assert_eq!(device.event_count(), 2);
    }

    #[test]
    fn test_mock_send_mouse_move() {
        let device = MockOutputDevice::new();

        device.send_mouse_move(100, 200, true).unwrap();

        assert_eq!(device.event_count(), 1);
        assert_eq!(
            device.last_event(),
            Some(OutputEvent::MouseMove {
                x: 100,
                y: 200,
                relative: true
            })
        );
    }

    #[test]
    fn test_mock_send_mouse_button() {
        let device = MockOutputDevice::new();

        device.send_mouse_button(MouseButton::Left, false).unwrap();
        device.send_mouse_button(MouseButton::Left, true).unwrap();

        assert_eq!(device.event_count(), 2);
    }

    #[test]
    fn test_mock_send_mouse_wheel() {
        let device = MockOutputDevice::new();

        device.send_mouse_wheel(3, false).unwrap();
        device.send_mouse_wheel(-3, true).unwrap();

        assert_eq!(device.event_count(), 2);

        let events = device.get_events();
        assert_eq!(
            events[0],
            OutputEvent::MouseWheel {
                delta: 3,
                horizontal: false
            }
        );
        assert_eq!(
            events[1],
            OutputEvent::MouseWheel {
                delta: -3,
                horizontal: true
            }
        );
    }

    #[test]
    fn test_mock_send_mouse_action_click() {
        let device = MockOutputDevice::new();

        let action = MouseAction::ButtonClick {
            button: MouseButton::Right,
        };
        device.send_mouse_action(&action).unwrap();

        assert_eq!(device.event_count(), 2);
    }

    #[test]
    fn test_mock_clear() {
        let device = MockOutputDevice::new();

        device.send_key(0x1E, 0x41, false).unwrap();
        assert_eq!(device.event_count(), 1);

        device.clear();
        assert_eq!(device.event_count(), 0);
    }

    #[test]
    fn test_mock_send_combo() {
        let device = MockOutputDevice::new();

        let modifiers = ModifierState {
            shift: false,
            ctrl: true,
            alt: false,
            meta: false,
        };
        let action = KeyAction::Combo {
            modifiers,
            key: (0x1E, 0x41), // 'A' key
        };
        device.send_key_action(&action).unwrap();

        // Ctrl down, A down, A up, Ctrl up = 4 events
        assert_eq!(device.event_count(), 4);
    }

    #[test]
    fn test_char_to_vk() {
        assert_eq!(char_to_vk('a'), Some(0x41));
        assert_eq!(char_to_vk('A'), Some(0x41));
        assert_eq!(char_to_vk('0'), Some(0x30));
        assert_eq!(char_to_vk('9'), Some(0x39));
        assert_eq!(char_to_vk(' '), Some(0x20));
        assert_eq!(char_to_vk('\n'), Some(0x0D));
    }
}
