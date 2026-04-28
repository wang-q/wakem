//! Platform abstraction traits
//!
//! This module defines the cross-platform interfaces that can be implemented
//! by each platform-specific module (Windows, macOS, Linux).
//!
//! Many trait methods and types here are used via dynamic dispatch (dyn Trait)
//! or only on one platform. The dead_code lint is suppressed at module level
//! because individual #[allow] annotations would be too verbose.
#![allow(dead_code)]
//!
//! Note: Some trait methods and struct fields may appear unused on certain
//! platforms but are required for cross-platform API completeness.

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
                self.send_combo(modifiers, key.scan_code, key.virtual_key)
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
/// Monitor work area (excludes taskbar/Dock). Used on macOS; Windows uses MonitorInfo directly.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct MonitorWorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
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

    pub fn aspect_ratio(&self) -> f64 {
        if self.height > 0 {
            self.width as f64 / self.height as f64
        } else {
            0.0
        }
    }

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
#[allow(dead_code)]
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

/// Basic window operations trait
///
/// Defines the fundamental window manipulation operations that work
/// across all supported platforms.
pub trait WindowOperations: Send + Sync {
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
}

/// Window state query operations
///
/// Provides methods to query the current state of windows.
pub trait WindowStateQueries: Send + Sync {
    fn is_window_valid(&self, window: WindowId) -> bool;
    fn is_minimized(&self, window: WindowId) -> bool;
    fn is_maximized(&self, window: WindowId) -> bool;
    fn is_topmost(&self, window: WindowId) -> bool;
}

/// Monitor operations trait
///
/// Defines operations related to display monitors.
pub trait MonitorOperations: Send + Sync {
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()>;
}

/// Foreground window operations
///
/// Operations related to the currently focused/foreground window.
pub trait ForegroundWindowOperations: Send + Sync {
    fn get_foreground_window(&self) -> Option<WindowId>;
    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()>;
}

/// Window manager trait - combines all window-related operations
///
/// This is a composite trait that combines all window management
/// capabilities. Implementors should implement the component traits
/// and this trait will be automatically satisfied.
pub trait WindowManagerTrait:
    WindowOperations
    + WindowStateQueries
    + MonitorOperations
    + ForegroundWindowOperations
    + Send
    + Sync
{
}

/// Find the monitor that contains the given point, falling back to the first monitor.
///
/// This function searches through the list of monitors and returns the one
/// that contains the specified point (x, y). If no monitor contains the point,
/// it returns the first monitor in the list as a fallback.
pub fn find_monitor_for_point(
    monitors: &[MonitorInfo],
    x: i32,
    y: i32,
) -> Option<&MonitorInfo> {
    monitors
        .iter()
        .find(|m| x >= m.x && x < m.x + m.width && y >= m.y && y < m.y + m.height)
        .or_else(|| monitors.first())
}

/// Find the monitor index containing a point, returning the index.
pub fn find_monitor_index_for_point(monitors: &[MonitorInfo], x: i32, y: i32) -> usize {
    monitors
        .iter()
        .position(|m| x >= m.x && x < m.x + m.width && y >= m.y && y < m.y + m.height)
        .unwrap_or(0)
}

/// Extension trait providing high-level window management operations
///
/// These methods combine basic operations to provide convenient
/// high-level functionality like centering windows, moving to edges,
/// and resizing with alignment.
pub trait WindowManagerExt:
    WindowOperations + WindowStateQueries + MonitorOperations + ForegroundWindowOperations
{
    /// Get information about the currently focused window
    fn get_foreground_window_info(&self) -> Result<WindowInfo> {
        let window = self
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window found"))?;
        self.get_window_info(window)
    }

    /// Move window to center of its current monitor
    fn move_to_center(&self, window: WindowId) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let new_x = monitor.x + (monitor.width - info.width) / 2;
        let new_y = monitor.y + (monitor.height - info.height) / 2;
        self.set_window_pos(window, new_x, new_y, info.width, info.height)
    }

    /// Move window to edge of its current monitor
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

    /// Resize window to half screen on specified edge
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

    /// Cycle window width through predefined ratios
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

    /// Cycle window height through predefined ratios
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

    /// Set window to fixed aspect ratio with scaling
    ///
    /// # Arguments
    /// * `window` - The window to resize
    /// * `ratio` - The aspect ratio (width / height)
    /// * `scale_index` - Index into the scale array [1.0, 0.9, 0.7, 0.5].
    ///   If None, cycles through scales based on current window size.
    fn set_fixed_ratio(
        &self,
        window: WindowId,
        ratio: f32,
        scale_index: Option<usize>,
    ) -> Result<()> {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let base_size = std::cmp::min(monitor.width, monitor.height);
        let base_width = (base_size as f32 * ratio) as i32;
        let base_height = base_size;

        // Determine which scale to use
        let next_scale = match scale_index {
            Some(idx) if idx < SCALES.len() => SCALES[idx],
            Some(idx) => {
                anyhow::bail!(
                    "Scale index {} out of range (0-{})",
                    idx,
                    SCALES.len() - 1
                );
            }
            None => {
                // Auto-detect next scale based on current window size
                let current_scale = (info.width as f32 / base_width as f32
                    + info.height as f32 / base_height as f32)
                    / 2.0;
                let mut next = SCALES[0];
                for (i, scale) in SCALES.iter().enumerate() {
                    if (current_scale - scale).abs() < 0.05 {
                        next = SCALES[(i + 1) % SCALES.len()];
                        break;
                    }
                }
                next
            }
        };

        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;
        let new_x = monitor.x + (monitor.width - new_width) / 2;
        let new_y = monitor.y + (monitor.height - new_height) / 2;
        self.set_window_pos(window, new_x, new_y, new_width, new_height)
    }

    /// Set window to native monitor aspect ratio
    ///
    /// # Arguments
    /// * `window` - The window to resize
    /// * `scale_index` - Index into the scale array [1.0, 0.9, 0.7, 0.5].
    ///   If None, cycles through scales based on current window size.
    fn set_native_ratio(
        &self,
        window: WindowId,
        scale_index: Option<usize>,
    ) -> Result<()> {
        let monitors = self.get_monitors();
        let info = self.get_window_info(window)?;
        let monitor = find_monitor_for_point(&monitors, info.x, info.y)
            .ok_or_else(|| anyhow::anyhow!("No monitors found"))?;
        let ratio = monitor.width as f32 / monitor.height as f32;
        self.set_fixed_ratio(window, ratio, scale_index)
    }

    /// Toggle window topmost state
    fn toggle_topmost(&self, window: WindowId) -> Result<bool> {
        let current = self.is_topmost(window);
        let new_state = !current;
        self.set_topmost(window, new_state)?;
        Ok(new_state)
    }
}

impl<
        T: ?Sized
            + WindowOperations
            + WindowStateQueries
            + MonitorOperations
            + ForegroundWindowOperations,
    > WindowManagerExt for T
{
}

/// Platform utility functions trait
///
/// Provides common platform operations that are implemented differently
/// on each platform (Windows, macOS).
pub trait PlatformUtilities {
    /// Get current modifier state
    fn get_modifier_state() -> ModifierState;

    /// Get process name by PID
    fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String>;

    /// Get executable path by PID
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
///
/// Associated types allow compile-time type safety while maintaining
/// platform abstraction.
pub trait PlatformFactory {
    type InputDevice: InputDeviceTrait;
    type OutputDevice: OutputDeviceTrait + Send + Sync;
    type WindowManager: WindowManagerTrait;
    type WindowPresetManager: WindowPresetManagerTrait;
    type NotificationService: NotificationService;
    type Launcher: LauncherTrait;
    type WindowEventHook: WindowEventHookTrait;

    fn create_input_device(
        config: InputDeviceConfig,
        sender: Option<std::sync::mpsc::Sender<InputEvent>>,
    ) -> Result<Self::InputDevice>;

    fn create_output_device() -> Self::OutputDevice;

    fn create_window_manager() -> Self::WindowManager;

    fn create_window_preset_manager() -> Self::WindowPresetManager;

    fn create_notification_service() -> Self::NotificationService;

    fn create_launcher() -> Self::Launcher;

    fn create_window_event_hook(
        sender: std::sync::mpsc::Sender<PlatformWindowEvent>,
    ) -> Self::WindowEventHook;
}

/// Macro to implement `PlatformFactory` with the six boilerplate factory methods
/// that are identical across platforms. Only the associated types differ.
#[macro_export]
macro_rules! impl_platform_factory_methods {
    ($platform:ty, $input:ty, $output:ty, $wm:ty, $wpm:ty, $notif:ty,
     $launcher:ty, $hook:ty) => {
        fn create_input_device(
            _config: $crate::platform::traits::InputDeviceConfig,
            sender: Option<std::sync::mpsc::Sender<$crate::types::InputEvent>>,
        ) -> anyhow::Result<Self::InputDevice> {
            match sender {
                Some(tx) => <$input>::with_sender(tx),
                None => {
                    <$input>::new($crate::platform::traits::InputDeviceConfig::default())
                }
            }
        }

        fn create_output_device() -> Self::OutputDevice {
            <$output>::new()
        }

        fn create_window_manager() -> Self::WindowManager {
            <$wm>::new()
        }

        fn create_window_preset_manager() -> Self::WindowPresetManager {
            <$wpm>::new(<$wm>::new())
        }

        fn create_notification_service() -> Self::NotificationService {
            <$notif>::new()
        }

        fn create_launcher() -> Self::Launcher {
            <$launcher>::new()
        }

        fn create_window_event_hook(
            sender: std::sync::mpsc::Sender<
                $crate::platform::traits::PlatformWindowEvent,
            >,
        ) -> Self::WindowEventHook {
            <$hook>::new(sender)
        }
    };
}

/// Macro to implement `WindowEventHookTrait` with the standard delegation pattern.
#[macro_export]
macro_rules! impl_window_event_hook_trait {
    ($hook:ty) => {
        fn start_with_shutdown(
            &mut self,
            shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        ) -> anyhow::Result<()> {
            self.start_with_shutdown(shutdown_flag)
        }

        fn stop(&mut self) {
            self.stop()
        }

        fn shutdown_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
            self.shutdown_flag()
        }
    };
}

/// Macro implementing the boilerplate `WindowApiBase` trait delegation methods
/// that trivially forward to identically-named inherent methods. Eliminates ~50
/// lines of duplication across Windows and macOS `RealWindowApi`.
#[macro_export]
macro_rules! impl_window_api_base_delegations {
    () => {
        fn get_foreground_window(&self) -> Option<Self::WindowId> {
            self.get_foreground_window()
        }

        fn set_window_pos(
            &self,
            window: Self::WindowId,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        ) -> anyhow::Result<()> {
            self.set_window_pos(window, x, y, width, height)
        }

        fn minimize_window(&self, window: Self::WindowId) -> anyhow::Result<()> {
            self.minimize_window(window)
        }

        fn maximize_window(&self, window: Self::WindowId) -> anyhow::Result<()> {
            self.maximize_window(window)
        }

        fn restore_window(&self, window: Self::WindowId) -> anyhow::Result<()> {
            self.restore_window(window)
        }

        fn close_window(&self, window: Self::WindowId) -> anyhow::Result<()> {
            self.close_window(window)
        }

        fn set_topmost(
            &self,
            window: Self::WindowId,
            topmost: bool,
        ) -> anyhow::Result<()> {
            self.set_topmost(window, topmost)
        }

        fn is_topmost(&self, window: Self::WindowId) -> bool {
            self.is_topmost(window)
        }
    };
}

/// Window context information (for context-aware mappings)
#[derive(Debug, Clone, Default)]
pub struct WindowContext {
    pub process_name: String,
    pub window_class: String,
    pub window_title: String,
    pub executable_path: Option<String>,
}

/// Criteria for matching window context.
///
/// All fields are optional. A field of `None` means "match any value" (no restriction).
/// A field of `Some(pattern)` means the corresponding window property must match
/// the pattern (supports wildcards like `*` and `?`).
///
/// # Example
///
/// ```
/// use wakem::platform::traits::WindowMatchCriteria;
///
/// let criteria = WindowMatchCriteria {
///     process_name: Some("Chrome*".to_string()),
///     window_title: Some("*GitHub*".to_string()),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct WindowMatchCriteria {
    pub process_name: Option<String>,
    pub window_class: Option<String>,
    pub window_title: Option<String>,
    pub executable_path: Option<String>,
}

impl WindowContext {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Check if this context matches the given criteria.
    ///
    /// # Arguments
    ///
    /// * `criteria` - The matching criteria. Fields of `None` match any value.
    ///
    /// # Returns
    ///
    /// Returns `true` if all specified criteria match (or if no criteria are specified).
    pub fn matches_criteria(&self, criteria: &WindowMatchCriteria) -> bool {
        use crate::config::wildcard_match;

        if let Some(pattern) = &criteria.process_name {
            if !wildcard_match(&self.process_name, pattern) {
                return false;
            }
        }
        if let Some(pattern) = &criteria.window_class {
            if !wildcard_match(&self.window_class, pattern) {
                return false;
            }
        }
        if let Some(pattern) = &criteria.window_title {
            if !wildcard_match(&self.window_title, pattern) {
                return false;
            }
        }
        if let Some(pattern) = &criteria.executable_path {
            match &self.executable_path {
                Some(path) if !wildcard_match(path, pattern) => return false,
                None => return false,
                _ => {}
            }
        }
        true
    }

    /// Check if this context matches the given patterns (legacy API).
    ///
    /// # Arguments
    ///
    /// * `process_name` - Optional pattern to match against process name
    /// * `window_class` - Optional pattern to match against window class
    /// * `window_title` - Optional pattern to match against window title
    /// * `executable_path` - Optional pattern to match against executable path
    ///
    /// # Returns
    ///
    /// Returns `true` if all specified patterns match. A pattern of `None` means
    /// "match any value". If all patterns are `None`, returns `true` (matches any window).
    ///
    /// # Deprecated
    ///
    /// Consider using [`matches_criteria`](Self::matches_criteria) with [`WindowMatchCriteria`] instead.
    pub fn matches(
        &self,
        process_name: Option<&str>,
        window_class: Option<&str>,
        window_title: Option<&str>,
        executable_path: Option<&str>,
    ) -> bool {
        let criteria = WindowMatchCriteria {
            process_name: process_name.map(|s| s.to_string()),
            window_class: window_class.map(|s| s.to_string()),
            window_title: window_title.map(|s| s.to_string()),
            executable_path: executable_path.map(|s| s.to_string()),
        };
        self.matches_criteria(&criteria)
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
