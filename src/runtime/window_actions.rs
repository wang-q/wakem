//! Window action execution — platform-agnostic dispatch of [`WindowAction`] variants.
//!
//! Extracted from [`KeyMapper`](super::KeyMapper) to keep the mapper focused on
//! rule matching and event processing.

use crate::platform::traits::{
    NotificationService, WindowManagerTrait, WindowPresetManager,
};
use crate::platform::types::WindowId;
use crate::types::{MonitorDirection, WindowAction};
use tracing::debug;

/// Execute a window action using the provided window manager.
///
/// All window operations are dispatched through [`WindowManagerTrait`] trait,
/// which provides both basic and advanced window management operations.
/// Optional notification and preset services enable ShowDebugInfo, SavePreset, etc.
pub fn execute_window_action(
    wm: &dyn WindowManagerTrait,
    action: &WindowAction,
    notification_service: Option<&dyn NotificationService>,
    window_preset_manager: Option<&mut (dyn WindowPresetManager + '_)>,
) -> anyhow::Result<()> {
    let window = wm
        .get_foreground_window()
        .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

    match action {
        WindowAction::Center => {
            wm.move_to_center(window)?;
        }
        WindowAction::MoveToEdge(edge) => {
            wm.move_to_edge(window, *edge)?;
        }
        WindowAction::HalfScreen(edge) => {
            wm.set_half_screen(window, *edge)?;
        }
        WindowAction::LoopWidth(align) => {
            wm.loop_width(window, *align)?;
        }
        WindowAction::LoopHeight(align) => {
            wm.loop_height(window, *align)?;
        }
        WindowAction::FixedRatio { ratio, scale_index } => {
            wm.set_fixed_ratio(window, *ratio, Some(*scale_index))?;
        }
        WindowAction::NativeRatio { scale_index } => {
            wm.set_native_ratio(window, Some(*scale_index))?;
        }
        WindowAction::SwitchToNextWindow => {
            wm.switch_to_next_window_of_same_process()?;
        }
        WindowAction::MoveToMonitor(direction) => {
            execute_move_to_monitor(wm, window, direction)?;
        }
        WindowAction::Move { x, y } => {
            let info = wm.get_window_info(window)?;
            wm.set_window_pos(window, *x, *y, info.width, info.height)?;
        }
        WindowAction::Resize { width, height } => {
            let info = wm.get_window_info(window)?;
            wm.set_window_pos(window, info.x, info.y, *width, *height)?;
        }
        WindowAction::Minimize => wm.minimize_window(window)?,
        WindowAction::Maximize => wm.maximize_window(window)?,
        WindowAction::Restore => wm.restore_window(window)?,
        WindowAction::Close => wm.close_window(window)?,
        WindowAction::ToggleTopmost => {
            wm.toggle_topmost(window)?;
        }
        WindowAction::ShowDebugInfo => {
            show_debug_info(wm, window, notification_service);
        }
        WindowAction::ShowNotification { title, message } => {
            show_notification(title, message, notification_service);
        }
        WindowAction::SavePreset { name } => {
            save_preset(name, notification_service, window_preset_manager);
        }
        WindowAction::LoadPreset { name } => {
            load_preset(window, name, window_preset_manager);
        }
        WindowAction::ApplyPreset => {
            apply_preset(window, window_preset_manager);
        }
        WindowAction::None => {}
    }

    Ok(())
}

fn execute_move_to_monitor(
    wm: &dyn WindowManagerTrait,
    window: WindowId,
    direction: &MonitorDirection,
) -> anyhow::Result<()> {
    let monitors = wm.get_monitors();
    if monitors.len() <= 1 {
        debug!("Only one monitor, skipping move");
        return Ok(());
    }

    let info = wm.get_window_info(window)?;
    let current_monitor_idx = monitors
        .iter()
        .position(|m| {
            info.x >= m.x
                && info.x < m.x + m.width
                && info.y >= m.y
                && info.y < m.y + m.height
        })
        .unwrap_or(0);

    let target_index = match direction {
        MonitorDirection::Next => (current_monitor_idx + 1) % monitors.len(),
        MonitorDirection::Prev => {
            if current_monitor_idx == 0 {
                monitors.len() - 1
            } else {
                current_monitor_idx - 1
            }
        }
        MonitorDirection::Index(idx) => {
            if *idx >= 0 && (*idx as usize) < monitors.len() {
                *idx as usize
            } else {
                current_monitor_idx
            }
        }
    };

    if target_index == current_monitor_idx {
        debug!("Already on target monitor, skipping move");
        return Ok(());
    }

    let target = &monitors[target_index];
    let current = &monitors[current_monitor_idx];
    let rel_x = (info.x - current.x) as f32 / current.width as f32;
    let rel_y = (info.y - current.y) as f32 / current.height as f32;
    let new_x = target.x + (rel_x * target.width as f32) as i32;
    let new_y = target.y + (rel_y * target.height as f32) as i32;
    wm.set_window_pos(window, new_x, new_y, info.width, info.height)
}

fn show_debug_info(
    wm: &dyn WindowManagerTrait,
    window: WindowId,
    notification_service: Option<&dyn NotificationService>,
) {
    match wm.get_window_info(window) {
        Ok(info) => {
            let debug_info = format!(
                "Window Debug Info:\n\
                 Position: ({}, {})\n\
                 Size: {}x{}\n\
                 Process: {}",
                info.x, info.y, info.width, info.height, info.process_name
            );
            debug!("{}", debug_info);
            show_notification("wakem - Debug Info", &debug_info, notification_service);
        }
        Err(e) => {
            debug!("Failed to get debug info: {}", e);
        }
    }
}

fn show_notification(
    title: &str,
    message: &str,
    notification_service: Option<&dyn NotificationService>,
) {
    if let Some(ns) = notification_service {
        if let Err(e) = ns.show(title, message) {
            debug!("Failed to show notification: {}", e);
        }
    } else {
        debug!("NotificationService not available, cannot show notification");
    }
}

fn save_preset(
    name: &str,
    notification_service: Option<&dyn NotificationService>,
    window_preset_manager: Option<&mut (dyn WindowPresetManager + '_)>,
) {
    if let Some(pm) = window_preset_manager {
        match pm.get_foreground_window_info() {
            Some(Ok(_info)) => {
                if let Err(e) = pm.save_preset(name.to_string()) {
                    debug!("Failed to save preset '{}': {}", name, e);
                } else {
                    debug!("Saved preset '{}' for current window", name);
                    show_notification(
                        "wakem",
                        &format!("Preset '{}' saved", name),
                        notification_service,
                    );
                }
            }
            Some(Err(e)) => {
                debug!("Failed to get foreground window info: {}", e);
            }
            None => {
                debug!("No foreground window found");
            }
        }
    } else {
        debug!("WindowPresetManager not available, cannot save preset");
    }
}

fn load_preset(
    _window: WindowId,
    name: &str,
    window_preset_manager: Option<&mut (dyn WindowPresetManager + '_)>,
) {
    if let Some(pm) = window_preset_manager {
        if let Err(e) = pm.load_preset(name) {
            debug!("Failed to load preset '{}': {}", name, e);
        } else {
            debug!("Loaded preset '{}' for current window", name);
        }
    } else {
        debug!("WindowPresetManager not available, cannot load preset");
    }
}

fn apply_preset(
    window: WindowId,
    window_preset_manager: Option<&mut (dyn WindowPresetManager + '_)>,
) {
    if let Some(pm) = window_preset_manager {
        match pm.apply_preset_for_window_by_id(window) {
            Ok(true) => {
                debug!("Applied matching preset to current window");
            }
            Ok(false) => {
                debug!("No matching preset found for current window");
            }
            Err(e) => {
                debug!("Failed to apply preset: {}", e);
            }
        }
    } else {
        debug!("WindowPresetManager not available, cannot apply preset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::traits::{
        ForegroundWindowOperations, MonitorOperations, WindowOperations,
        WindowStateQueries, WindowSwitching,
    };
    use crate::platform::types::{MonitorInfo, WindowInfo};
    use std::cell::RefCell;

    #[derive(Clone, Copy)]
    struct TestWindowInfo {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    struct TestWindowManager {
        info: RefCell<TestWindowInfo>,
        monitors: Vec<MonitorInfo>,
        pos_log: RefCell<Vec<(i32, i32, i32, i32)>>,
    }

    unsafe impl Sync for TestWindowManager {}

    impl TestWindowManager {
        fn new(monitor: MonitorInfo, window_width: i32, window_height: i32) -> Self {
            Self {
                info: RefCell::new(TestWindowInfo {
                    x: monitor.x,
                    y: monitor.y,
                    width: window_width,
                    height: window_height,
                }),
                monitors: vec![monitor],
                pos_log: RefCell::new(Vec::new()),
            }
        }

        fn last_pos(&self) -> (i32, i32, i32, i32) {
            self.pos_log.borrow().last().copied().unwrap()
        }
    }

    impl WindowOperations for TestWindowManager {
        fn get_window_info(&self, _window: usize) -> anyhow::Result<WindowInfo> {
            let info = self.info.borrow();
            Ok(WindowInfo {
                id: 0,
                title: "Test".to_string(),
                process_name: "test.exe".to_string(),
                executable_path: None,
                x: info.x,
                y: info.y,
                width: info.width,
                height: info.height,
            })
        }

        fn set_window_pos(
            &self,
            _window: usize,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
        ) -> anyhow::Result<()> {
            self.pos_log.borrow_mut().push((x, y, width, height));
            *self.info.borrow_mut() = TestWindowInfo {
                x,
                y,
                width,
                height,
            };
            Ok(())
        }

        fn minimize_window(&self, _window: usize) -> anyhow::Result<()> {
            Ok(())
        }

        fn maximize_window(&self, _window: usize) -> anyhow::Result<()> {
            Ok(())
        }

        fn restore_window(&self, _window: usize) -> anyhow::Result<()> {
            Ok(())
        }

        fn close_window(&self, _window: usize) -> anyhow::Result<()> {
            Ok(())
        }
    }

    impl WindowStateQueries for TestWindowManager {
        fn is_window_valid(&self, _window: usize) -> bool {
            true
        }

        fn is_minimized(&self, _window: usize) -> bool {
            false
        }

        fn is_maximized(&self, _window: usize) -> bool {
            false
        }

        fn is_topmost(&self, _window: usize) -> bool {
            false
        }
    }

    impl MonitorOperations for TestWindowManager {
        fn get_monitors(&self) -> Vec<MonitorInfo> {
            self.monitors.clone()
        }

        fn move_to_monitor(
            &self,
            _window: usize,
            _monitor_index: usize,
        ) -> anyhow::Result<()> {
            Ok(())
        }
    }

    impl ForegroundWindowOperations for TestWindowManager {
        fn get_foreground_window(&self) -> Option<usize> {
            Some(0)
        }

        fn set_topmost(&self, _window: usize, _topmost: bool) -> anyhow::Result<()> {
            Ok(())
        }
    }

    impl WindowSwitching for TestWindowManager {}

    #[test]
    fn test_execute_move_action() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let wm = TestWindowManager::new(monitor, 800, 600);
        execute_window_action(&wm, &WindowAction::Move { x: 100, y: 200 }, None, None)
            .unwrap();
        let (x, y, w, h) = wm.last_pos();
        assert_eq!(x, 100);
        assert_eq!(y, 200);
        assert_eq!(w, 800);
        assert_eq!(h, 600);
    }

    #[test]
    fn test_execute_resize_action() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let wm = TestWindowManager::new(monitor, 800, 600);
        execute_window_action(
            &wm,
            &WindowAction::Resize {
                width: 1024,
                height: 768,
            },
            None,
            None,
        )
        .unwrap();
        let (x, y, w, h) = wm.last_pos();
        assert_eq!(x, 0);
        assert_eq!(y, 0);
        assert_eq!(w, 1024);
        assert_eq!(h, 768);
    }

    #[test]
    fn test_execute_center_action() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let wm = TestWindowManager::new(monitor, 800, 600);
        execute_window_action(&wm, &WindowAction::Center, None, None).unwrap();
        let (x, y, w, h) = wm.last_pos();
        assert_eq!(w, 800);
        assert_eq!(h, 600);
        assert_eq!(x, (1920 - 800) / 2);
        assert_eq!(y, (1080 - 600) / 2);
    }

    #[test]
    fn test_execute_half_screen_left() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let wm = TestWindowManager::new(monitor, 800, 600);
        execute_window_action(
            &wm,
            &WindowAction::HalfScreen(crate::types::Edge::Left),
            None,
            None,
        )
        .unwrap();
        let (x, _y, w, h) = wm.last_pos();
        assert_eq!(x, 0);
        assert_eq!(w, 960);
        assert_eq!(h, 1080);
    }

    #[test]
    fn test_execute_loop_width() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let wm = TestWindowManager::new(monitor, 1440, 1080);
        execute_window_action(
            &wm,
            &WindowAction::LoopWidth(crate::types::Alignment::Left),
            None,
            None,
        )
        .unwrap();
        let (_, _, w, _) = wm.last_pos();
        assert_eq!(w, 1152);
    }

    #[test]
    fn test_execute_none_action() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let wm = TestWindowManager::new(monitor, 800, 600);
        execute_window_action(&wm, &WindowAction::None, None, None).unwrap();
        assert_eq!(wm.pos_log.borrow().len(), 0);
    }
}
