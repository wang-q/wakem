//! Cross-platform mock implementations for testing
//!
//! This module provides mock implementations of platform-specific traits
//! that can be used for testing on any platform.

use crate::platform::traits::{InputDeviceTrait, WindowId};
use crate::types::{
    InputEvent, KeyEvent, KeyState, ModifierState, MouseButton, MouseEvent,
    MouseEventType,
};
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Macro to generate test-mode [OutputDeviceTrait] implementation.
///
/// Eliminates duplicated `#[cfg(test)]` boilerplate across platforms.
/// Each method logs a `[TEST MODE]` message and returns `Ok(())`.
///
/// # Usage
///
/// ```ignore
/// // In macos/output_device.rs or windows/output_device.rs:
/// crate::platform::mock::impl_test_output_device!(MyOutputDevice);
/// ```
#[macro_export]
macro_rules! impl_test_output_device {
    ($device:ty) => {
        #[cfg(test)]
        impl OutputDeviceTrait for $device {
            fn send_key(
                &self,
                _scan_code: u16,
                _virtual_key: u16,
                _release: bool,
            ) -> Result<()> {
                tracing::debug!(
                    "[TEST MODE] Mock key event: scan={:#04X}, vk={:#04X}, release={}",
                    _scan_code,
                    _virtual_key,
                    _release
                );
                Ok(())
            }

            fn send_mouse_move(&self, _x: i32, _y: i32, _relative: bool) -> Result<()> {
                tracing::debug!(
                    "[TEST MODE] Mock mouse move: x={}, y={}, relative={}",
                    _x,
                    _y,
                    _relative
                );
                Ok(())
            }

            fn send_mouse_button(
                &self,
                _button: MouseButton,
                _release: bool,
            ) -> Result<()> {
                tracing::debug!(
                    "[TEST MODE] Mock mouse button: {:?}, release={}",
                    _button,
                    _release
                );
                Ok(())
            }

            fn send_mouse_wheel(&self, _delta: i32, _horizontal: bool) -> Result<()> {
                tracing::debug!(
                    "[TEST MODE] Mock mouse wheel: delta={}, horizontal={}",
                    _delta,
                    _horizontal
                );
                Ok(())
            }
        }
    };
}

/// Mock input device for testing
///
/// Uses `Arc<Mutex<>>` for thread-safe interior mutability, consistent
/// with [MockOutputDevice].
#[cfg(test)]
pub struct MockInputDevice {
    state: Arc<Mutex<MockInputState>>,
}

#[cfg(test)]
struct MockInputState {
    events: VecDeque<InputEvent>,
    running: bool,
    modifier_state: ModifierState,
    captured_events: Vec<InputEvent>,
}

#[cfg(test)]
impl MockInputDevice {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockInputState {
                events: VecDeque::new(),
                running: false,
                modifier_state: ModifierState::default(),
                captured_events: Vec::new(),
            })),
        }
    }

    pub fn inject_key_press(&self, scan_code: u16, virtual_key: u16) {
        let event = KeyEvent::new(scan_code, virtual_key, KeyState::Pressed);
        self.state
            .lock()
            .unwrap()
            .events
            .push_back(InputEvent::Key(event));
    }

    pub fn inject_key_release(&self, scan_code: u16, virtual_key: u16) {
        let event = KeyEvent::new(scan_code, virtual_key, KeyState::Released);
        self.state
            .lock()
            .unwrap()
            .events
            .push_back(InputEvent::Key(event));
    }

    pub fn inject_mouse_move(&self, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::Move { relative: false }, x, y);
        self.state
            .lock()
            .unwrap()
            .events
            .push_back(InputEvent::Mouse(event));
    }

    pub fn inject_mouse_button_down(&self, button: MouseButton, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::ButtonDown(button), x, y);
        self.state
            .lock()
            .unwrap()
            .events
            .push_back(InputEvent::Mouse(event));
    }

    pub fn inject_mouse_button_up(&self, button: MouseButton, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::ButtonUp(button), x, y);
        self.state
            .lock()
            .unwrap()
            .events
            .push_back(InputEvent::Mouse(event));
    }

    pub fn inject_wheel(&self, delta: i32, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::Wheel(delta), x, y);
        self.state
            .lock()
            .unwrap()
            .events
            .push_back(InputEvent::Mouse(event));
    }

    pub fn inject_hwheel(&self, delta: i32, x: i32, y: i32) {
        let event = MouseEvent::new(MouseEventType::HWheel(delta), x, y);
        self.state
            .lock()
            .unwrap()
            .events
            .push_back(InputEvent::Mouse(event));
    }

    pub fn get_captured_events(&self) -> Vec<InputEvent> {
        self.state.lock().unwrap().captured_events.clone()
    }

    pub fn pending_count(&self) -> usize {
        self.state.lock().unwrap().events.len()
    }

    pub fn clear(&self) {
        self.state.lock().unwrap().events.clear();
    }

    pub fn get_modifier_state(&self) -> ModifierState {
        self.state.lock().unwrap().modifier_state
    }
}

#[cfg(test)]
impl Default for MockInputDevice {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl InputDeviceTrait for MockInputDevice {
    fn register(&mut self) -> Result<()> {
        self.state.lock().unwrap().running = true;
        Ok(())
    }

    fn unregister(&mut self) {
        self.state.lock().unwrap().running = false;
    }

    fn poll_event(&mut self) -> Option<InputEvent> {
        let mut state = self.state.lock().unwrap();
        if !state.running {
            return None;
        }

        let event = state.events.pop_front();

        if let Some(ref e) = event {
            state.captured_events.push(e.clone());

            if let InputEvent::Key(key_event) = e {
                state.modifier_state.apply_from_virtual_key(
                    key_event.virtual_key,
                    key_event.state == KeyState::Pressed,
                );
            }
        }

        event
    }

    fn is_running(&self) -> bool {
        self.state.lock().unwrap().running
    }

    fn stop(&mut self) {
        self.state.lock().unwrap().running = false;
    }
}

/// Trait for converting platform-specific window IDs to usize for mock storage.
///
/// Windows uses HWND (a pointer type) which doesn't implement Hash/Eq,
/// so we convert to usize for internal storage. macOS uses usize directly.
#[cfg(test)]
pub trait MockWindowId: Copy + std::fmt::Debug + 'static {
    fn to_usize(self) -> usize;
    fn from_usize(v: usize) -> Self;
}

#[cfg(test)]
impl MockWindowId for WindowId {
    fn to_usize(self) -> usize {
        self
    }
    fn from_usize(v: usize) -> Self {
        v
    }
}

#[cfg(all(test, target_os = "windows"))]
impl MockWindowId for windows::Win32::Foundation::HWND {
    fn to_usize(self) -> usize {
        self.0 as usize
    }
    fn from_usize(v: usize) -> Self {
        Self(v as *mut std::ffi::c_void)
    }
}

/// Generic mock window API for testing
///
/// Uses `Id: MockWindowId` to support both `usize` (macOS) and `HWND` (Windows)
/// as window identifier types, while storing data internally using `usize` keys.
#[cfg(test)]
mod mock_window_api {
    use super::MockWindowId;
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
}

#[cfg(test)]
pub use mock_window_api::{MockWindowApi, WindowApiCall};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_input_device_creation() {
        let device = MockInputDevice::new();
        assert!(!device.is_running());
        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_mock_input_device_register() {
        let mut device = MockInputDevice::new();
        assert!(!device.is_running());

        device.register().unwrap();
        assert!(device.is_running());

        device.unregister();
        assert!(!device.is_running());
    }

    #[test]
    fn test_mock_inject_key_events() {
        let device = MockInputDevice::new();

        device.inject_key_press(0x1E, 0x41); // 'A' key
        device.inject_key_release(0x1E, 0x41);

        assert_eq!(device.pending_count(), 2);
    }

    #[test]
    fn test_mock_poll_key_events() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        device.inject_key_press(0x1E, 0x41);
        device.inject_key_release(0x1E, 0x41);

        // Poll the first event
        let event1 = device.poll_event().unwrap();
        assert!(matches!(
            event1,
            InputEvent::Key(KeyEvent {
                state: KeyState::Pressed,
                ..
            })
        ));

        // Poll second event
        let event2 = device.poll_event().unwrap();
        assert!(matches!(
            event2,
            InputEvent::Key(KeyEvent {
                state: KeyState::Released,
                ..
            })
        ));

        // No more events
        assert!(device.poll_event().is_none());
    }

    #[test]
    fn test_mock_poll_without_register() {
        let mut device = MockInputDevice::new();
        device.inject_key_press(0x1E, 0x41);

        // Should return None when not registered
        assert!(device.poll_event().is_none());
    }

    #[test]
    fn test_mock_inject_mouse_events() {
        let device = MockInputDevice::new();

        device.inject_mouse_move(100, 200);
        device.inject_mouse_button_down(MouseButton::Left, 100, 200);
        device.inject_mouse_button_up(MouseButton::Left, 100, 200);
        device.inject_wheel(120, 100, 200);

        assert_eq!(device.pending_count(), 4);
    }

    #[test]
    fn test_mock_captured_events() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        device.inject_key_press(0x1E, 0x41);
        device.inject_key_release(0x1E, 0x41);

        // Poll all events
        let _ = device.poll_event();
        let _ = device.poll_event();

        // Check captured events
        let captured = device.get_captured_events();
        assert_eq!(captured.len(), 2);
    }

    #[test]
    fn test_mock_clear() {
        let device = MockInputDevice::new();

        device.inject_key_press(0x1E, 0x41);
        device.inject_key_press(0x30, 0x42);
        assert_eq!(device.pending_count(), 2);

        device.clear();
        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_input_device_config_default() {
        let config = crate::platform::traits::InputDeviceConfig::default();
        assert!(config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(!config.block_legacy_input);
    }

    // ==================== Edge case and error path tests ====================

    #[test]
    fn test_mock_poll_empty_device() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        // Empty device should return None
        assert!(device.poll_event().is_none());

        // Multiple polls on empty device should all return None
        for _ in 0..10 {
            assert!(device.poll_event().is_none());
        }
    }

    #[test]
    fn test_mock_poll_unregistered_device() {
        let mut device = MockInputDevice::new();

        // Unregistered device should return None
        assert!(device.poll_event().is_none());

        // Inject events but not registered, should still return None
        device.inject_key_press(0x1E, 0x41);
        assert!(device.poll_event().is_none());
    }

    #[test]
    fn test_mock_rapid_register_unregister() {
        let mut device = MockInputDevice::new();

        // Rapid register/unregister
        for _ in 0..100 {
            device.register().unwrap();
            assert!(device.is_running());
            device.unregister();
            assert!(!device.is_running());
        }
    }

    #[test]
    fn test_mock_large_event_batch() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        // Inject large batch of events
        for i in 0..1000 {
            device.inject_key_press(i as u16, 0x41 + i as u16);
        }

        assert_eq!(device.pending_count(), 1000);

        // Poll all events
        let mut polled_count = 0;
        while device.poll_event().is_some() {
            polled_count += 1;
            if polled_count > 1100 {
                panic!("Polled more events than injected (possible infinite loop)");
            }
        }

        assert_eq!(polled_count, 1000);
        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_mock_mixed_event_types() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        // Inject mixed event types
        device.inject_key_press(0x3A, 0x14); // CapsLock
        device.inject_mouse_move(100, 200);
        device.inject_key_release(0x3A, 0x14);
        device.inject_mouse_button_down(MouseButton::Left, 150, 250);
        device.inject_mouse_button_up(MouseButton::Left, 150, 250);
        device.inject_wheel(120, 150, 250);
        device.inject_hwheel(-60, 150, 250);

        assert_eq!(device.pending_count(), 7);

        // Verify event order and types
        if let InputEvent::Key(event) = device.poll_event().unwrap() {
            assert_eq!(event.state, KeyState::Pressed);
            assert_eq!(event.scan_code, 0x3A);
        } else {
            panic!("Expected Key event");
        }

        if let InputEvent::Mouse(mouse) = device.poll_event().unwrap() {
            assert!(matches!(
                mouse.event_type,
                MouseEventType::Move { relative: false }
            ));
            assert_eq!(mouse.x, 100);
            assert_eq!(mouse.y, 200);
        } else {
            panic!("Expected Mouse Move event");
        }
    }

    #[test]
    fn test_mock_concurrent_access_simulation() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        // Simulate rapid consecutive injection of different event types
        for round in 0..100 {
            match round % 3 {
                0 => device.inject_key_press(round as u16, round as u16),
                1 => device.inject_mouse_move(round * 10, round * 20),
                2 => device.inject_wheel(round, 0, 0),
                _ => unreachable!(),
            }
        }

        assert_eq!(device.pending_count(), 100);

        // Rapid clear and refill
        for _ in 0..10 {
            device.clear();
            for i in 0..50 {
                device.inject_key_press(i as u16, i as u16);
            }
            assert_eq!(device.pending_count(), 50);
            device.clear();
        }

        assert_eq!(device.pending_count(), 0);
    }

    #[test]
    fn test_mock_modifier_state_tracking() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        // Initial state has no modifiers
        let initial_state = device.get_modifier_state();
        assert!(!initial_state.shift);
        assert!(!initial_state.ctrl);
        assert!(!initial_state.alt);
        assert!(!initial_state.meta);

        // Press Ctrl
        device.inject_key_press(0x1D, 0x11); // Ctrl
        let _ = device.poll_event();
        let state_after_ctrl = device.get_modifier_state();
        assert!(state_after_ctrl.ctrl); // Ctrl should be set

        // Press Shift (Ctrl should remain)
        device.inject_key_press(0x2A, 0xA0); // LShift
        let _ = device.poll_event();
        let state_after_shift = device.get_modifier_state();
        assert!(state_after_shift.ctrl); // Ctrl remains
        assert!(state_after_shift.shift); // Shift is set

        // Release Ctrl
        device.inject_key_release(0x1D, 0x11); // Release Ctrl
        let _ = device.poll_event();
        let state_after_release = device.get_modifier_state();
        assert!(!state_after_release.ctrl); // Ctrl should be cleared on release
        assert!(state_after_shift.shift); // Shift should still be set
    }

    #[test]
    fn test_mock_captured_events_ordering() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        // Inject ordered sequence of events
        for i in 0..5 {
            device.inject_key_press(0x1E + i, 0x41 + i); // A, B, C, D, E
        }

        // Verify captured events maintain order
        for i in 0..5 {
            let event = device.poll_event().unwrap();
            if let InputEvent::Key(key) = event {
                assert_eq!(key.scan_code, 0x1E + i);
                assert_eq!(key.virtual_key, 0x41 + i);
                assert_eq!(key.state, KeyState::Pressed);
            } else {
                panic!("Expected Key event at index {}", i);
            }
        }

        // Verify get_captured_events also maintains same order
        let captured = device.get_captured_events();
        assert_eq!(captured.len(), 5);
        for (i, event) in captured.iter().enumerate() {
            if let InputEvent::Key(key) = event {
                assert_eq!(key.scan_code, 0x1E + i as u16);
            } else {
                panic!("Captured event {} should be Key", i);
            }
        }
    }

    #[test]
    fn test_mock_extreme_scan_codes() {
        let mut device = MockInputDevice::new();
        device.register().unwrap();

        // Test boundary scan code values
        device.inject_key_press(0x0000, 0x00); // Minimum scan code
        device.inject_key_press(0x00FF, 0xFF); // Maximum scan code
        device.inject_key_press(0xE05B, 0x5B); // Extended key (LWin)

        assert_eq!(device.pending_count(), 3);

        // Verify extreme scan codes are handled correctly
        let event_min = device.poll_event().unwrap();
        if let InputEvent::Key(key) = event_min {
            assert_eq!(key.scan_code, 0x0000);
        }

        let event_max = device.poll_event().unwrap();
        if let InputEvent::Key(key) = event_max {
            assert_eq!(key.scan_code, 0x00FF);
        }

        let event_extended = device.poll_event().unwrap();
        if let InputEvent::Key(key) = event_extended {
            assert_eq!(key.scan_code, 0xE05B);
        }
    }
}
