//! Window action execution — platform-agnostic dispatch of [`WindowAction`] variants.
//!
//! Extracted from [`KeyMapper`](super::KeyMapper) to keep the mapper focused on
//! rule matching and event processing.
//!
//! Uses [`CommonWindowApi`] which provides both basic operations and advanced
//! window management operations (center, half-screen, loop, etc.).

use crate::platform::common::window_manager::CommonWindowApi;
use crate::platform::traits::WindowInfoProvider;
use crate::types::{MonitorDirection, WindowAction};
use tracing::debug;

/// Execute a window action using the provided window API.
///
/// All window operations are dispatched through [`CommonWindowApi`], which
/// provides both basic and advanced window management operations.
#[allow(dead_code)]
pub fn execute_window_action<A: CommonWindowApi>(
    api: &A,
    action: &WindowAction,
) -> anyhow::Result<()> {
    let window_id = api
        .get_foreground_window()
        .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

    match action {
        WindowAction::Move { x, y } => {
            let info = api.get_window_info(window_id)?;
            api.set_window_pos(window_id, *x, *y, info.width(), info.height())?;
        }
        WindowAction::Resize { width, height } => {
            let info = api.get_window_info(window_id)?;
            api.set_window_pos(window_id, info.x(), info.y(), *width, *height)?;
        }

        WindowAction::Minimize => {
            api.minimize_window(window_id)?;
        }
        WindowAction::Maximize => {
            api.maximize_window(window_id)?;
        }
        WindowAction::Restore => {
            api.restore_window(window_id)?;
        }
        WindowAction::Close => {
            api.close_window(window_id)?;
        }

        WindowAction::ToggleTopmost => {
            let is_top = api.is_topmost(window_id);
            api.set_topmost(window_id, !is_top)?;
        }

        WindowAction::MoveToMonitor(direction) => {
            execute_move_to_monitor(api, window_id, direction)?;
        }

        WindowAction::Center => {
            api.move_to_center(window_id)?;
        }
        WindowAction::MoveToEdge(edge) => {
            api.move_to_edge(window_id, *edge)?;
        }
        WindowAction::HalfScreen(edge) => {
            api.set_half_screen(window_id, *edge)?;
        }
        WindowAction::LoopWidth(align) => {
            api.loop_width(window_id, *align)?;
        }
        WindowAction::LoopHeight(align) => {
            api.loop_height(window_id, *align)?;
        }
        WindowAction::FixedRatio { ratio, .. } => {
            api.set_fixed_ratio(window_id, *ratio)?;
        }
        WindowAction::NativeRatio { .. } => {
            api.set_native_ratio(window_id)?;
        }

        WindowAction::SwitchToNextWindow => {
            debug!("SwitchToNextWindow: requires platform-specific implementation");
        }

        WindowAction::ShowDebugInfo => match api.get_window_info(window_id) {
            Ok(info) => {
                debug!(
                    x = info.x(),
                    y = info.y(),
                    width = info.width(),
                    height = info.height(),
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

#[allow(dead_code)]
fn execute_move_to_monitor<A: CommonWindowApi>(
    api: &A,
    window_id: A::WindowId,
    direction: &MonitorDirection,
) -> anyhow::Result<()> {
    let monitors = api.get_monitors();
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
            let info = api.get_window_info(window_id)?;
            let cx = info.x() + info.width() / 2;
            let cy = info.y() + info.height() / 2;

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

    let info = api.get_window_info(window_id)?;
    let target = &monitors[monitor_index];
    let new_x = target.x + (target.width - info.width()) / 2;
    let new_y = target.y + (target.height - info.height()) / 2;
    api.set_window_pos(window_id, new_x, new_y, info.width(), info.height())?;

    Ok(())
}

#[allow(dead_code)]
fn find_monitor_index_for_point(
    monitors: &[crate::platform::traits::MonitorInfo],
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
    use crate::platform::traits::{MonitorInfo, WindowInfoProvider};
    use std::cell::RefCell;

    #[derive(Clone, Copy)]
    struct TestWindowInfo {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    }

    impl WindowInfoProvider for TestWindowInfo {
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

    struct TestApi {
        info: RefCell<TestWindowInfo>,
        monitors: Vec<MonitorInfo>,
        pos_log: RefCell<Vec<(i32, i32, i32, i32)>>,
    }

    impl TestApi {
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

    impl CommonWindowApi for TestApi {
        type WindowId = ();
        type WindowInfo = TestWindowInfo;

        fn get_foreground_window(&self) -> Option<Self::WindowId> {
            Some(())
        }
        fn get_window_info(
            &self,
            _window: Self::WindowId,
        ) -> anyhow::Result<Self::WindowInfo> {
            Ok(*self.info.borrow())
        }
        fn set_window_pos(
            &self,
            _window: Self::WindowId,
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
        fn minimize_window(&self, _window: Self::WindowId) -> anyhow::Result<()> {
            Ok(())
        }
        fn maximize_window(&self, _window: Self::WindowId) -> anyhow::Result<()> {
            Ok(())
        }
        fn restore_window(&self, _window: Self::WindowId) -> anyhow::Result<()> {
            Ok(())
        }
        fn close_window(&self, _window: Self::WindowId) -> anyhow::Result<()> {
            Ok(())
        }
        fn get_monitors(&self) -> Vec<MonitorInfo> {
            self.monitors.clone()
        }
        fn is_window_valid(&self, _window: Self::WindowId) -> bool {
            true
        }
        fn is_maximized(&self, _window: Self::WindowId) -> bool {
            false
        }
        fn is_topmost(&self, _window: Self::WindowId) -> bool {
            false
        }
        fn set_topmost(
            &self,
            _window: Self::WindowId,
            _topmost: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn api(&self) -> &dyn std::any::Any {
            self
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
        let api = TestApi::new(monitor, 800, 600);
        execute_window_action(&api, &WindowAction::Move { x: 100, y: 200 }).unwrap();
        let (x, y, w, h) = api.last_pos();
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
        let api = TestApi::new(monitor, 800, 600);
        execute_window_action(
            &api,
            &WindowAction::Resize {
                width: 1024,
                height: 768,
            },
        )
        .unwrap();
        let (x, y, w, h) = api.last_pos();
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
        let api = TestApi::new(monitor, 800, 600);
        execute_window_action(&api, &WindowAction::Center).unwrap();
        let (x, y, w, h) = api.last_pos();
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
        let api = TestApi::new(monitor, 800, 600);
        execute_window_action(&api, &WindowAction::HalfScreen(crate::types::Edge::Left))
            .unwrap();
        let (x, _y, w, h) = api.last_pos();
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
        let api = TestApi::new(monitor, 1440, 1080);
        execute_window_action(
            &api,
            &WindowAction::LoopWidth(crate::types::Alignment::Left),
        )
        .unwrap();
        let (_, _, w, _) = api.last_pos();
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
        let api = TestApi::new(monitor, 800, 600);
        execute_window_action(&api, &WindowAction::None).unwrap();
        assert_eq!(api.pos_log.borrow().len(), 0);
    }
}
