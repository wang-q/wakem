//! Platform abstraction traits
//!
//! This module defines the cross-platform interfaces that can be implemented
//! by each platform-specific module (Windows, macOS, Linux).

#[allow(unused_imports)]
use crate::platform::output_helpers::char_to_vk;
use crate::types::{
    InputEvent, KeyAction, ModifierState, MouseAction, MouseButton, SystemAction,
};
use anyhow::Result;

/// Input device configuration
#[derive(Debug, Clone)]
pub struct InputDeviceConfig {
    /// Capture keyboard events
    pub capture_keyboard: bool,
    /// Capture mouse events
    pub capture_mouse: bool,
    /// Block legacy input (platform-specific behavior)
    pub block_legacy_input: bool,
}

impl Default for InputDeviceConfig {
    fn default() -> Self {
        Self {
            capture_keyboard: true,
            capture_mouse: true,
            block_legacy_input: false,
        }
    }
}

/// Input device trait - for capturing keyboard and mouse events
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub type WindowId = usize;

/// Window information
#[allow(dead_code)]
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
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Monitor work area (usable screen area excluding taskbar/dock)
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct MonitorWorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl MonitorWorkArea {
    /// Create a new monitor work area
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Trait for window information needed by common operations
pub trait WindowInfoProvider {
    /// Get window position X
    fn x(&self) -> i32;
    /// Get window position Y
    fn y(&self) -> i32;
    /// Get window width
    fn width(&self) -> i32;
    /// Get window height
    fn height(&self) -> i32;
}

/// Window frame with position and size
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct WindowFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WindowFrame {
    /// Create a new window frame
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Calculate aspect ratio (width / height)
    pub fn aspect_ratio(&self) -> f64 {
        if self.height > 0 {
            self.width as f64 / self.height as f64
        } else {
            0.0
        }
    }

    /// Check if frame is valid (positive dimensions)
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Calculate centered position within a monitor
    pub fn center_in(&self, monitor: &MonitorInfo) -> (i32, i32) {
        let x = monitor.x + (monitor.width - self.width) / 2;
        let y = monitor.y + (monitor.height - self.height) / 2;
        (x, y)
    }
}

impl WindowInfoProvider for WindowInfo {
    fn x(&self) -> i32 {
        self.x
    }

    fn y(&self) -> i32 {
        self.y
    }

    fn width(&self) -> i32 {
        self.width
    }

    fn height(&self) -> i32 {
        self.height
    }
}

/// Window state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    /// Normal window state
    Normal,
    /// Window is minimized
    Minimized,
    /// Window is maximized
    Maximized,
    /// Window is in fullscreen mode
    FullScreen,
}

/// Window operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowOperation {
    /// Minimize window
    Minimize,
    /// Maximize window
    Maximize,
    /// Restore window
    Restore,
    /// Close window
    Close,
    /// Toggle topmost state
    ToggleTopmost,
}

/// Application commands sent from tray menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    /// Toggle active state
    ToggleActive,
    /// Reload configuration
    ReloadConfig,
    /// Open config folder
    OpenConfigFolder,
    /// Exit application
    Exit,
}

/// Menu action results from user interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// No action
    None,
    /// Toggle active state
    ToggleActive,
    /// Reload configuration
    Reload,
    /// Open config folder
    OpenConfig,
    /// Exit application
    Exit,
}

/// Window API trait - low-level window operations
///
/// # Deprecation Notice
///
/// This trait is deprecated in favor of [WindowManagerTrait], which provides
/// the identical method set. The two traits were created separately but have
/// converged to the same interface. New code should use [WindowManagerTrait].
#[deprecated(since = "0.1.2", note = "Use WindowManagerTrait instead")]
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Check if context matches the given conditions using wildcard matching.
    ///
    /// Supports glob-style patterns: `*` matches any sequence, `?` matches single char.
    pub fn matches(
        &self,
        process_name: Option<&str>,
        window_class: Option<&str>,
        window_title: Option<&str>,
        executable_path: Option<&str>,
    ) -> bool {
        use crate::config::wildcard_match;

        if let Some(pattern) = process_name {
            if !wildcard_match(&self.process_name, pattern) {
                return false;
            }
        }
        if let Some(pattern) = window_class {
            if !wildcard_match(&self.window_class, pattern) {
                return false;
            }
        }
        if let Some(pattern) = window_title {
            if !wildcard_match(&self.window_title, pattern) {
                return false;
            }
        }
        if let Some(pattern) = executable_path {
            match &self.executable_path {
                Some(path) if !wildcard_match(path, pattern) => return false,
                None => return false,
                _ => {}
            }
        }
        true
    }
}
