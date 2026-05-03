//! Platform abstraction traits
//!
//! This module defines the cross-platform interfaces implemented
//! by each platform-specific module (Windows, macOS).

use crate::platform::types::*;
use crate::types::{InputEvent, KeyAction, ModifierState, MouseAction};
use anyhow::Result;
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

/// Window manager trait - unified interface for window operations
pub trait WindowManager: Send + Sync {
    // === Required: platform-specific implementations ===
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

    // === Platform-specific extensions (with default bail) ===
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()> {
        let _ = (window, monitor_index);
        anyhow::bail!("move_to_monitor not implemented on this platform")
    }

    fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        anyhow::bail!(
            "switch_to_next_window_of_same_process not implemented on this platform"
        )
    }

    // === Extension methods with default implementations ===
    fn move_to_center(&self, window: WindowId) -> Result<()> {
        use crate::platform::common::window_ops::calc_centered_pos;
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if let Some((x, y)) = calc_centered_pos(&info, &monitors) {
            self.set_window_pos(window, x, y, info.width, info.height)?;
        }
        Ok(())
    }

    fn move_to_edge(&self, window: WindowId, edge: crate::types::Edge) -> Result<()> {
        use crate::platform::common::window_ops::calc_edge_pos;
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if let Some((x, y)) = calc_edge_pos(&info, &monitors, edge) {
            self.set_window_pos(window, x, y, info.width, info.height)?;
        }
        Ok(())
    }

    fn set_half_screen(&self, window: WindowId, edge: crate::types::Edge) -> Result<()> {
        use crate::platform::common::window_ops::calc_half_screen;
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if let Some((x, y, w, h)) = calc_half_screen(&info, &monitors, edge) {
            self.set_window_pos(window, x, y, w, h)?;
        }
        Ok(())
    }

    fn loop_width(
        &self,
        window: WindowId,
        align: crate::types::Alignment,
    ) -> Result<()> {
        use crate::platform::common::window_ops::calc_looped_width;
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if let Some((x, y, w, h)) = calc_looped_width(&info, &monitors, align) {
            self.set_window_pos(window, x, y, w, h)?;
        }
        Ok(())
    }

    fn loop_height(
        &self,
        window: WindowId,
        align: crate::types::Alignment,
    ) -> Result<()> {
        use crate::platform::common::window_ops::calc_looped_height;
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if let Some((x, y, w, h)) = calc_looped_height(&info, &monitors, align) {
            self.set_window_pos(window, x, y, w, h)?;
        }
        Ok(())
    }

    fn set_fixed_ratio(
        &self,
        window: WindowId,
        ratio: f32,
        scale_index: Option<usize>,
    ) -> Result<()> {
        use crate::platform::common::window_ops::calc_fixed_ratio;
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if let Some((x, y, w, h)) =
            calc_fixed_ratio(&info, &monitors, ratio, scale_index)
        {
            self.set_window_pos(window, x, y, w, h)?;
        }
        Ok(())
    }

    fn set_native_ratio(
        &self,
        window: WindowId,
        scale_index: Option<usize>,
    ) -> Result<()> {
        use crate::platform::common::window_ops::calc_native_ratio;
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if let Some((x, y, w, h)) = calc_native_ratio(&info, &monitors, scale_index) {
            self.set_window_pos(window, x, y, w, h)?;
        }
        Ok(())
    }

    fn toggle_topmost(&self, window: WindowId) -> Result<bool> {
        let current = self.is_topmost(window);
        let new_state = !current;
        self.set_topmost(window, new_state)?;
        Ok(new_state)
    }
}

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
    fn open_folder(path: &std::path::Path) -> Result<()>;
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
