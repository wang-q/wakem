//! Platform abstraction traits
//!
//! This module defines the cross-platform interfaces implemented
//! by each platform-specific module (Windows, macOS).
//!
//! Shared data types are defined in [`super::types`] and re-exported here.

#[allow(unused_imports)]
use crate::platform::common::output_helpers::char_to_vk;
use crate::types::{InputEvent, KeyAction, ModifierState, MouseAction, MouseButton};
use anyhow::Result;
use std::sync::Arc;
use tracing::debug;

pub use super::types::*;

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
        use crate::platform::common::output_helpers::modifier_vk;

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

/// Base window API trait - shared operations across platforms
///
/// This trait defines the common window operations that both Windows and macOS
/// implement. Platform-specific traits extend this with their own methods.
pub trait WindowApiBase {
    type WindowId: Copy + std::fmt::Debug;

    fn get_foreground_window(&self) -> Option<Self::WindowId>;
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
    fn is_window_valid(&self, window: Self::WindowId) -> bool;
    fn is_minimized(&self, window: Self::WindowId) -> bool;
    fn is_maximized(&self, window: Self::WindowId) -> bool;
}

/// Basic window manipulation operations
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
    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()>;
}

/// Window state query operations
pub trait WindowStateQueries: Send + Sync {
    fn is_window_valid(&self, window: WindowId) -> bool;
    fn is_minimized(&self, window: WindowId) -> bool;
    fn is_maximized(&self, window: WindowId) -> bool;
    fn is_topmost(&self, window: WindowId) -> bool;
}

/// Monitor-related operations
pub trait MonitorOperations: Send + Sync {
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()>;
}

/// Foreground window operations
pub trait ForegroundWindowOperations: Send + Sync {
    fn get_foreground_window(&self) -> Option<WindowId>;
    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        debug!("SwitchToNextWindow: not implemented on this platform");
        Ok(())
    }
}

/// Window manager trait - composed of fine-grained operation traits
///
/// This is a marker trait that combines all window operation traits.
/// Implementors automatically satisfy this by implementing the
/// constituent traits.
pub trait WindowManagerTrait:
    WindowOperations
    + WindowStateQueries
    + MonitorOperations
    + ForegroundWindowOperations
    + WindowManagerExt
    + Send
    + Sync
{
}

/// Extended window manager operations with default implementations
///
/// Provides high-level window management operations (center, half-screen,
/// loop, fixed ratio, etc.) built on top of the basic
/// [`WindowManagerTrait`] operations.
pub trait WindowManagerExt:
    WindowOperations + WindowStateQueries + MonitorOperations + ForegroundWindowOperations
{
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

/// Find the monitor that contains the given point
pub fn find_monitor_for_point(
    monitors: &[MonitorInfo],
    x: i32,
    y: i32,
) -> Option<MonitorInfo> {
    for monitor in monitors {
        if x >= monitor.x
            && x < monitor.x + monitor.width
            && y >= monitor.y
            && y < monitor.y + monitor.height
        {
            return Some(*monitor);
        }
    }
    None
}

/// Platform utility functions trait
///
/// Provides common platform operations that are implemented differently
/// on each platform (Windows, macOS).
pub trait PlatformUtilities {
    /// Get current modifier state
    fn get_modifier_state() -> ModifierState;
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
    fn show(&self, title: &str, message: &str) -> Result<()>;

    /// Set the message window handle (Windows only, no-op on other platforms)
    fn set_message_window_handle(&self, _hwnd: isize) {}
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

/// Macro to implement [WindowApiBase] by delegating to a platform-specific trait.
#[macro_export]
macro_rules! impl_window_api_base_via {
    (
        $(#[$meta:meta])*
        $impl_type:ty, $inner_trait:path, $window_id:ty $(,)?
    ) => {
        $(#[$meta])*
        impl $crate::platform::traits::WindowApiBase for $impl_type {
            type WindowId = $window_id;

            fn get_foreground_window(&self) -> Option<Self::WindowId> {
                <$impl_type as $inner_trait>::get_foreground_window(self)
            }

            fn set_window_pos(
                &self,
                window: Self::WindowId,
                x: i32,
                y: i32,
                width: i32,
                height: i32,
            ) -> ::anyhow::Result<()> {
                <$impl_type as $inner_trait>::set_window_pos(self, window, x, y, width, height)
            }

            fn minimize_window(&self, window: Self::WindowId) -> ::anyhow::Result<()> {
                <$impl_type as $inner_trait>::minimize_window(self, window)
            }

            fn maximize_window(&self, window: Self::WindowId) -> ::anyhow::Result<()> {
                <$impl_type as $inner_trait>::maximize_window(self, window)
            }

            fn restore_window(&self, window: Self::WindowId) -> ::anyhow::Result<()> {
                <$impl_type as $inner_trait>::restore_window(self, window)
            }

            fn close_window(&self, window: Self::WindowId) -> ::anyhow::Result<()> {
                <$impl_type as $inner_trait>::close_window(self, window)
            }

            fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> ::anyhow::Result<()> {
                <$impl_type as $inner_trait>::set_topmost(self, window, topmost)
            }

            fn is_topmost(&self, window: Self::WindowId) -> bool {
                <$impl_type as $inner_trait>::is_topmost(self, window)
            }

            fn get_monitors(&self) -> Vec<$crate::platform::traits::MonitorInfo> {
                <$impl_type as $inner_trait>::get_monitors(self)
            }

            fn is_window_valid(&self, window: Self::WindowId) -> bool {
                <$impl_type as $inner_trait>::is_window_valid(self, window)
            }

            fn is_minimized(&self, window: Self::WindowId) -> bool {
                <$impl_type as $inner_trait>::is_minimized(self, window)
            }

            fn is_maximized(&self, window: Self::WindowId) -> bool {
                <$impl_type as $inner_trait>::is_maximized(self, window)
            }
        }
    };
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

    #[test]
    fn test_find_monitor_for_point() {
        let monitors = vec![
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            MonitorInfo {
                x: 1920,
                y: 0,
                width: 1920,
                height: 1080,
            },
        ];

        assert!(find_monitor_for_point(&monitors, 960, 540).is_some());
        assert!(find_monitor_for_point(&monitors, 2880, 540).is_some());
        assert!(find_monitor_for_point(&monitors, -1, -1).is_none());
        assert!(find_monitor_for_point(&[], 0, 0).is_none());
    }
}
