//! Platform abstraction traits
//!
//! This module defines the cross-platform interfaces that can be implemented
//! by each platform-specific module (Windows, macOS, Linux).

#[allow(unused_imports)]
use crate::platform::output_helpers::char_to_vk;
use crate::types::{InputEvent, KeyAction, ModifierState, MouseAction, MouseButton};
use anyhow::Result;

/// Input device configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InputDeviceConfig {
    pub capture_keyboard: bool,
    pub capture_mouse: bool,
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
    fn register(&mut self) -> Result<()>;
    fn unregister(&mut self);
    fn poll_event(&mut self) -> Option<InputEvent>;
    fn is_running(&self) -> bool;
    fn stop(&mut self);
}

/// Output device trait - for sending simulated input events
pub trait OutputDeviceTrait: Send {
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
        use crate::platform::output_helpers::modifier_vk;

        if modifiers.shift {
            self.send_key(0, modifier_vk::SHIFT, false)?;
        }
        if modifiers.ctrl {
            self.send_key(0, modifier_vk::CONTROL, false)?;
        }
        if modifiers.alt {
            self.send_key(0, modifier_vk::ALT, false)?;
        }
        if modifiers.meta {
            self.send_key(0, modifier_vk::META, false)?;
        }

        self.send_key(scan_code, virtual_key, false)?;
        self.send_key(scan_code, virtual_key, true)?;

        if modifiers.meta {
            self.send_key(0, modifier_vk::META, true)?;
        }
        if modifiers.alt {
            self.send_key(0, modifier_vk::ALT, true)?;
        }
        if modifiers.ctrl {
            self.send_key(0, modifier_vk::CONTROL, true)?;
        }
        if modifiers.shift {
            self.send_key(0, modifier_vk::SHIFT, true)?;
        }

        Ok(())
    }

    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()>;
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

    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()>;
    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()>;
    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()>;
}

/// Window identifier type (platform-specific)
pub type WindowId = usize;

/// Window information
#[derive(Debug, Clone)]
#[allow(dead_code)]
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

/// Monitor work area (usable screen area excluding taskbar/dock)
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct MonitorWorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl MonitorWorkArea {
    #[allow(dead_code)]
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
    fn x(&self) -> i32;
    fn y(&self) -> i32;
    fn width(&self) -> i32;
    fn height(&self) -> i32;
}

/// Window frame with position and size
#[derive(Debug, Clone, Copy)]
pub struct WindowFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WindowFrame {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn aspect_ratio(&self) -> f64 {
        if self.height > 0 {
            self.width as f64 / self.height as f64
        } else {
            0.0
        }
    }

    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }

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
#[allow(dead_code)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    FullScreen,
}

/// Window operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum WindowOperation {
    Minimize,
    Maximize,
    Restore,
    Close,
    ToggleTopmost,
}

/// Monitor direction for moving windows between displays
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MonitorDirection {
    Next,
    Prev,
    Index(i32),
}

/// Application commands sent from tray menu
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    ToggleActive,
    ReloadConfig,
    OpenConfigFolder,
    Exit,
}

/// Unified platform window event type
///
/// This enum provides a cross-platform representation of window events
/// that can be produced by both Windows (WinEventHook) and macOS
/// (CGWindowList polling) window event hooks.
///
/// Platform implementations that cannot detect certain events simply
/// won't emit them. For example, Windows currently only emits
/// [PlatformWindowEvent::Activated].
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PlatformWindowEvent {
    WindowActivated {
        process_name: String,
        window_title: String,
        window_id: usize,
    },
    WindowMinimized {
        process_name: String,
    },
    WindowRestored {
        process_name: String,
    },
    WindowMovedOrResized {
        process_name: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    WindowCreated {
        process_name: String,
        window_title: String,
    },
    WindowClosed {
        process_name: String,
    },
}

/// Menu action results from user interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MenuAction {
    None,
    ToggleActive,
    Reload,
    OpenConfig,
    Exit,
}

/// Base window API trait - shared operations across platforms
///
/// This trait defines the common window operations that both Windows and macOS
/// implement. Platform-specific traits (`MacosWindowApi`, `WindowApi`) extend
/// this with their own methods.
///
/// The associated type `WindowId` abstracts the platform-specific window
/// identifier (`HWND` on Windows, `CGWindowNumber` on macOS).
#[allow(dead_code)]
pub trait WindowApiBase {
    type WindowId: Copy + std::fmt::Debug;

    fn get_foreground_window(&self) -> Option<Self::WindowId>;
    fn get_window_info(&self, window: Self::WindowId) -> Result<WindowInfo>;
    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()>;
    fn minimize_window(&self, window: Self::WindowId) -> Result<()>;
    fn maximize_window(&self, window: Self::WindowId) -> Result<()>;
    fn restore_window(&self, window: Self::WindowId) -> Result<()>;
    fn close_window(&self, window: Self::WindowId) -> Result<()>;
    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()>;
    fn is_topmost(&self, window: Self::WindowId) -> bool;
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    fn move_to_monitor(
        &self,
        window: Self::WindowId,
        monitor_index: usize,
    ) -> Result<()>;
    fn is_window_valid(&self, window: Self::WindowId) -> bool;
    fn is_minimized(&self, window: Self::WindowId) -> bool;
    fn is_maximized(&self, window: Self::WindowId) -> bool;

    /// Ensure window is restored (not minimized or maximized)
    fn ensure_window_restored(&self, window: Self::WindowId) -> Result<()> {
        if self.is_minimized(window) || self.is_maximized(window) {
            self.restore_window(window)?;
        }
        Ok(())
    }
}

/// Window manager trait - high-level window operations
#[allow(dead_code)]
pub trait WindowManagerTrait: Send + Sync {
    fn get_foreground_window(&self) -> Option<WindowId>;
    fn get_window_info(&self, window: WindowId) -> Result<WindowInfo>;
    fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()>;
    fn minimize_window(&self, window: WindowId) -> Result<()>;
    fn maximize_window(&self, window: WindowId) -> Result<()>;
    fn restore_window(&self, window: WindowId) -> Result<()>;
    fn close_window(&self, window: WindowId) -> Result<()>;
    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()>;
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()>;
    fn is_window_valid(&self, window: WindowId) -> bool;
    fn is_minimized(&self, window: WindowId) -> bool;
    fn is_maximized(&self, window: WindowId) -> bool;
}

/// Platform utility functions trait
///
/// Provides common platform operations that are implemented differently
/// on each platform (Windows, macOS).
pub trait PlatformUtilities {
    /// Get current modifier state
    fn get_modifier_state() -> ModifierState;

    /// Get process name by PID
    #[allow(dead_code)]
    fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String>;

    /// Get executable path by PID
    #[allow(dead_code)]
    fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String>;
}

/// Window context information (for context-aware mappings)
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct WindowContext {
    pub process_name: String,
    pub window_class: String,
    pub window_title: String,
    pub executable_path: Option<String>,
}

impl WindowContext {
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
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

/// Macro to implement [WindowApiBase] by delegating to a platform-specific trait.
///
/// This eliminates the boilerplate of writing forwarding implementations
/// for each platform's API struct where method names match 1:1.
///
/// # Usage
///
/// ```ignore
/// impl_window_api_base_via!(RealMacosWindowApi, MacosWindowApi, WindowId);
/// ```
#[macro_export]
macro_rules! impl_window_api_base_via {
    (
        $(#[$meta:meta])*
        $impl_type:ty, $inner_trait:ty, $window_id:ty $(,)?
    ) => {
        $(#[$meta])*
        impl $crate::platform::traits::WindowApiBase for $impl_type {
            type WindowId = $window_id;

            fn get_foreground_window(&self) -> Option<Self::WindowId> {
                <$inner_trait>::get_foreground_window(self)
            }

            fn get_window_info(&self, window: Self::WindowId) -> ::anyhow::Result<$crate::platform::traits::WindowInfo> {
                <$inner_trait>::get_window_info(self, window)
            }

            fn set_window_pos(
                &self,
                window: Self::WindowId,
                x: i32,
                y: i32,
                width: i32,
                height: i32,
            ) -> ::anyhow::Result<()> {
                <$inner_trait>::set_window_pos(self, window, x, y, width, height)
            }

            fn minimize_window(&self, window: Self::WindowId) -> ::anyhow::Result<()> {
                <$inner_trait>::minimize_window(self, window)
            }

            fn maximize_window(&self, window: Self::WindowId) -> ::anyhow::Result<()> {
                <$inner_trait>::maximize_window(self, window)
            }

            fn restore_window(&self, window: Self::WindowId) -> ::anyhow::Result<()> {
                <$inner_trait>::restore_window(self, window)
            }

            fn close_window(&self, window: Self::WindowId) -> ::anyhow::Result<()> {
                <$inner_trait>::close_window(self, window)
            }

            fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> ::anyhow::Result<()> {
                <$inner_trait>::set_topmost(self, window, topmost)
            }

            fn is_topmost(&self, window: Self::WindowId) -> bool {
                <$inner_trait>::is_topmost(self, window)
            }

            fn get_monitors(&self) -> Vec<$crate::platform::traits::MonitorInfo> {
                <$inner_trait>::get_monitors(self)
            }

            fn move_to_monitor(&self, window: Self::WindowId, monitor_index: usize) -> ::anyhow::Result<()> {
                <$inner_trait>::move_to_monitor(self, window, monitor_index)
            }

            fn is_window_valid(&self, window: Self::WindowId) -> bool {
                <$inner_trait>::is_window_valid(self, window)
            }

            fn is_minimized(&self, window: Self::WindowId) -> bool {
                <$inner_trait>::is_minimized(self, window)
            }

            fn is_maximized(&self, window: Self::WindowId) -> bool {
                <$inner_trait>::is_maximized(self, window)
            }
        }
    };
}
