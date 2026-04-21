//! Platform abstraction traits
//!
//! This module defines the cross-platform interfaces that can be implemented
//! by each platform-specific module (Windows, macOS, Linux).

use crate::platform::output_helpers::char_to_vk;
use crate::types::{
    InputEvent, KeyAction, ModifierState, MouseAction, MouseButton, SystemAction,
};
use anyhow::Result;

/// Input device trait - for capturing keyboard and mouse events
pub trait InputDeviceTrait: Send {
    /// Register the device and start capturing events
    fn register(&mut self) -> Result<()>;

    /// Unregister the device and stop capturing
    fn unregister(&mut self);

    /// Poll for the next input event (non-blocking)
    fn poll_event(&mut self) -> Option<InputEvent>;

    /// Check if the device is currently running
    fn is_running(&self) -> bool;

    /// Stop the device
    fn stop(&mut self);
}

/// Output device trait - for sending simulated input events
pub trait OutputDeviceTrait: Send {
    /// Send a key action
    fn send_key_action(&self, action: &KeyAction) -> Result<()> {
        match action {
            KeyAction::Press {
                scan_code,
                virtual_key,
            } => self.send_key(*scan_code, *virtual_key, false),
            KeyAction::Release {
                scan_code,
                virtual_key,
            } => self.send_key(*scan_code, *virtual_key, true),
            KeyAction::Click {
                scan_code,
                virtual_key,
            } => {
                self.send_key(*scan_code, *virtual_key, false)?;
                self.send_key(*scan_code, *virtual_key, true)
            }
            KeyAction::TypeText(text) => self.send_text(text),
            KeyAction::Combo { modifiers, key } => {
                self.send_combo(modifiers, key.0, key.1)
            }
            KeyAction::None => Ok(()),
        }
    }

    /// Send text by typing each character
    fn send_text(&self, text: &str) -> Result<()> {
        for ch in text.chars() {
            if let Some(vk) = char_to_vk(ch) {
                self.send_key(0, vk, false)?;
                self.send_key(0, vk, true)?;
            }
        }
        Ok(())
    }

    /// Send key combination with modifiers
    ///
    /// Sequence: press modifiers → press target → release target → release modifiers (reverse order).
    fn send_combo(
        &self,
        modifiers: &ModifierState,
        scan_code: u16,
        virtual_key: u16,
    ) -> Result<()> {
        if modifiers.shift {
            self.send_key(0, 0x10, false)?;
        }
        if modifiers.ctrl {
            self.send_key(0, 0x11, false)?;
        }
        if modifiers.alt {
            self.send_key(0, 0x12, false)?;
        }
        if modifiers.meta {
            self.send_key(0, 0x5B, false)?;
        }

        self.send_key(scan_code, virtual_key, false)?;
        self.send_key(scan_code, virtual_key, true)?;

        if modifiers.meta {
            self.send_key(0, 0x5B, true)?;
        }
        if modifiers.alt {
            self.send_key(0, 0x12, true)?;
        }
        if modifiers.ctrl {
            self.send_key(0, 0x11, true)?;
        }
        if modifiers.shift {
            self.send_key(0, 0x10, true)?;
        }

        Ok(())
    }

    /// Send a single key event
    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()>;

    /// Send a mouse action
    fn send_mouse_action(&self, action: &MouseAction) -> Result<()> {
        match action {
            MouseAction::Move { x, y, relative } => {
                self.send_mouse_move(*x, *y, *relative)
            }
            MouseAction::ButtonDown { button } => self.send_mouse_button(*button, false),
            MouseAction::ButtonUp { button } => self.send_mouse_button(*button, true),
            MouseAction::ButtonClick { button } => {
                self.send_mouse_button(*button, false)?;
                self.send_mouse_button(*button, true)
            }
            MouseAction::Wheel { delta } => self.send_mouse_wheel(*delta, false),
            MouseAction::HWheel { delta } => self.send_mouse_wheel(*delta, true),
            MouseAction::None => Ok(()),
        }
    }

    /// Send mouse move
    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()>;

    /// Send mouse button
    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()>;

    /// Send mouse wheel
    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()>;

    /// Send system action (volume, brightness, etc.)
    fn send_system_action(&self, action: &SystemAction) -> Result<()>;
}

/// Window identifier type (platform-specific)
pub type WindowId = usize;

/// Window information
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub id: WindowId,
    pub title: String,
    pub process_name: String,
    pub executable_path: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Monitor information
#[derive(Debug, Clone, Copy)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Window API trait - low-level window operations
pub trait WindowApiTrait: Send + Sync {
    /// Get the currently focused window
    fn get_foreground_window(&self) -> Option<WindowId>;

    /// Get window information
    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo>;

    /// Set window position and size
    fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()>;

    /// Minimize window
    fn minimize_window(&self, window: WindowId) -> Result<()>;

    /// Maximize window
    fn maximize_window(&self, window: WindowId) -> Result<()>;

    /// Restore window
    fn restore_window(&self, window: WindowId) -> Result<()>;

    /// Close window
    fn close_window(&self, window: WindowId) -> Result<()>;

    /// Set window topmost state
    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()>;

    /// Set window opacity
    fn set_opacity(&self, window: WindowId, opacity: u8) -> Result<()>;

    /// Get all monitors
    fn get_monitors(&self) -> Vec<MonitorInfo>;

    /// Move window to another monitor
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()>;

    /// Check if window is valid
    fn is_window_valid(&self, window: WindowId) -> bool;

    /// Check if window is minimized
    fn is_minimized(&self, window: WindowId) -> bool;

    /// Check if window is maximized
    fn is_maximized(&self, window: WindowId) -> bool;
}

/// Window manager trait - high-level window operations
pub trait WindowManagerTrait: Send + Sync {
    /// Get the currently focused window
    fn get_foreground_window(&self) -> Option<WindowId>;

    /// Get window information
    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo>;

    /// Set window position and size
    fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()>;

    /// Minimize window
    fn minimize_window(&self, window: WindowId) -> Result<()>;

    /// Maximize window
    fn maximize_window(&self, window: WindowId) -> Result<()>;

    /// Restore window
    fn restore_window(&self, window: WindowId) -> Result<()>;

    /// Close window
    fn close_window(&self, window: WindowId) -> Result<()>;

    /// Set window topmost state
    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()>;

    /// Set window opacity
    fn set_opacity(&self, window: WindowId, opacity: u8) -> Result<()>;

    /// Get all monitors
    fn get_monitors(&self) -> Vec<MonitorInfo>;

    /// Move window to another monitor
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()>;

    /// Check if window is valid
    fn is_window_valid(&self, window: WindowId) -> bool;

    /// Check if window is minimized
    fn is_minimized(&self, window: WindowId) -> bool;

    /// Check if window is maximized
    fn is_maximized(&self, window: WindowId) -> bool;
}

/// Tray icon trait - for system tray integration
pub trait TrayIconTrait: Send {
    /// Show the tray icon
    fn show(&mut self) -> Result<()>;

    /// Hide the tray icon
    fn hide(&mut self) -> Result<()>;

    /// Show a notification
    fn show_notification(&mut self, title: &str, message: &str) -> Result<()>;

    /// Show context menu
    fn show_menu(&mut self) -> Result<()>;
}

/// Window context information (for context-aware mappings)
#[derive(Debug, Clone, Default)]
pub struct WindowContext {
    pub process_name: String,
    pub window_class: String,
    pub window_title: String,
    pub executable_path: Option<String>,
}

impl WindowContext {
    /// Create an empty context
    pub fn empty() -> Self {
        Self::default()
    }

    /// Check if context matches the given conditions
    pub fn matches(
        &self,
        process: Option<&str>,
        class: Option<&str>,
        title: Option<&str>,
    ) -> bool {
        if let Some(p) = process {
            if !self.process_name.to_lowercase().contains(&p.to_lowercase()) {
                return false;
            }
        }
        if let Some(c) = class {
            if !self.window_class.to_lowercase().contains(&c.to_lowercase()) {
                return false;
            }
        }
        if let Some(t) = title {
            if !self.window_title.to_lowercase().contains(&t.to_lowercase()) {
                return false;
            }
        }
        true
    }
}
