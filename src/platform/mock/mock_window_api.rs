//! Mock window API for testing
//!
//! Provides a mock implementation of WindowApiBase that can be used
//! in unit tests without platform dependencies.

use crate::platform::common::window_manager::CommonWindowApi;
#[allow(unused_imports)]
use crate::platform::mock::MockWindowId;
#[allow(unused_imports)]
use crate::platform::traits::WindowInfoProvider;
#[allow(unused_imports)]
use crate::platform::traits::{
    ForegroundWindowOperations, MonitorInfo, MonitorOperations, WindowApiBase,
    WindowFrame, WindowId, WindowManagerTrait, WindowOperations, WindowStateQueries,
};
#[allow(unused_imports)]
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum WindowApiCall {
    GetForegroundWindow,
    GetWindowRect {
        window: usize,
    },
    SetWindowPos {
        window: usize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    GetMonitorInfo {
        window: usize,
    },
    IsWindow {
        window: usize,
    },
    GetWindowTitle {
        window: usize,
    },
    MinimizeWindow {
        window: usize,
    },
    MaximizeWindow {
        window: usize,
    },
    RestoreWindow {
        window: usize,
    },
    CloseWindow {
        window: usize,
    },
    SetTopmost {
        window: usize,
        topmost: bool,
    },
}

#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct MockWindowState {
    minimized: bool,
    maximized: bool,
    topmost: bool,
}

#[allow(dead_code)]
pub struct MockWindowApi<Id: MockWindowId> {
    pub foreground_window: Mutex<Option<Id>>,
    pub window_rects: Mutex<HashMap<usize, WindowFrame>>,
    pub monitor_info: Mutex<HashMap<usize, MonitorInfo>>,
    pub window_states: Mutex<HashMap<usize, MockWindowState>>,
    pub operations_log: Mutex<Vec<WindowApiCall>>,
}

// SAFETY: All fields are behind Mutex, providing thread-safe access.
// The Mutex wraps all inner state so MockWindowApi is both Send and Sync.
unsafe impl<Id: MockWindowId> Send for MockWindowApi<Id> {}
unsafe impl<Id: MockWindowId> Sync for MockWindowApi<Id> {}

#[allow(dead_code)]
impl<Id: MockWindowId> MockWindowApi<Id> {
    pub fn new() -> Self {
        Self {
            foreground_window: Mutex::new(None),
            window_rects: Mutex::new(HashMap::new()),
            monitor_info: Mutex::new(HashMap::new()),
            window_states: Mutex::new(HashMap::new()),
            operations_log: Mutex::new(Vec::new()),
        }
    }

    pub fn set_foreground_window(&self, window: Id) {
        *self.foreground_window.lock().unwrap() = Some(window);
    }

    pub fn set_window_rect(&self, window: Id, frame: WindowFrame) {
        self.window_rects
            .lock()
            .unwrap()
            .insert(window.to_usize(), frame);
    }

    pub fn set_monitor_info(&self, window: Id, info: MonitorInfo) {
        self.monitor_info
            .lock()
            .unwrap()
            .insert(window.to_usize(), info);
    }

    pub fn get_operations(&self) -> Vec<WindowApiCall> {
        self.operations_log.lock().unwrap().clone()
    }

    pub fn log_operation(&self, op: WindowApiCall) {
        self.operations_log.lock().unwrap().push(op);
    }

    pub fn get_foreground_window_inner(&self) -> Option<Id> {
        self.log_operation(WindowApiCall::GetForegroundWindow);
        *self.foreground_window.lock().unwrap()
    }

    pub fn get_window_rect(&self, window: Id) -> Option<WindowFrame> {
        self.log_operation(WindowApiCall::GetWindowRect {
            window: window.to_usize(),
        });
        self.window_rects
            .lock()
            .unwrap()
            .get(&window.to_usize())
            .copied()
    }

    fn set_window_pos_inner(
        &self,
        window: Id,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.log_operation(WindowApiCall::SetWindowPos {
            window: window.to_usize(),
            x,
            y,
            width,
            height,
        });

        let mut rects = self.window_rects.lock().unwrap();
        rects.insert(window.to_usize(), WindowFrame::new(x, y, width, height));

        let mut states = self.window_states.lock().unwrap();
        if let Some(state) = states.get_mut(&window.to_usize()) {
            state.minimized = false;
            state.maximized = false;
        }

        Ok(())
    }

    pub fn get_monitor_info(&self, window: Id) -> Option<MonitorInfo> {
        self.log_operation(WindowApiCall::GetMonitorInfo {
            window: window.to_usize(),
        });
        self.monitor_info
            .lock()
            .unwrap()
            .get(&window.to_usize())
            .cloned()
    }

    pub fn is_window_valid_inner(&self, window: Id) -> bool {
        self.log_operation(WindowApiCall::IsWindow {
            window: window.to_usize(),
        });
        self.window_rects
            .lock()
            .unwrap()
            .contains_key(&window.to_usize())
    }

    pub fn get_window_title(&self, window: Id) -> Option<String> {
        self.log_operation(WindowApiCall::GetWindowTitle {
            window: window.to_usize(),
        });
        Some(format!("Window {}", window.to_usize()))
    }

    pub fn is_minimized_inner(&self, window: Id) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window.to_usize())
            .map(|s| s.minimized)
            .unwrap_or(false)
    }

    pub fn is_maximized_inner(&self, window: Id) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window.to_usize())
            .map(|s| s.maximized)
            .unwrap_or(false)
    }

    pub fn minimize_window_inner(&self, window: Id) -> Result<()> {
        self.log_operation(WindowApiCall::MinimizeWindow {
            window: window.to_usize(),
        });
        let mut states = self.window_states.lock().unwrap();
        states.entry(window.to_usize()).or_default().minimized = true;
        Ok(())
    }

    pub fn maximize_window_inner(&self, window: Id) -> Result<()> {
        self.log_operation(WindowApiCall::MaximizeWindow {
            window: window.to_usize(),
        });
        let mut states = self.window_states.lock().unwrap();
        states.entry(window.to_usize()).or_default().maximized = true;
        Ok(())
    }

    pub fn restore_window_inner(&self, window: Id) -> Result<()> {
        self.log_operation(WindowApiCall::RestoreWindow {
            window: window.to_usize(),
        });
        let mut states = self.window_states.lock().unwrap();
        if let Some(state) = states.get_mut(&window.to_usize()) {
            state.minimized = false;
            state.maximized = false;
        }
        Ok(())
    }

    pub fn close_window_inner(&self, window: Id) -> Result<()> {
        self.log_operation(WindowApiCall::CloseWindow {
            window: window.to_usize(),
        });
        self.window_rects.lock().unwrap().remove(&window.to_usize());
        self.window_states
            .lock()
            .unwrap()
            .remove(&window.to_usize());
        Ok(())
    }

    pub fn set_topmost_inner(&self, window: Id, topmost: bool) -> Result<()> {
        self.log_operation(WindowApiCall::SetTopmost {
            window: window.to_usize(),
            topmost,
        });
        let mut states = self.window_states.lock().unwrap();
        states.entry(window.to_usize()).or_default().topmost = topmost;
        Ok(())
    }

    pub fn is_topmost_inner(&self, window: Id) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window.to_usize())
            .map(|s| s.topmost)
            .unwrap_or(false)
    }
}

impl<Id: MockWindowId> Default for MockWindowApi<Id> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl<Id: MockWindowId> WindowApiBase for MockWindowApi<Id> {
    type WindowId = Id;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.log_operation(WindowApiCall::GetForegroundWindow);
        *self.foreground_window.lock().unwrap()
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        self.log_operation(WindowApiCall::SetWindowPos {
            window: window.to_usize(),
            x,
            y,
            width,
            height,
        });

        let mut rects = self.window_rects.lock().unwrap();
        rects.insert(window.to_usize(), WindowFrame::new(x, y, width, height));

        let mut states = self.window_states.lock().unwrap();
        if let Some(state) = states.get_mut(&window.to_usize()) {
            state.minimized = false;
            state.maximized = false;
        }

        Ok(())
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::MinimizeWindow {
            window: window.to_usize(),
        });
        let mut states = self.window_states.lock().unwrap();
        states.entry(window.to_usize()).or_default().minimized = true;
        Ok(())
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::MaximizeWindow {
            window: window.to_usize(),
        });
        let mut states = self.window_states.lock().unwrap();
        states.entry(window.to_usize()).or_default().maximized = true;
        Ok(())
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::RestoreWindow {
            window: window.to_usize(),
        });
        let mut states = self.window_states.lock().unwrap();
        if let Some(state) = states.get_mut(&window.to_usize()) {
            state.minimized = false;
            state.maximized = false;
        }
        Ok(())
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        self.log_operation(WindowApiCall::CloseWindow {
            window: window.to_usize(),
        });
        self.window_rects.lock().unwrap().remove(&window.to_usize());
        self.window_states
            .lock()
            .unwrap()
            .remove(&window.to_usize());
        Ok(())
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        self.log_operation(WindowApiCall::SetTopmost {
            window: window.to_usize(),
            topmost,
        });
        let mut states = self.window_states.lock().unwrap();
        states.entry(window.to_usize()).or_default().topmost = topmost;
        Ok(())
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window.to_usize())
            .map(|s| s.topmost)
            .unwrap_or(false)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        let fg = self.get_foreground_window();
        fg.and_then(|window| self.get_monitor_info(window))
            .map(|info| vec![info])
            .unwrap_or_default()
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        self.log_operation(WindowApiCall::IsWindow {
            window: window.to_usize(),
        });
        self.window_rects
            .lock()
            .unwrap()
            .contains_key(&window.to_usize())
    }

    fn is_minimized(&self, window: Self::WindowId) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window.to_usize())
            .map(|s| s.minimized)
            .unwrap_or(false)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window.to_usize())
            .map(|s| s.maximized)
            .unwrap_or(false)
    }
}

impl WindowOperations for MockWindowApi<usize> {
    fn get_window_info(
        &self,
        window: WindowId,
    ) -> Result<crate::platform::traits::WindowInfo> {
        let frame = self
            .window_rects
            .lock()
            .unwrap()
            .get(&window)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Failed to get window rect"))?;
        Ok(crate::platform::traits::WindowInfo {
            id: window,
            title: format!("Window {}", window),
            process_name: "TestProcess".to_string(),
            executable_path: None,
            x: frame.x,
            y: frame.y,
            width: frame.width,
            height: frame.height,
        })
    }

    fn set_window_pos(
        &self,
        window: WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        let mut rects = self.window_rects.lock().unwrap();
        rects.insert(window, WindowFrame::new(x, y, width, height));
        Ok(())
    }

    fn minimize_window(&self, window: WindowId) -> Result<()> {
        let mut states = self.window_states.lock().unwrap();
        states.entry(window).or_default().minimized = true;
        Ok(())
    }

    fn maximize_window(&self, window: WindowId) -> Result<()> {
        let mut states = self.window_states.lock().unwrap();
        states.entry(window).or_default().maximized = true;
        Ok(())
    }

    fn restore_window(&self, window: WindowId) -> Result<()> {
        let mut states = self.window_states.lock().unwrap();
        if let Some(state) = states.get_mut(&window) {
            state.minimized = false;
            state.maximized = false;
        }
        Ok(())
    }

    fn close_window(&self, window: WindowId) -> Result<()> {
        self.window_rects.lock().unwrap().remove(&window);
        self.window_states.lock().unwrap().remove(&window);
        Ok(())
    }

    fn set_topmost(&self, window: WindowId, topmost: bool) -> Result<()> {
        let mut states = self.window_states.lock().unwrap();
        states.entry(window).or_default().topmost = topmost;
        Ok(())
    }
}

impl WindowStateQueries for MockWindowApi<usize> {
    fn is_window_valid(&self, window: WindowId) -> bool {
        self.window_rects.lock().unwrap().contains_key(&window)
    }

    fn is_minimized(&self, window: WindowId) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window)
            .map(|s| s.minimized)
            .unwrap_or(false)
    }

    fn is_maximized(&self, window: WindowId) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window)
            .map(|s| s.maximized)
            .unwrap_or(false)
    }

    fn is_topmost(&self, window: WindowId) -> bool {
        self.window_states
            .lock()
            .unwrap()
            .get(&window)
            .map(|s| s.topmost)
            .unwrap_or(false)
    }
}

impl MonitorOperations for MockWindowApi<usize> {
    fn get_monitors(&self) -> Vec<MonitorInfo> {
        let fg = *self.foreground_window.lock().unwrap();
        fg.and_then(|w| self.monitor_info.lock().unwrap().get(&w).cloned())
            .map(|info| vec![info])
            .unwrap_or_default()
    }

    fn move_to_monitor(&self, _window: WindowId, _monitor_index: usize) -> Result<()> {
        Ok(())
    }
}

impl ForegroundWindowOperations for MockWindowApi<usize> {
    fn get_foreground_window(&self) -> Option<WindowId> {
        *self.foreground_window.lock().unwrap()
    }
}

impl WindowManagerTrait for MockWindowApi<usize> {}

impl CommonWindowApi for MockWindowApi<usize> {
    type WindowId = WindowId;
    type WindowInfo = crate::platform::traits::WindowInfo;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        <Self as ForegroundWindowOperations>::get_foreground_window(self)
    }

    fn get_window_info(&self, window: Self::WindowId) -> Result<Self::WindowInfo> {
        <Self as WindowOperations>::get_window_info(self, window)
    }

    fn set_window_pos(
        &self,
        window: Self::WindowId,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        <Self as WindowOperations>::set_window_pos(self, window, x, y, width, height)
    }

    fn minimize_window(&self, window: Self::WindowId) -> Result<()> {
        <Self as WindowOperations>::minimize_window(self, window)
    }

    fn maximize_window(&self, window: Self::WindowId) -> Result<()> {
        <Self as WindowOperations>::maximize_window(self, window)
    }

    fn restore_window(&self, window: Self::WindowId) -> Result<()> {
        <Self as WindowOperations>::restore_window(self, window)
    }

    fn close_window(&self, window: Self::WindowId) -> Result<()> {
        <Self as WindowOperations>::close_window(self, window)
    }

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        <Self as MonitorOperations>::get_monitors(self)
    }

    fn is_window_valid(&self, window: Self::WindowId) -> bool {
        <Self as WindowStateQueries>::is_window_valid(self, window)
    }

    fn is_maximized(&self, window: Self::WindowId) -> bool {
        <Self as WindowStateQueries>::is_maximized(self, window)
    }

    fn is_topmost(&self, window: Self::WindowId) -> bool {
        <Self as WindowStateQueries>::is_topmost(self, window)
    }

    fn set_topmost(&self, window: Self::WindowId, topmost: bool) -> Result<()> {
        <Self as WindowOperations>::set_topmost(self, window, topmost)
    }

    fn api(&self) -> &dyn std::any::Any {
        self
    }
}
