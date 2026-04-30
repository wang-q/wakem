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

pub use super::types::*;

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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub trait WindowStateQueries: Send + Sync {
    fn is_window_valid(&self, window: WindowId) -> bool;
    fn is_minimized(&self, window: WindowId) -> bool;
    fn is_maximized(&self, window: WindowId) -> bool;
    fn is_topmost(&self, window: WindowId) -> bool;
}

/// Monitor-related operations
#[allow(dead_code)]
pub trait MonitorOperations: Send + Sync {
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    fn move_to_monitor(&self, window: WindowId, monitor_index: usize) -> Result<()>;
}

/// Foreground window operations
#[allow(dead_code)]
pub trait ForegroundWindowOperations: Send + Sync {
    fn get_foreground_window(&self) -> Option<WindowId>;
}

/// Window manager trait - composed of fine-grained operation traits
///
/// This is a marker trait that combines all window operation traits.
/// Implementors automatically satisfy this by implementing the
/// constituent traits.
#[allow(dead_code)]
pub trait WindowManagerTrait:
    WindowOperations
    + WindowStateQueries
    + MonitorOperations
    + ForegroundWindowOperations
    + Send
    + Sync
{
}

/// Extended window manager operations with default implementations
///
/// Provides high-level window management operations (center, half-screen,
/// loop, fixed ratio, etc.) built on top of the basic
/// [`WindowManagerTrait`] operations.
#[allow(dead_code)]
pub trait WindowManagerExt:
    WindowOperations + WindowStateQueries + MonitorOperations + ForegroundWindowOperations
{
    fn move_to_center(&self, window: WindowId) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if monitors.is_empty() {
            anyhow::bail!("No monitors found");
        }
        let monitor = find_monitor_for_point(
            &monitors,
            info.x + info.width / 2,
            info.y + info.height / 2,
        );
        let frame = WindowFrame::new(info.x, info.y, info.width, info.height);
        let (new_x, new_y) = frame.center_in(&monitor);
        self.set_window_pos(window, new_x, new_y, info.width, info.height)
    }

    fn move_to_edge(&self, window: WindowId, edge: crate::types::Edge) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if monitors.is_empty() {
            anyhow::bail!("No monitors found");
        }
        let monitor = find_monitor_for_point(
            &monitors,
            info.x + info.width / 2,
            info.y + info.height / 2,
        );
        let (x, y) = match edge {
            crate::types::Edge::Left => (monitor.x, monitor.y),
            crate::types::Edge::Right => {
                (monitor.x + monitor.width - info.width, monitor.y)
            }
            crate::types::Edge::Top => (monitor.x, monitor.y),
            crate::types::Edge::Bottom => {
                (monitor.x, monitor.y + monitor.height - info.height)
            }
        };
        self.set_window_pos(window, x, y, info.width, info.height)
    }

    fn set_half_screen(&self, window: WindowId, edge: crate::types::Edge) -> Result<()> {
        let info = self.get_window_info(window)?;
        let monitors = self.get_monitors();
        if monitors.is_empty() {
            anyhow::bail!("No monitors found");
        }
        let monitor = find_monitor_for_point(
            &monitors,
            info.x + info.width / 2,
            info.y + info.height / 2,
        );
        let (x, y, w, h) = match edge {
            crate::types::Edge::Left => {
                (monitor.x, monitor.y, monitor.width / 2, monitor.height)
            }
            crate::types::Edge::Right => (
                monitor.x + monitor.width / 2,
                monitor.y,
                monitor.width / 2,
                monitor.height,
            ),
            crate::types::Edge::Top => {
                (monitor.x, monitor.y, monitor.width, monitor.height / 2)
            }
            crate::types::Edge::Bottom => (
                monitor.x,
                monitor.y + monitor.height / 2,
                monitor.width,
                monitor.height / 2,
            ),
        };
        self.set_window_pos(window, x, y, w, h)
    }

    fn loop_width(
        &self,
        _window: WindowId,
        _align: crate::types::Alignment,
    ) -> Result<()> {
        debug_assert!(
            false,
            "loop_width should be overridden by platform implementation"
        );
        Ok(())
    }

    fn loop_height(
        &self,
        _window: WindowId,
        _align: crate::types::Alignment,
    ) -> Result<()> {
        debug_assert!(
            false,
            "loop_height should be overridden by platform implementation"
        );
        Ok(())
    }

    fn set_fixed_ratio(
        &self,
        _window: WindowId,
        _ratio: f32,
        _scale_index: Option<u32>,
    ) -> Result<()> {
        debug_assert!(
            false,
            "set_fixed_ratio should be overridden by platform implementation"
        );
        Ok(())
    }

    fn set_native_ratio(
        &self,
        _window: WindowId,
        _scale_index: Option<u32>,
    ) -> Result<()> {
        debug_assert!(
            false,
            "set_native_ratio should be overridden by platform implementation"
        );
        Ok(())
    }
}

/// Find the monitor that contains the given point, falling back to the first monitor
pub fn find_monitor_for_point(monitors: &[MonitorInfo], x: i32, y: i32) -> MonitorInfo {
    for monitor in monitors {
        if x >= monitor.x
            && x < monitor.x + monitor.width
            && y >= monitor.y
            && y < monitor.y + monitor.height
        {
            return *monitor;
        }
    }
    monitors.first().copied().unwrap_or(MonitorInfo {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    })
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
