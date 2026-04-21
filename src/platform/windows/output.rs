use crate::types::{KeyAction, MouseAction, MouseButton, SystemAction};
use anyhow::Result;
#[allow(unused_imports)]
use tracing::trace;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, INPUT_MOUSE, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_HWHEEL,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP,
};

/// Output device (for sending simulated input)
#[derive(Clone, Copy)]
pub struct OutputDevice;

impl OutputDevice {
    /// Create new output device
    pub fn new() -> Self {
        Self
    }

    /// Send key action
    pub fn send_key_action(&self, action: &KeyAction) -> Result<()> {
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

    /// Send single key
    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
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

    /// Send text
    fn send_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            // Simple implementation: convert character to virtual key code and send
            // Actual implementation needs more complex Unicode input handling
            if let Some(vk) = char_to_vk(ch) {
                self.send_key(0, vk, false)?;
                self.send_key(0, vk, true)?;
            }
        }
        Ok(())
    }

    /// Send key combo
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

    /// Send mouse action
    pub fn send_mouse_action(&self, action: &MouseAction) -> Result<()> {
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

    /// Send mouse move
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

    /// Send mouse button
    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
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

    /// Send mouse wheel
    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            ..Default::default()
        };

        input.Anonymous.mi.mouseData = (delta * 120) as u32; // WHEEL_DELTA = 120
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

    /// Send system control action (volume, brightness)
    pub fn send_system_action(&self, action: &SystemAction) -> Result<()> {
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
                // Brightness control has no standard virtual key, use system shortcut
                // Simulate Win + A to open action center, then send brightness up key
                // Simplified handling here using combo keys
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

impl Default for OutputDevice {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert character to virtual key code (simplified)
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
