//! Windows output device implementation using SendInput API
//!
//! This module uses Windows SendInput for simulated input events.
//! Shared logic (char mapping, text input, key combos) is in [output_helpers].
#![cfg(target_os = "windows")]

use crate::platform::traits::OutputDeviceTrait;
use crate::types::MouseButton;
use anyhow::Result;
#[cfg(not(test))]
use tracing::{debug, warn};
#[cfg(not(test))]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_WHEEL, VIRTUAL_KEY,
};

/// SendInput-based output device implementing [OutputDeviceTrait]
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

#[cfg(not(test))]
impl OutputDeviceTrait for SendInputDevice {
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
            // MOUSEEVENTF_XDOWN = 0x0080, MOUSEEVENTF_XUP = 0x0100
            // XBUTTON1 = 0x0001, XBUTTON2 = 0x0002
            (MouseButton::X1, false) => {
                input.Anonymous.mi.mouseData = 0x0001;
                windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(0x0080)
            }
            (MouseButton::X1, true) => {
                input.Anonymous.mi.mouseData = 0x0001;
                windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(0x0100)
            }
            (MouseButton::X2, false) => {
                input.Anonymous.mi.mouseData = 0x0002;
                windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(0x0080)
            }
            (MouseButton::X2, true) => {
                input.Anonymous.mi.mouseData = 0x0002;
                windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS(0x0100)
            }
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
crate::impl_test_output_device!(SendInputDevice);
