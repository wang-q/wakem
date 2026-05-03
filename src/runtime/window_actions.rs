//! Window action execution — platform-agnostic dispatch of [`WindowAction`] variants.
//!
//! Extracted from [`KeyMapper`](super::KeyMapper) to keep the mapper focused on
//! rule matching and event processing.

use crate::platform::traits::WindowManagerTrait;
use crate::platform::types::{MonitorInfo, WindowInfo};
use crate::types::{MonitorDirection, WindowAction};
use tracing::debug;

/// Execute a window action using the provided window manager.
///
/// All window operations are dispatched through [`WindowManagerTrait`] trait,
/// which provides both basic and advanced window management operations.
pub fn execute_window_action<W: WindowManagerTrait>(
    wm: &W,
    action: &WindowAction,
) -> anyhow::Result<()> {
    let window_id = wm
        .get_foreground_window()
        .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

    match action {
        WindowAction::Move { x, y } => {
            let info = wm.get_window_info(window_id)?;
            wm.set_window_pos(window_id, *x, *y, info.width, info.height)?;
        }
        WindowAction::Resize { width, height } => {
            let info = wm.get_window_info(window_id)?;
            wm.set_window_pos(window_id, info.x, info.y, *width, *height)?;
        }

        WindowAction::Minimize => {
            wm.minimize_window(window_id)?;
        }
        WindowAction::Maximize => {
            wm.maximize_window(window_id)?;
        }
        WindowAction::Restore => {
            wm.restore_window(window_id)?;
        }
        WindowAction::Close => {
            wm.close_window(window_id)?;
        }

        WindowAction::ToggleTopmost => {
            wm.toggle_topmost(window_id)?;
        }

        WindowAction::MoveToMonitor(direction) => {
            execute_move_to_monitor(wm, window_id, direction)?;
        }

        WindowAction::Center => {
            wm.move_to_center(window_id)?;
        }
        WindowAction::MoveToEdge(edge) => {
            wm.move_to_edge(window_id, *edge)?;
        }
        WindowAction::HalfScreen(edge) => {
            wm.set_half_screen(window_id, *edge)?;
        }
        WindowAction::LoopWidth(align) => {
            wm.loop_width(window_id, *align)?;
        }
        WindowAction::LoopHeight(align) => {
            wm.loop_height(window_id, *align)?;
        }
        WindowAction::FixedRatio { ratio, .. } => {
            wm.set_fixed_ratio(window_id, *ratio, None)?;
        }
        WindowAction::NativeRatio { .. } => {
            wm.set_native_ratio(window_id, None)?;
        }

        WindowAction::SwitchToNextWindow => {
            debug!("SwitchToNextWindow: requires platform-specific implementation");
        }

        WindowAction::ShowDebugInfo => match wm.get_window_info(window_id) {
            Ok(info) => {
                debug!(
                    x = info.x,
                    y = info.y,
                    width = info.width,
                    height = info.height,
                    "Window debug info"
                );
            }
            Err(e) => {
                debug!("Failed to get debug info: {}", e);
            }
        },
        WindowAction::ShowNotification { title, message } => {
            debug!(
                title,
                message, "ShowNotification: notification service not available"
            );
        }

        WindowAction::SavePreset { name } => {
            debug!(name, "SavePreset: preset manager not available");
        }
        WindowAction::LoadPreset { name } => {
            debug!(name, "LoadPreset: preset manager not available");
        }
        WindowAction::ApplyPreset => {
            debug!("ApplyPreset: preset manager not available");
        }

        WindowAction::None => {}
    }

    Ok(())
}

fn execute_move_to_monitor<W: WindowManagerTrait>(
    wm: &W,
    window_id: usize,
    direction: &MonitorDirection,
) -> anyhow::Result<()> {
    let monitors = wm.get_monitors();
    if monitors.is_empty() {
        anyhow::bail!("No monitors found");
    }

    let monitor_index: usize = match direction {
        MonitorDirection::Index(idx) => {
            let idx = *idx as usize;
            if idx >= monitors.len() {
                anyhow::bail!(
                    "Monitor index {} out of range (0-{})",
                    idx,
                    monitors.len() - 1
                );
            }
            idx
        }
        MonitorDirection::Next | MonitorDirection::Prev => {
            let info = wm.get_window_info(window_id)?;
            let cx = info.x + info.width / 2;
            let cy = info.y + info.height / 2;

            let current_monitor_idx = find_monitor_index_for_point(&monitors, cx, cy);

            match direction {
                MonitorDirection::Next => (current_monitor_idx + 1) % monitors.len(),
                MonitorDirection::Prev => {
                    if current_monitor_idx == 0 {
                        monitors.len() - 1
                    } else {
                        current_monitor_idx - 1
                    }
                }
                _ => unreachable!(),
            }
        }
    };

    let info = wm.get_window_info(window_id)?;
    let target = &monitors[monitor_index];
    let new_x = target.x + (target.width - info.width) / 2;
    let new_y = target.y + (target.height - info.height) / 2;
    wm.set_window_pos(window_id, new_x, new_y, info.width, info.height)?;

    Ok(())
}

fn find_monitor_index_for_point(
    monitors: &[MonitorInfo],
    x: i32,
    y: i32,
) -> usize {
    for (i, monitor) in monitors.iter().enumerate() {
        if x >= monitor.x
            && x < monitor.x + monitor.width
            && y >= monitor.y
            && y < monitor.y + monitor.height
        {
            return i;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
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

        fn set_topmost(&self, _window: usize, _topmost: bool) -> anyhow::Result<()> {
            Ok(())
        }
    }

    impl MonitorOperations for TestWindowManager {
        fn get_monitors(&self) -> Vec<MonitorInfo> {
            self.monitors.clone()
        }
    }

    impl ForegroundWindowOperations for TestWindowManager {
        fn get_foreground_window(&self) -> Option<usize> {
            Some(0)
        }
    }

    #[test]
    fn test_execute_move_action() {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let wm = TestWindowManager::new(monitor, 800, 600);
        execute_window_action(&wm, &WindowAction::Move { x: 100, y: 200 }).unwrap();
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
        execute_window_action(&wm, &WindowAction::Center).unwrap();
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
        execute_window_action(&wm, &WindowAction::HalfScreen(crate::types::Edge::Left))
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
        execute_window_action(&wm, &WindowAction::None).unwrap();
        assert_eq!(wm.pos_log.borrow().len(), 0);
    }
}
