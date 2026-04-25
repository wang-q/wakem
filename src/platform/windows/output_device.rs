//! Windows output device implementation using SendInput API
//!
//! This module uses Windows SendInput for simulated input events.
//! Shared logic (char mapping, text input, key combos) is in [output_helpers].
#![cfg(target_os = "windows")]
#![allow(dead_code)]

use crate::platform::output_helpers::char_to_vk;
use crate::platform::traits::OutputDeviceTrait;
use crate::types::{KeyAction, ModifierState, MouseAction, MouseButton, SystemAction};
use anyhow::Result;
use tracing::{debug, warn};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_WHEEL, VIRTUAL_KEY,
};

/// Local OutputDevice trait for SendInput-based implementation
pub trait OutputDevice: Send + Sync {
    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()>;
    fn send_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            if let Some(vk) = char_to_vk(ch) {
                self.send_key(0, vk, false)?;
                self.send_key(0, vk, true)?;
            }
        }
        Ok(())
    }
    fn send_combo(
        &self,
        modifiers: &ModifierState,
        scan_code: u16,
        virtual_key: u16,
    ) -> Result<()> {
        if modifiers.shift {
            self.send_key(0x2A, 0x10, false)?;
        }
        if modifiers.ctrl {
            self.send_key(0x1D, 0x11, false)?;
        }
        if modifiers.alt {
            self.send_key(0x38, 0x12, false)?;
        }

        self.send_key(scan_code, virtual_key, false)?;
        self.send_key(scan_code, virtual_key, true)?;

        if modifiers.alt {
            self.send_key(0x38, 0x12, true)?;
        }
        if modifiers.ctrl {
            self.send_key(0x1D, 0x11, true)?;
        }
        if modifiers.shift {
            self.send_key(0x2A, 0x10, true)?;
        }

        Ok(())
    }
    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()>;
    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()>;
    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()>;
}

/// SendInput-based output device
#[derive(Debug, Clone)]
pub struct SendInputDevice;

impl Default for SendInputDevice {
    fn default() -> Self {
        Self
    }
}

impl SendInputDevice {
    pub fn new() -> Self {
        Self
    }
}

impl OutputDevice for SendInputDevice {
    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        let mut input = INPUT {
            r#type: INPUT_KEYBOARD,
            ..Default::default()
        };

        unsafe {
            input.Anonymous.ki.wVk = VIRTUAL_KEY(virtual_key);
            input.Anonymous.ki.wScan = scan_code;
            input.Anonymous.ki.dwFlags = KEYEVENTF_SCANCODE;

            if release {
                input.Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
            }

            // Set extended flag for certain keys
            let is_extended =
                matches!(scan_code, 0x36 | 0x2A | 0x1D | 0x38 | 0x5B | 0x5C | 0x5D);
            if is_extended {
                input.Anonymous.ki.dwFlags |= KEYEVENTF_EXTENDEDKEY;
            }

            input.Anonymous.ki.time = 0;
            input.Anonymous.ki.dwExtraInfo = 0;
        }

        let result = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if result == 0 {
            warn!("SendInput failed for key event");
        }

        debug!(
            "Sent key: scan={:#04X}, vk={:#04X}, release={}",
            scan_code, virtual_key, release
        );
        Ok(())
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            ..Default::default()
        };

        unsafe {
            input.Anonymous.mi.dx = x;
            input.Anonymous.mi.dy = y;
            input.Anonymous.mi.dwFlags = MOUSEEVENTF_MOVE;

            if !relative {
                input.Anonymous.mi.dwFlags |= MOUSEEVENTF_ABSOLUTE;
            }

            input.Anonymous.mi.time = 0;
            input.Anonymous.mi.dwExtraInfo = 0;
        }

        let result = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if result == 0 {
            warn!("SendInput failed for mouse move");
        }

        debug!("Sent mouse move: x={}, y={}, relative={}", x, y, relative);
        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            ..Default::default()
        };

        input.Anonymous.mi.mouseData = 0;
        input.Anonymous.mi.time = 0;
        input.Anonymous.mi.dwExtraInfo = 0;

        input.Anonymous.mi.dwFlags = match (button, release) {
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTUP,
            (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEDOWN,
            (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEUP,
            _ => return Ok(()),
        };

        let result = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if result != 1 {
            warn!("SendInput failed for mouse button");
        }

        debug!("Sent mouse button: {:?}, release={}", button, release);
        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            ..Default::default()
        };

        input.Anonymous.mi.dx = 0;
        input.Anonymous.mi.dy = 0;
        input.Anonymous.mi.mouseData = delta as u32;
        input.Anonymous.mi.dwFlags = if horizontal {
            MOUSEEVENTF_HWHEEL
        } else {
            MOUSEEVENTF_WHEEL
        };
        input.Anonymous.mi.time = 0;
        input.Anonymous.mi.dwExtraInfo = 0;

        let result = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if result != 1 {
            warn!("SendInput failed for mouse wheel");
        }

        debug!(
            "Sent mouse wheel: delta={}, horizontal={}",
            delta, horizontal
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::output_helpers::char_to_vk;

    // --- char_to_vk tests (shared logic) ---

    #[test]
    fn test_char_to_vk_lowercase() {
        assert_eq!(char_to_vk('a'), Some(0x41));
        assert_eq!(char_to_vk('b'), Some(0x42));
        assert_eq!(char_to_vk('m'), Some(0x4D));
        assert_eq!(char_to_vk('z'), Some(0x5A));
    }

    #[test]
    fn test_char_to_vk_uppercase() {
        assert_eq!(char_to_vk('A'), Some(0x41));
        assert_eq!(char_to_vk('Z'), Some(0x5A));
    }

    #[test]
    fn test_char_to_vk_digits() {
        assert_eq!(char_to_vk('0'), Some(0x30));
        assert_eq!(char_to_vk('5'), Some(0x35));
        assert_eq!(char_to_vk('9'), Some(0x39));
    }

    #[test]
    fn test_char_to_vk_special() {
        assert_eq!(char_to_vk(' '), Some(0x20));
        assert_eq!(char_to_vk('\t'), Some(0x09));
        assert_eq!(char_to_vk('\n'), Some(0x0D));
        assert_eq!(char_to_vk('\r'), Some(0x0D));
    }

    #[test]
    fn test_char_to_vk_unsupported() {
        assert_eq!(char_to_vk('@'), None);
        assert_eq!(char_to_vk('#'), None);
        assert_eq!(char_to_vk('中'), None);
        assert_eq!(char_to_vk('é'), None);
    }

    // --- SendInputDevice lifecycle (no side effects) ---

    #[test]
    fn test_send_input_device_creation() {
        let device = SendInputDevice::new();
        let _cloned = device.clone();
    }

    #[test]
    fn test_send_input_device_default() {
        let _device = SendInputDevice::default();
    }

    // --- WindowsOutputDevice lifecycle (no side effects) ---

    #[test]
    fn test_windows_output_device_creation() {
        let device = WindowsOutputDevice::new();
        let _cloned = device.clone();
    }

    #[test]
    fn test_windows_output_device_default() {
        let _device = WindowsOutputDevice::default();
    }
}

/// Re-export MockOutputDevice and MockOutputEvent from platform::mock
#[cfg(test)]
pub use crate::platform::mock::{MockOutputDevice, MockOutputEvent};

/// Windows output device implementing platform [OutputDeviceTrait]
///
/// Wraps [SendInputDevice] and provides system action support.
pub struct WindowsOutputDevice {
    inner: SendInputDevice,
}

impl Default for WindowsOutputDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for WindowsOutputDevice {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl WindowsOutputDevice {
    pub fn new() -> Self {
        Self {
            inner: SendInputDevice::new(),
        }
    }
}

// Non-test implementation: sends real events to the system
#[cfg(not(test))]
impl OutputDeviceTrait for WindowsOutputDevice {
    fn send_key_action(&self, action: &KeyAction) -> Result<()> {
        match action {
            KeyAction::Press {
                scan_code,
                virtual_key,
            } => self.inner.send_key(*scan_code, *virtual_key, false),
            KeyAction::Release {
                scan_code,
                virtual_key,
            } => self.inner.send_key(*scan_code, *virtual_key, true),
            KeyAction::Click {
                scan_code,
                virtual_key,
            } => {
                self.inner.send_key(*scan_code, *virtual_key, false)?;
                self.inner.send_key(*scan_code, *virtual_key, true)
            }
            KeyAction::TypeText(text) => self.inner.send_text(text),
            KeyAction::Combo { modifiers, key } => {
                self.inner.send_combo(modifiers, key.0, key.1)
            }
            KeyAction::None => Ok(()),
        }
    }

    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        self.inner.send_key(scan_code, virtual_key, release)
    }

    fn send_mouse_action(&self, action: &MouseAction) -> Result<()> {
        match action {
            MouseAction::Move { x, y, relative } => {
                self.inner.send_mouse_move(*x, *y, *relative)
            }
            MouseAction::ButtonDown { button } => {
                self.inner.send_mouse_button(*button, false)
            }
            MouseAction::ButtonUp { button } => {
                self.inner.send_mouse_button(*button, true)
            }
            MouseAction::ButtonClick { button } => {
                self.inner.send_mouse_button(*button, false)?;
                self.inner.send_mouse_button(*button, true)
            }
            MouseAction::Wheel { delta } => self.inner.send_mouse_wheel(*delta, false),
            MouseAction::HWheel { delta } => self.inner.send_mouse_wheel(*delta, true),
            MouseAction::None => Ok(()),
        }
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        self.inner.send_mouse_move(x, y, relative)
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        self.inner.send_mouse_button(button, release)
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        self.inner.send_mouse_wheel(delta, horizontal)
    }

    fn send_system_action(&self, action: &SystemAction) -> Result<()> {
        debug!("System action requested: {:?}", action);

        #[cfg(target_os = "windows")]
        {
            match action {
                SystemAction::VolumeUp => {
                    let mut input = INPUT {
                        r#type: INPUT_KEYBOARD,
                        ..Default::default()
                    };
                    input.Anonymous.ki.wVk = VIRTUAL_KEY(0xAF);
                    input.Anonymous.ki.wScan = 0;
                    input.Anonymous.ki.dwFlags = KEYEVENTF_SCANCODE;
                    input.Anonymous.ki.time = 0;
                    input.Anonymous.ki.dwExtraInfo = 0;
                    unsafe {
                        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                    }
                    unsafe {
                        input.Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
                    }
                    unsafe {
                        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                    }
                }
                SystemAction::VolumeDown => {
                    let mut input = INPUT {
                        r#type: INPUT_KEYBOARD,
                        ..Default::default()
                    };
                    input.Anonymous.ki.wVk = VIRTUAL_KEY(0xAE);
                    input.Anonymous.ki.wScan = 0;
                    input.Anonymous.ki.dwFlags = KEYEVENTF_SCANCODE;
                    input.Anonymous.ki.time = 0;
                    input.Anonymous.ki.dwExtraInfo = 0;
                    unsafe {
                        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                    }
                    unsafe {
                        input.Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
                    }
                    unsafe {
                        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                    }
                }
                SystemAction::VolumeMute => {
                    let mut input = INPUT {
                        r#type: INPUT_KEYBOARD,
                        ..Default::default()
                    };
                    input.Anonymous.ki.wVk = VIRTUAL_KEY(0xAD);
                    input.Anonymous.ki.wScan = 0;
                    input.Anonymous.ki.dwFlags = KEYEVENTF_SCANCODE;
                    input.Anonymous.ki.time = 0;
                    input.Anonymous.ki.dwExtraInfo = 0;
                    unsafe {
                        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                    }
                    unsafe {
                        input.Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
                    }
                    unsafe {
                        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
                    }
                }
                _ => {
                    warn!("System action not implemented on Windows: {:?}", action);
                }
            }
        }

        Ok(())
    }
}

// Test implementation: no-op to prevent interfering with the test environment
#[cfg(test)]
impl OutputDeviceTrait for WindowsOutputDevice {
    fn send_key_action(&self, action: &KeyAction) -> Result<()> {
        debug!("[TEST MODE] Mock key action: {:?}", action);
        Ok(())
    }

    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        debug!(
            "[TEST MODE] Mock key event: scan={:#04X}, vk={:#04X}, release={}",
            scan_code, virtual_key, release
        );
        Ok(())
    }

    fn send_mouse_action(&self, action: &MouseAction) -> Result<()> {
        debug!("[TEST MODE] Mock mouse action: {:?}", action);
        Ok(())
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        debug!(
            "[TEST MODE] Mock mouse move: x={}, y={}, relative={}",
            x, y, relative
        );
        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        debug!(
            "[TEST MODE] Mock mouse button: {:?}, release={}",
            button, release
        );
        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        debug!(
            "[TEST MODE] Mock mouse wheel: delta={}, horizontal={}",
            delta, horizontal
        );
        Ok(())
    }

    fn send_system_action(&self, action: &SystemAction) -> Result<()> {
        debug!("[TEST MODE] Mock system action: {:?}", action);
        Ok(())
    }
}
