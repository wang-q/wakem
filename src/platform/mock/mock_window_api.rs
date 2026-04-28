//! Mock window API for testing
//!
//! Provides a mock implementation of WindowApiBase that can be used
//! in unit tests without platform dependencies.

use crate::platform::mock::MockWindowId;
use crate::platform::traits::{MonitorInfo, WindowApiBase, WindowFrame};
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
pub struct MockWindowState {
    minimized: bool,
    maximized: bool,
    topmost: bool,
}

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

#[cfg(test)]
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

    fn log_operation(&self, op: WindowApiCall) {
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

    fn get_monitor_info(&self, window: Id) -> Option<MonitorInfo> {
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

#[cfg(test)]
impl<Id: MockWindowId> Default for MockWindowApi<Id> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl<Id: MockWindowId> WindowApiBase for MockWindowApi<Id> {
    type WindowId = Id;

    fn get_foreground_window(&self) -> Option<Self::WindowId> {
        self.get_foreground_window_inner()
    }

    fn get_window_info(
        &self,
        window: Self::WindowId,
    ) -> Result<crate::platform::traits::WindowInfo> {
        let title = self.get_window_title(window).unwrap_or_default();
        let frame = self
            .get_window_rect(window)
            .ok_or_else(|| anyhow::anyhow!("Failed to get window rect"))?;
        Ok(crate::platform::traits::WindowInfo {
            id: window.to_usize(),
            title,
            process_name: "TestProcess".to_string(),
            executable_path: None,
            x: frame.x,
            y: frame.y,
            width: frame.width,
            height: frame.height,
        })
    }

    crate::impl_window_api_base_inner!();

    fn get_monitors(&self) -> Vec<MonitorInfo> {
        let fg = self.get_foreground_window_inner();
        fg.and_then(|window| self.get_monitor_info(window))
            .map(|info| vec![info])
            .unwrap_or_default()
    }

    fn move_to_monitor(
        &self,
        _window: Self::WindowId,
        _monitor_index: usize,
    ) -> Result<()> {
        Ok(())
    }

    fn window_id_to_usize(id: Self::WindowId) -> usize {
        id.to_usize()
    }

    fn usize_to_window_id(id: usize) -> Self::WindowId {
        Id::from_usize(id)
    }
}
