//! Windows output device implementation using SendInput API
//!
//! This module uses Windows SendInput for simulated input events.
//! Shared logic (char mapping, text input, key combos) is in [output_helpers].

use crate::platform::output_helpers::char_to_vk;
use crate::platform::traits::OutputDeviceTrait;
use crate::types::{KeyAction, ModifierState, MouseAction, MouseButton, SystemAction};
use anyhow::Result;
use tracing::{debug, warn};

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
        let mut input =
            unsafe { std::mem::zeroed::<windows_sys::Win32::UI::Input::INPUT>() };
        input.type_ = windows_sys::Win32::UI::Input::INPUT_KEYBOARD;

        let ki = unsafe { &mut input.u.ki_mut() };

        // Set extended flag for certain keys
        let is_extended =
            matches!(scan_code, 0x36 | 0x2A | 0x1D | 0x38 | 0x5B | 0x5C | 0x5D);

        ki.wVk = virtual_key;
        ki.wScan = scan_code;
        ki.dwFlags = if release {
            windows_sys::Win32::UI::Input::KEYEVENTF_KEYUP
        } else {
            0
        } | if is_extended {
            windows_sys::Win32::UI::Input::KEYEVENTF_EXTENDEDKEY
        } else {
            0
        };
        ki.time = 0;
        ki.dwExtraInfo = 0;

        let result = unsafe {
            windows_sys::Win32::UI::Input::SendInput(
                1,
                &input,
                std::mem::size_of::<windows_sys::Win32::UI::Input::INPUT>() as i32,
            )
        };

        if result != 1 {
            warn!("SendInput failed for key event");
        }

        debug!(
            "Sent key: scan={:#04X}, vk={:#04X}, release={}",
            scan_code, virtual_key, release
        );
        Ok(())
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        let mut input =
            unsafe { std::mem::zeroed::<windows_sys::Win32::UI::Input::INPUT>() };
        input.type_ = windows_sys::Win32::UI::Input::INPUT_MOUSE;

        let mi = unsafe { &mut input.u.mi_mut() };
        mi.dx = x;
        mi.dy = y;
        mi.mouseData = 0;
        mi.dwFlags = if relative {
            windows_sys::Win32::UI::Input::MOUSEEVENTF_MOVE
        } else {
            windows_sys::Win32::UI::Input::MOUSEEVENTF_MOVE
                | windows_sys::Win32::UI::Input::MOUSEEVENTF_ABSOLUTE
        };
        mi.time = 0;
        mi.dwExtraInfo = 0;

        let result = unsafe {
            windows_sys::Win32::UI::Input::SendInput(
                1,
                &input,
                std::mem::size_of::<windows_sys::Win32::UI::Input::INPUT>() as i32,
            )
        };

        if result != 1 {
            warn!("SendInput failed for mouse move");
        }

        debug!("Sent mouse move: x={}, y={}, relative={}", x, y, relative);
        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        let mut input =
            unsafe { std::mem::zeroed::<windows_sys::Win32::UI::Input::INPUT>() };
        input.type_ = windows_sys::Win32::UI::Input::INPUT_MOUSE;

        let mi = unsafe { &mut input.u.mi_mut() };
        mi.mouseData = 0;
        mi.time = 0;
        mi.dwExtraInfo = 0;

        match (button, release) {
            (MouseButton::Left, false) => {
                mi.dwFlags = windows_sys::Win32::UI::Input::MOUSEEVENTF_LEFTDOWN;
            }
            (MouseButton::Left, true) => {
                mi.dwFlags = windows_sys::Win32::UI::Input::MOUSEEVENTF_LEFTUP;
            }
            (MouseButton::Right, false) => {
                mi.dwFlags = windows_sys::Win32::UI::Input::MOUSEEVENTF_RIGHTDOWN;
            }
            (MouseButton::Right, true) => {
                mi.dwFlags = windows_sys::Win32::UI::Input::MOUSEEVENTF_RIGHTUP;
            }
            (MouseButton::Middle, false) => {
                mi.dwFlags = windows_sys::Win32::UI::Input::MOUSEEVENTF_MIDDLEDOWN;
            }
            (MouseButton::Middle, true) => {
                mi.dwFlags = windows_sys::Win32::UI::Input::MOUSEEVENTF_MIDDLEUP;
            }
            _ => {}
        }

        let result = unsafe {
            windows_sys::Win32::UI::Input::SendInput(
                1,
                &input,
                std::mem::size_of::<windows_sys::Win32::UI::Input::INPUT>() as i32,
            )
        };

        if result != 1 {
            warn!("SendInput failed for mouse button");
        }

        debug!("Sent mouse button: {:?}, release={}", button, release);
        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        let mut input =
            unsafe { std::mem::zeroed::<windows_sys::Win32::UI::Input::INPUT>() };
        input.type_ = windows_sys::Win32::UI::Input::INPUT_MOUSE;

        let mi = unsafe { &mut input.u.mi_mut() };
        mi.dx = 0;
        mi.dy = 0;
        mi.mouseData = delta as u32;
        mi.dwFlags = if horizontal {
            windows_sys::Win32::UI::Input::MOUSEEVENTF_HWHEEL
        } else {
            windows_sys::Win32::UI::Input::MOUSEEVENTF_WHEEL
        };
        mi.time = 0;
        mi.dwExtraInfo = 0;

        let result = unsafe {
            windows_sys::Win32::UI::Input::SendInput(
                1,
                &input,
                std::mem::size_of::<windows_sys::Win32::UI::Input::INPUT>() as i32,
            )
        };

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
    use crate::types::{KeyAction, ModifierState, MouseAction, MouseButton};
    use std::cell::RefCell;
    use std::sync::Arc;

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

/// Recorded output event for mock verification
#[derive(Debug, Clone)]
pub enum MockOutputEvent {
    Key {
        scan_code: u16,
        virtual_key: u16,
        release: bool,
    },
    MouseMove {
        x: i32,
        y: i32,
        relative: bool,
    },
    MouseButton {
        button: MouseButton,
        release: bool,
    },
    MouseWheel {
        delta: i32,
        horizontal: bool,
    },
    SystemAction(SystemAction),
}

#[cfg(test)]
use std::cell::RefCell;

/// Mock output device implementing the local OutputDevice trait
///
/// Records all calls without sending real input events.
#[cfg(test)]
pub struct MockOutputDevice {
    events: RefCell<Vec<MockOutputEvent>>,
}

#[cfg(test)]
impl MockOutputDevice {
    pub fn new() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
        }
    }

    pub fn recorded_events(&self) -> Vec<MockOutputEvent> {
        self.events.borrow().clone()
    }

    pub fn clear(&self) {
        self.events.borrow_mut().clear();
    }

    pub fn event_count(&self) -> usize {
        self.events.borrow().len()
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
    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()> {
        self.events.borrow_mut().push(MockOutputEvent::Key {
            scan_code,
            virtual_key,
            release,
        });
        Ok(())
    }

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()> {
        self.events
            .borrow_mut()
            .push(MockOutputEvent::MouseMove { x, y, relative });
        Ok(())
    }

    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()> {
        self.events
            .borrow_mut()
            .push(MockOutputEvent::MouseButton { button, release });
        Ok(())
    }

    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()> {
        self.events
            .borrow_mut()
            .push(MockOutputEvent::MouseWheel { delta, horizontal });
        Ok(())
    }
}

/// Mock output device implementing the platform OutputDeviceTrait
///
/// Wraps MockOutputDevice and is safe for use in tests.
#[cfg(test)]
pub struct MockWindowsOutputDevice {
    inner: MockOutputDevice,
}

#[cfg(test)]
impl MockWindowsOutputDevice {
    pub fn new() -> Self {
        Self {
            inner: MockOutputDevice::new(),
        }
    }

    pub fn recorded_events(&self) -> Vec<MockOutputEvent> {
        self.inner.recorded_events()
    }

    pub fn clear(&self) {
        self.inner.clear()
    }

    pub fn event_count(&self) -> usize {
        self.inner.event_count()
    }
}

#[cfg(test)]
impl Default for MockWindowsOutputDevice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Clone for MockWindowsOutputDevice {
    fn clone(&self) -> Self {
        Self {
            inner: MockOutputDevice::new(),
        }
    }
}

#[cfg(test)]
impl OutputDeviceTrait for MockWindowsOutputDevice {
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
        self.inner
            .events
            .borrow_mut()
            .push(MockOutputEvent::SystemAction(action.clone()));
        Ok(())
    }
}

#[cfg(test)]
mod mock_tests {
    use super::*;
    use crate::types::{KeyAction, ModifierState, MouseAction, MouseButton};

    // --- MockOutputDevice: send_text / send_combo ---

    #[test]
    fn test_mock_send_text_ab() {
        let device = MockOutputDevice::new();
        device.send_text("ab").unwrap();
        let events = device.recorded_events();
        assert_eq!(events.len(), 4); // a press+release, b press+release
        if let MockOutputEvent::Key {
            virtual_key,
            release,
            ..
        } = &events[0]
        {
            assert_eq!(*virtual_key, 0x41);
            assert!(!release);
        } else {
            panic!("Expected key event");
        }
    }

    #[test]
    fn test_mock_send_text_empty() {
        let device = MockOutputDevice::new();
        device.send_text("").unwrap();
        assert_eq!(device.event_count(), 0);
    }

    #[test]
    fn test_mock_send_text_unsupported_chars_skipped() {
        let device = MockOutputDevice::new();
        device.send_text("a中b").unwrap();
        let events = device.recorded_events();
        assert_eq!(events.len(), 4); // '中' is skipped
    }

    #[test]
    fn test_mock_send_combo_ctrl_a() {
        let device = MockOutputDevice::new();
        let modifiers = ModifierState {
            ctrl: true,
            ..ModifierState::default()
        };
        device.send_combo(&modifiers, 0x1D, 0x41).unwrap();
        let events = device.recorded_events();
        assert_eq!(events.len(), 6); // ctrl down, a down+up, ctrl up
    }

    #[test]
    fn test_mock_send_combo_shift_alt() {
        let device = MockOutputDevice::new();
        let modifiers = ModifierState {
            shift: true,
            alt: true,
            ..ModifierState::default()
        };
        device.send_combo(&modifiers, 0, 0x42).unwrap();
        let events = device.recorded_events();
        assert_eq!(events.len(), 8); // shift down, alt down, b down+up, alt up, shift up
    }

    // --- MockWindowsOutputDevice: send_key_action ---

    #[test]
    fn test_mock_send_key_action_type_text() {
        let device = MockWindowsOutputDevice::new();
        let action = KeyAction::TypeText("hello".to_string());
        device.send_key_action(&action).unwrap();
        assert_eq!(device.event_count(), 10); // 5 chars * 2 (press+release)
    }

    #[test]
    fn test_mock_send_key_action_click() {
        let device = MockWindowsOutputDevice::new();
        let action = KeyAction::Click {
            scan_code: 0x1C,
            virtual_key: 0x41,
        };
        device.send_key_action(&action).unwrap();
        assert_eq!(device.event_count(), 2); // press + release
    }

    #[test]
    fn test_mock_send_key_action_press_release() {
        let device = MockWindowsOutputDevice::new();
        let press = KeyAction::Press {
            scan_code: 0x1C,
            virtual_key: 0x41,
        };
        let release = KeyAction::Release {
            scan_code: 0x1C,
            virtual_key: 0x41,
        };
        device.send_key_action(&press).unwrap();
        device.send_key_action(&release).unwrap();
        assert_eq!(device.event_count(), 2);
        let events = device.recorded_events();
        if let MockOutputEvent::Key { release: r, .. } = &events[0] {
            assert!(!r);
        }
        if let MockOutputEvent::Key { release: r, .. } = &events[1] {
            assert!(*r);
        }
    }

    #[test]
    fn test_mock_send_key_action_combo() {
        let device = MockWindowsOutputDevice::new();
        let modifiers = ModifierState {
            ctrl: true,
            shift: true,
            ..ModifierState::default()
        };
        let action = KeyAction::Combo {
            modifiers,
            key: (0x2A, 0x53),
        };
        device.send_key_action(&action).unwrap();
        assert!(device.event_count() > 0);
    }

    #[test]
    fn test_mock_send_key_action_none() {
        let device = MockWindowsOutputDevice::new();
        device.send_key_action(&KeyAction::None).unwrap();
        assert_eq!(device.event_count(), 0);
    }

    // --- MockWindowsOutputDevice: mouse actions ---

    #[test]
    fn test_mock_send_mouse_action_move_relative() {
        let device = MockWindowsOutputDevice::new();
        let action = MouseAction::Move {
            x: 100,
            y: 200,
            relative: true,
        };
        device.send_mouse_action(&action).unwrap();
        assert_eq!(device.event_count(), 1);
        if let MockOutputEvent::MouseMove { relative, .. } = &device.recorded_events()[0]
        {
            assert!(*relative);
        }
    }

    #[test]
    fn test_mock_send_mouse_action_move_absolute() {
        let device = MockWindowsOutputDevice::new();
        let action = MouseAction::Move {
            x: 1920,
            y: 1080,
            relative: false,
        };
        device.send_mouse_action(&action).unwrap();
        assert_eq!(device.event_count(), 1);
        if let MockOutputEvent::MouseMove { relative, .. } = &device.recorded_events()[0]
        {
            assert!(!*relative);
        }
    }

    #[test]
    fn test_mock_send_mouse_action_left_click() {
        let device = MockWindowsOutputDevice::new();
        let action = MouseAction::ButtonClick {
            button: MouseButton::Left,
        };
        device.send_mouse_action(&action).unwrap();
        assert_eq!(device.event_count(), 2); // down + up
    }

    #[test]
    fn test_mock_send_mouse_action_right_click() {
        let device = MockWindowsOutputDevice::new();
        let action = MouseAction::ButtonClick {
            button: MouseButton::Right,
        };
        device.send_mouse_action(&action).unwrap();
        assert_eq!(device.event_count(), 2);
    }

    #[test]
    fn test_mock_send_mouse_action_middle_click() {
        let device = MockWindowsOutputDevice::new();
        let action = MouseAction::ButtonClick {
            button: MouseButton::Middle,
        };
        device.send_mouse_action(&action).unwrap();
        assert_eq!(device.event_count(), 2);
    }

    #[test]
    fn test_mock_send_mouse_action_wheel_vertical() {
        let device = MockWindowsOutputDevice::new();
        let action = MouseAction::Wheel { delta: 120 };
        device.send_mouse_action(&action).unwrap();
        assert_eq!(device.event_count(), 1);
        if let MockOutputEvent::MouseWheel { horizontal, .. } =
            &device.recorded_events()[0]
        {
            assert!(!*horizontal);
        }
    }

    #[test]
    fn test_mock_send_mouse_action_wheel_horizontal() {
        let device = MockWindowsOutputDevice::new();
        let action = MouseAction::HWheel { delta: 120 };
        device.send_mouse_action(&action).unwrap();
        assert_eq!(device.event_count(), 1);
        if let MockOutputEvent::MouseWheel { horizontal, .. } =
            &device.recorded_events()[0]
        {
            assert!(*horizontal);
        }
    }

    // --- MockWindowsOutputDevice: system actions ---

    #[test]
    fn test_mock_send_system_action_volume_up() {
        let device = MockWindowsOutputDevice::new();
        device.send_system_action(&SystemAction::VolumeUp).unwrap();
        assert_eq!(device.event_count(), 1);
        if let MockOutputEvent::SystemAction(action) = &device.recorded_events()[0] {
            assert!(matches!(action, SystemAction::VolumeUp));
        }
    }

    #[test]
    fn test_mock_send_system_action_mute() {
        let device = MockWindowsOutputDevice::new();
        device.send_system_action(&SystemAction::Mute).unwrap();
        assert_eq!(device.event_count(), 1);
    }

    // --- Mock lifecycle and utility ---

    #[test]
    fn test_mock_output_device_creation() {
        let _device = MockOutputDevice::new();
        let _device = MockOutputDevice::default();
    }

    #[test]
    fn test_mock_windows_output_device_creation() {
        let _device = MockWindowsOutputDevice::new();
        let _device = MockWindowsOutputDevice::default();
        let _cloned = MockWindowsOutputDevice::new().clone();
    }

    #[test]
    fn test_mock_clear() {
        let device = MockOutputDevice::new();
        device.send_text("ab").unwrap();
        assert_eq!(device.event_count(), 4);
        device.clear();
        assert_eq!(device.event_count(), 0);
    }

    #[test]
    fn test_mock_event_ordering() {
        let device = MockOutputDevice::new();
        device.send_key(0x1E, 0x41, false).unwrap(); // A press
        device.send_key(0x1E, 0x41, true).unwrap(); // A release
        device.send_key(0x30, 0x42, false).unwrap(); // B press
        let events = device.recorded_events();
        assert_eq!(events.len(), 3);
        if let MockOutputEvent::Key {
            virtual_key,
            release,
            ..
        } = &events[0]
        {
            assert_eq!(*virtual_key, 0x41);
            assert!(!release);
        }
        if let MockOutputEvent::Key {
            virtual_key,
            release,
            ..
        } = &events[1]
        {
            assert_eq!(*virtual_key, 0x41);
            assert!(*release);
        }
        if let MockOutputEvent::Key {
            virtual_key,
            release,
            ..
        } = &events[2]
        {
            assert_eq!(*virtual_key, 0x42);
            assert!(!release);
        }
    }
}

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
            use windows_sys::Win32::UI::Input::*;

            match action {
                SystemAction::VolumeUp => {
                    let mut input = unsafe { std::mem::zeroed::<INPUT>() };
                    input.type_ = INPUT_KEYBOARD;
                    let ki = unsafe { &mut input.u.ki_mut() };
                    ki.wVk = VK_VOLUME_UP as u16;
                    ki.wScan = 0;
                    ki.dwFlags = 0;
                    ki.time = 0;
                    ki.dwExtraInfo = 0;
                    unsafe {
                        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
                    }
                    ki.dwFlags = KEYEVENTF_KEYUP;
                    unsafe {
                        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
                    }
                }
                SystemAction::VolumeDown => {
                    let mut input = unsafe { std::mem::zeroed::<INPUT>() };
                    input.type_ = INPUT_KEYBOARD;
                    let ki = unsafe { &mut input.u.ki_mut() };
                    ki.wVk = VK_VOLUME_DOWN as u16;
                    ki.wScan = 0;
                    ki.dwFlags = 0;
                    ki.time = 0;
                    ki.dwExtraInfo = 0;
                    unsafe {
                        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
                    }
                    ki.dwFlags = KEYEVENTF_KEYUP;
                    unsafe {
                        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
                    }
                }
                SystemAction::Mute => {
                    let mut input = unsafe { std::mem::zeroed::<INPUT>() };
                    input.type_ = INPUT_KEYBOARD;
                    let ki = unsafe { &mut input.u.ki_mut() };
                    ki.wVk = VK_VOLUME_MUTE as u16;
                    ki.wScan = 0;
                    ki.dwFlags = 0;
                    ki.time = 0;
                    ki.dwExtraInfo = 0;
                    unsafe {
                        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
                    }
                    ki.dwFlags = KEYEVENTF_KEYUP;
                    unsafe {
                        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
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
