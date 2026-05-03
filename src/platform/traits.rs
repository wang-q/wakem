//! Platform abstraction traits
//!
//! This module defines the cross-platform interfaces implemented
//! by each platform-specific module (Windows, macOS).

use crate::platform::types::*;
use crate::types::{Alignment, Edge, InputEvent, KeyAction, ModifierState, MouseAction};
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Input device trait - for capturing keyboard and mouse events
pub trait InputDevice: Send {
    fn register(&mut self) -> Result<()>;
    fn unregister(&mut self);
    fn poll_event(&mut self) -> Option<InputEvent>;
    fn is_running(&self) -> bool;
    fn stop(&mut self);
}

/// Output device trait - for sending simulated input events
pub trait OutputDevice: Send + Sync {
    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()>;
    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()>;
    fn send_mouse_button(
        &self,
        button: crate::types::MouseButton,
        release: bool,
    ) -> Result<()>;
    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()>;

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
        use crate::platform::common::output_helpers::char_to_vk;
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
        modifiers: &crate::types::ModifierState,
        scan_code: u16,
        virtual_key: u16,
    ) -> Result<()> {
        use crate::types::key_codes::*;

        if modifiers.shift {
            self.send_key(0, VK_SHIFT, false)?;
        }
        if modifiers.ctrl {
            self.send_key(0, VK_CONTROL, false)?;
        }
        if modifiers.alt {
            self.send_key(0, VK_ALT, false)?;
        }
        if modifiers.meta {
            self.send_key(0, VK_LMETA, false)?;
        }

        self.send_key(scan_code, virtual_key, false)?;
        self.send_key(scan_code, virtual_key, true)?;

        if modifiers.meta {
            self.send_key(0, VK_LMETA, true)?;
        }
        if modifiers.alt {
            self.send_key(0, VK_ALT, true)?;
        }
        if modifiers.ctrl {
            self.send_key(0, VK_CONTROL, true)?;
        }
        if modifiers.shift {
            self.send_key(0, VK_SHIFT, true)?;
        }

        Ok(())
    }

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
}

// ============================================================================
// Window Management Traits - Refactored Design
// ============================================================================

/// Window API base trait - platform-specific low-level window operations
///
/// This trait defines the minimal set of operations that each platform must implement.
/// It uses associated types to allow platform-specific window identifiers.
pub trait WindowApiBase: Send + Sync {
    type WindowId: Copy + Send + 'static;

    // === Required: platform-specific implementations (inner methods) ===
    fn get_foreground_window_inner(&self) -> Option<Self::WindowId>;
    fn set_window_pos_inner(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()>;
    fn minimize_window_inner(&self, window: Self::WindowId) -> Result<()>;
    fn maximize_window_inner(&self, window: Self::WindowId) -> Result<()>;
    fn restore_window_inner(&self, window: Self::WindowId) -> Result<()>;
    fn close_window_inner(&self, window: Self::WindowId) -> Result<()>;
    fn set_topmost_inner(&self, window: Self::WindowId, topmost: bool) -> Result<()>;
    fn is_topmost_inner(&self, window: Self::WindowId) -> bool;
    fn is_window_valid_inner(&self, window: Self::WindowId) -> bool;
    fn is_minimized_inner(&self, window: Self::WindowId) -> bool;
    fn is_maximized_inner(&self, window: Self::WindowId) -> bool;
    fn get_window_title_inner(&self, window: Self::WindowId) -> Option<String>;
    fn get_window_rect_inner(&self, window: Self::WindowId) -> Result<WindowFrame>;
    fn get_monitors_inner(&self) -> Vec<MonitorInfo>;

    // === Default delegations to inner methods ===
    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.get_foreground_window_inner()
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.set_window_pos_inner(window, x, y, width, height)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.minimize_window_inner(window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.maximize_window_inner(window)
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        self.restore_window_inner(window)
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        self.close_window_inner(window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.set_topmost_inner(window, topmost)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.is_topmost_inner(window)
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.is_window_valid_inner(window)
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        self.is_minimized_inner(window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.is_maximized_inner(window)
    }

    fn get_window_title(&self, window: Self::WindowId) -> Option<String> {
        self.get_window_title_inner(window)
    }

    fn get_window_rect(&self, window: Self::WindowId) -> Result<WindowFrame> {
        self.get_window_rect_inner(window)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        self.get_monitors_inner()
    }

    /// Ensure window is restored (not minimized or maximized)
    fn ensure_window_restored(&self, window: Self::WindowId) -> Result<()> {
        if self.is_minimized(window) || self.is_maximized(window) {
            self.restore_window(window)?;
        }
        Ok(())
    }
}

/// Basic window operations trait
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
pub trait WindowStateQueries: Send + Sync {
    fn is_window_valid(&self, window: WindowId) -> bool;
    fn is_minimized(&self, window: WindowId) -> bool;
    fn is_maximized(&self, window: WindowId) -> bool;
    fn is_topmost(&self, window: WindowId) -> bool;
}

/// Monitor operations trait
pub trait MonitorOperations: Send + Sync {
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()>;
}

/// Foreground window operations
pub trait ForegroundWindowOperations: Send + Sync {
    fn get_foreground_window(&self) -> Option<WindowId>;
    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()>;
}

/// Window manager trait - combines all window-related operations
pub trait WindowManagerTrait:
    WindowOperations
    + WindowStateQueries
    + MonitorOperations
    + ForegroundWindowOperations
    + Send
    + Sync
{
}

/// Auto-implement WindowManagerTrait for any type that implements all component traits
impl<T> WindowManagerTrait for T where
    T: WindowOperations
        + WindowStateQueries
        + MonitorOperations
        + ForegroundWindowOperations
{
}

/// Legacy WindowManager trait - kept for backward compatibility
///
/// This trait is deprecated in favor of the more granular traits above.
/// New code should use `WindowManagerTrait` or the specific component traits.
pub trait WindowManager: Send + Sync {
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
    fn is_window_valid(&self, window: WindowId) -> bool;
    fn is_minimized(&self, window: WindowId) -> bool;
    fn is_maximized(&self, window: WindowId) -> bool;
    fn get_monitors(&self) -> Vec<MonitorInfo>;

    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()> {
        let _ = (window, monitor_index);
        anyhow::bail!("move_to_monitor not implemented on this platform")
    }

    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        anyhow::bail!(
            "switch_to_next_window_of_same_process not implemented on this platform"
        )
    }

    fn move_to_center(&self, window: WindowId) -> Result<()>;
    fn move_to_edge(&self, window: WindowId, edge: Edge) -> Result<()>;
    fn set_half_screen(&self, window: WindowId, edge: Edge) -> Result<()>;
    fn loop_width(&self, window: WindowId, align: Alignment) -> Result<()>;
    fn loop_height(&self, window: WindowId, align: Alignment) -> Result<()>;
    fn set_fixed_ratio(
        &self,
        window: WindowId,
        ratio: f32,
        scale_index: Option<usize>,
    ) -> Result<()>;
    fn set_native_ratio(
        &self,
        window: WindowId,
        scale_index: Option<usize>,
    ) -> Result<()>;
    fn toggle_topmost(&self, window: WindowId) -> Result<bool>;
}

/// Window manager extension trait - high-level window operations
///
/// These methods combine basic operations to provide convenient
/// high-level functionality like centering windows, moving to edges,
/// and resizing with alignment.
pub trait WindowManagerExt:
    WindowOperations + WindowStateQueries + MonitorOperations + ForegroundWindowOperations
{
    fn move_to_center(&self, window: WindowId) -> Result<()>;
    fn move_to_edge(&self, window: WindowId, edge: Edge) -> Result<()>;
    fn set_half_screen(&self, window: WindowId, edge: Edge) -> Result<()>;
    fn loop_width(&self, window: WindowId, align: Alignment) -> Result<()>;
    fn loop_height(&self, window: WindowId, align: Alignment) -> Result<()>;
    fn set_fixed_ratio(
        &self,
        window: WindowId,
        ratio: f32,
        scale_index: Option<usize>,
    ) -> Result<()>;
    fn set_native_ratio(
        &self,
        window: WindowId,
        scale_index: Option<usize>,
    ) -> Result<()>;
    fn toggle_topmost(&self, window: WindowId) -> Result<bool>;
}

/// Find the monitor that contains the given point, falling back to the first monitor
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

/// Find the next ratio in the cycle after the current one
pub fn find_next_ratio(ratios: &[f32], current: f32) -> f32 {
    let closest_idx = ratios
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (current - **a)
                .abs()
                .partial_cmp(&(current - **b).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    ratios[(closest_idx + 1) % ratios.len()]
}

// ============================================================================
// Other Platform Traits
// ============================================================================

/// Window preset manager trait
pub trait WindowPresetManager: Send + Sync {
    fn load_presets(&mut self, presets: Vec<crate::config::WindowPreset>);
    fn save_preset(&mut self, name: String) -> Result<()>;
    fn load_preset(&self, name: &str) -> Result<()>;
    fn get_foreground_window_info(&self) -> Option<Result<WindowInfo>>;
    fn apply_preset_for_window_by_id(&self, window_id: WindowId) -> Result<bool>;
}

/// Notification service trait
pub trait NotificationService: Send + Sync {
    fn show(&self, title: &str, message: &str) -> Result<()>;
    fn initialize(&self, _ctx: &NotificationInitContext) {}
}

/// Launcher trait
pub trait Launcher: Send + Sync {
    fn launch(&self, action: &crate::types::LaunchAction) -> Result<()>;
}

/// Window event hook trait
pub trait WindowEventHook: Send {
    fn start_with_shutdown(&mut self, shutdown_flag: Arc<AtomicBool>) -> Result<()>;
    fn stop(&mut self);
    fn shutdown_flag(&self) -> Arc<AtomicBool>;
}

/// Platform utilities trait
pub trait PlatformUtilities {
    fn get_modifier_state() -> ModifierState;
    fn get_process_name_by_pid(pid: u32) -> Result<String>;
    fn get_executable_path_by_pid(pid: u32) -> Result<String>;
}

/// Context provider trait
pub trait ContextProvider {
    fn get_current_context() -> Option<WindowContext>;
}

/// Tray lifecycle trait
pub trait TrayLifecycle {
    fn run_tray_message_loop(callback: Box<dyn Fn(AppCommand) + Send>) -> Result<()>;
    fn stop_tray();
}

/// Application control trait
pub trait ApplicationControl {
    fn detach_console();
    fn terminate_application();
    fn open_folder(path: &Path) -> Result<()>;
    fn force_kill_instance(instance_id: u32) -> Result<()>;
}

/// Platform factory trait - for creating platform-specific objects
pub trait PlatformFactory {
    type InputDevice: InputDevice;
    type OutputDevice: OutputDevice;
    type WindowManager: WindowManager;
    type WindowPresetManager: WindowPresetManager;
    type NotificationService: NotificationService;
    type Launcher: Launcher;
    type WindowEventHook: WindowEventHook;

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
