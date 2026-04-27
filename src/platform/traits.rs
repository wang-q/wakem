//! Platform abstraction traits
//!
//! This module defines the cross-platform interfaces that can be implemented
//! by each platform-specific module (Windows, macOS, Linux).
#![allow(dead_code)]

use crate::platform::output_helpers::char_to_vk;
use crate::types::{InputEvent, KeyAction, ModifierState, MouseAction, MouseButton};
use anyhow::Result;
use std::sync::Arc;

/// Input device configuration
#[derive(Debug, Clone)]
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

/// Monitor direction for moving windows between displays
#[derive(Debug, Clone, Copy)]
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
pub trait WindowApiBase {
    type WindowId: Copy + std::fmt::Debug + 'static;

    fn window_id_to_usize(id: Self::WindowId) -> usize;
    fn usize_to_window_id(id: usize) -> Self::WindowId;

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
    fn is_topmost(&self, window: WindowId) -> bool;
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()>;
    fn is_window_valid(&self, window: WindowId) -> bool;
    fn is_minimized(&self, window: WindowId) -> bool;
    fn is_maximized(&self, window: WindowId) -> bool;
}

fn find_monitor_for_point(
    monitors: &[MonitorInfo],
    x: i32,
    y: i32,
) -> Option<&MonitorInfo> {
    monitors
        .iter()
        .find(|m| x >= m.x && x < m.x + m.width && y >= m.y && y < m.y + m.height)
        .or_else(|| monitors.first())
}

pub trait WindowManagerExt: WindowManagerTrait {
    fn move_to_center(&self, window: WindowId) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let new_x = monitor.x + (monitor.width - info.width) / 2;
        let new_y = monitor.y + (monitor.height - info.height) / 2;
        self.set_window_pos(window, new_x, new_y, info.width, info.height)
    }

    fn move_to_edge(&self, window: WindowId, edge: crate::types::Edge) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let (new_x, new_y) = match edge {
            crate::types::Edge::Left => (monitor.x, info.y),
            crate::types::Edge::Right => {
                (monitor.x + monitor.width - info.width, info.y)
            }
            crate::types::Edge::Top => (info.x, monitor.y),
            crate::types::Edge::Bottom => {
                (info.x, monitor.y + monitor.height - info.height)
            }
        };
        self.set_window_pos(window, new_x, new_y, info.width, info.height)
    }

    fn set_half_screen(&self, window: WindowId, edge: crate::types::Edge) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let (new_x, new_y, new_width, new_height) = match edge {
            crate::types::Edge::Left => {
                (monitor.x, monitor.y, monitor.width / 2, monitor.height)
            }
            crate::types::Edge::Right => {
                let w = monitor.width / 2;
                (monitor.x + monitor.width - w, monitor.y, w, monitor.height)
            }
            crate::types::Edge::Top => {
                (monitor.x, monitor.y, monitor.width, monitor.height / 2)
            }
            crate::types::Edge::Bottom => {
                let h = monitor.height / 2;
                (monitor.x, monitor.y + monitor.height - h, monitor.width, h)
            }
        };
        self.set_window_pos(window, new_x, new_y, new_width, new_height)
    }

    fn loop_width(
        &self,
        window: WindowId,
        align: crate::types::Alignment,
    ) -> Result<()> {
        const WIDTH_RATIOS: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let current_ratio = info.width as f32 / monitor.width as f32;
        let mut next_ratio = WIDTH_RATIOS[0];
        for (i, ratio) in WIDTH_RATIOS.iter().enumerate() {
            if (current_ratio - ratio).abs() < 0.01 {
                next_ratio = WIDTH_RATIOS[(i + 1) % WIDTH_RATIOS.len()];
                break;
            }
        }
        let new_width = (monitor.width as f32 * next_ratio) as i32;
        let new_x = match align {
            crate::types::Alignment::Left => monitor.x,
            crate::types::Alignment::Right => monitor.x + monitor.width - new_width,
            _ => info.x,
        };
        self.set_window_pos(window, new_x, info.y, new_width, info.height)
    }

    fn loop_height(
        &self,
        window: WindowId,
        align: crate::types::Alignment,
    ) -> Result<()> {
        const HEIGHT_RATIOS: [f32; 3] = [0.75, 0.5, 0.25];
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let current_ratio = info.height as f32 / monitor.height as f32;
        let mut next_ratio = HEIGHT_RATIOS[0];
        for (i, ratio) in HEIGHT_RATIOS.iter().enumerate() {
            if (current_ratio - ratio).abs() < 0.01 {
                next_ratio = HEIGHT_RATIOS[(i + 1) % HEIGHT_RATIOS.len()];
                break;
            }
        }
        let new_height = (monitor.height as f32 * next_ratio) as i32;
        let new_y = match align {
            crate::types::Alignment::Top => monitor.y,
            crate::types::Alignment::Bottom => monitor.y + monitor.height - new_height,
            _ => info.y,
        };
        self.set_window_pos(window, info.x, new_y, info.width, new_height)
    }

    fn set_fixed_ratio(&self, window: WindowId, ratio: f32) -> Result<()> {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let base_size = std::cmp::min(monitor.width, monitor.height);
        let base_width = (base_size as f32 * ratio) as i32;
        let base_height = base_size;
        let current_scale = (info.width as f32 / base_width as f32
            + info.height as f32 / base_height as f32)
            / 2.0;
        let mut next_scale = SCALES[0];
        for (i, scale) in SCALES.iter().enumerate() {
            if (current_scale - scale).abs() < 0.05 {
                next_scale = SCALES[(i + 1) % SCALES.len()];
                break;
            }
        }
        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;
        let new_x = monitor.x + (monitor.width - new_width) / 2;
        let new_y = monitor.y + (monitor.height - new_height) / 2;
        self.set_window_pos(window, new_x, new_y, new_width, new_height)
    }

    fn set_native_ratio(&self, window: WindowId) -> Result<()> {
        let monitors = self.get_monitors();
        let info = self.get_window_info(window)?;
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let ratio = monitor.width as f32 / monitor.height as f32;
        self.set_fixed_ratio(window, ratio)
    }

    fn toggle_topmost(&self, window: WindowId) -> Result<bool> {
        let current = self.is_topmost(window);
        let new_state = !current;
        self.set_topmost(window, new_state)?;
        Ok(new_state)
    }
}

impl<T: ?Sized + WindowManagerTrait> WindowManagerExt for T {}

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

/// Trait for providing current window context information
///
/// This trait abstracts the platform-specific implementation of obtaining
/// the current foreground window's context (process name, window title, etc.).
/// It enables core logic to access window context without direct platform API calls.
pub trait ContextProvider {
    /// Get the current foreground window context
    ///
    /// Returns `None` if no window is in foreground or if the information
    /// cannot be obtained (e.g., insufficient permissions on macOS).
    fn get_current_context() -> Option<WindowContext>;
}

/// Trait for showing desktop notifications
///
/// This trait provides a cross-platform abstraction for displaying
/// system notifications (toast notifications on Windows, notification center
/// on macOS). Implementations should handle platform-specific details internally.
pub trait NotificationService: Send + Sync {
    /// Show a notification with the given title and message
    ///
    /// Returns `Ok(())` if the notification was shown successfully,
    /// or an error if the notification could not be displayed.
    fn show(&self, title: &str, message: &str) -> Result<()>;

    /// Initialize the notification service with platform-specific context
    ///
    /// The [NotificationInitContext] carries opaque platform data that
    /// individual implementations interpret as needed. Callers should
    /// obtain the context from the platform layer and pass it through
    /// without inspecting its contents.
    fn initialize(&self, _ctx: &NotificationInitContext) {}
}

/// Platform-agnostic initialization context for [NotificationService].
///
/// Carries opaque native handles that platform implementations need
/// during initialization. The `native_handle` field stores a platform-specific
/// window or message handle (e.g., HWND on Windows) as an opaque integer;
/// non-Windows platforms typically receive `None`.
pub struct NotificationInitContext {
    #[allow(dead_code)]
    pub native_handle: Option<usize>,
}

/// Trait for window preset management
///
/// Provides a cross-platform abstraction for managing window presets
/// (saving, loading, and auto-applying window positions/sizes).
pub trait WindowPresetManagerTrait: Send + Sync {
    fn load_presets(&mut self, presets: Vec<crate::config::WindowPreset>);
    fn save_preset(&mut self, name: String) -> Result<()>;
    fn load_preset(&self, name: &str) -> Result<()>;
    fn get_foreground_window_info(&self) -> Option<Result<WindowInfo>>;
    fn apply_preset_for_window_by_id(&self, window_id: WindowId) -> Result<bool>;
    #[allow(dead_code)]
    fn apply_preset_for_window(&self) -> Result<bool>;
}

/// Trait for window event hook
///
/// Provides a cross-platform abstraction for monitoring window events
/// such as foreground window changes. Windows uses SetWinEventHook,
/// macOS uses CGWindowList polling.
pub trait WindowEventHookTrait: Send {
    fn start_with_shutdown(
        &mut self,
        shutdown_flag: Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()>;
    fn stop(&mut self);
    fn shutdown_flag(&self) -> Arc<std::sync::atomic::AtomicBool>;
}

/// Trait for program launcher
///
/// Provides a cross-platform abstraction for launching programs
/// and opening files/folders.
pub trait LauncherTrait: Send {
    fn launch(&self, action: &crate::types::LaunchAction) -> Result<()>;
}

/// Trait for tray lifecycle management
///
/// Provides a cross-platform abstraction for running the system tray
/// message loop and stopping it.
pub trait TrayLifecycle {
    fn run_tray_message_loop(callback: Box<dyn Fn(AppCommand) + Send>) -> Result<()>;
    fn stop_tray();
}

/// Trait for application control
///
/// Provides a cross-platform abstraction for application lifecycle
/// operations that differ between platforms.
pub trait ApplicationControl {
    fn detach_console();
    fn terminate_application();
    fn open_folder(path: &std::path::Path) -> Result<()>;
    fn force_kill_instance(instance_id: u32) -> Result<()>;
}

/// Factory trait for creating platform-specific objects
///
/// Centralizes all platform-specific object creation so that
/// non-platform code never needs conditional compilation.
pub trait PlatformFactory {
    fn create_input_device(
        config: InputDeviceConfig,
        sender: Option<std::sync::mpsc::Sender<InputEvent>>,
    ) -> Result<Box<dyn InputDeviceTrait>>;

    fn create_output_device() -> Box<dyn OutputDeviceTrait + Send + Sync>;

    fn create_window_manager() -> Box<dyn WindowManagerTrait>;

    fn create_window_preset_manager() -> Box<dyn WindowPresetManagerTrait>;

    fn create_notification_service() -> Box<dyn NotificationService>;

    fn create_launcher() -> Box<dyn LauncherTrait>;

    fn create_window_event_hook(
        sender: std::sync::mpsc::Sender<PlatformWindowEvent>,
    ) -> Box<dyn WindowEventHookTrait>;
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
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self::default()
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_context_empty() {
        let ctx = WindowContext::empty();
        assert!(ctx.process_name.is_empty());
        assert!(ctx.window_class.is_empty());
        assert!(ctx.window_title.is_empty());
        assert!(ctx.executable_path.is_none());
    }
}
