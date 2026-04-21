use anyhow::Result;
use tracing::debug;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, IsIconic,
};
use windows_core::BOOL;

// Import Edge and Alignment from types
use super::window_api::{MonitorInfo, MonitorWorkArea, RealWindowApi, WindowApi};
pub use crate::types::{Alignment, Edge};

/// Monitor direction (for moving between displays)
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MonitorDirection {
    Next,
    Prev,
    Index(i32),
}

/// Window frame information
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct WindowFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WindowFrame {
    /// Create new window frame
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create from RECT
    pub fn from_rect(rect: &RECT) -> Self {
        Self {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }
}

/// Window information
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: HWND,
    pub title: String,
    pub frame: WindowFrame,
    pub work_area: MonitorWorkArea,
}

#[allow(dead_code)]
/// Window manager (generic version)
pub struct WindowManager<A: WindowApi> {
    api: A,
}

/// Type alias for window manager using real Windows API
pub type RealWindowManager = WindowManager<RealWindowApi>;

impl WindowManager<RealWindowApi> {
    /// Create a window manager using real Windows API
    pub fn new() -> Self {
        Self {
            api: RealWindowApi::new(),
        }
    }
}

impl Default for WindowManager<RealWindowApi> {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl<A: WindowApi> WindowManager<A> {
    /// Create a window manager with specified API implementation
    pub fn with_api(api: A) -> Self {
        Self { api }
    }

    /// Get API reference (for testing)
    pub fn api(&self) -> &A {
        &self.api
    }

    /// Get foreground window information
    pub fn get_foreground_window_info(&self) -> Result<WindowInfo> {
        let hwnd = self
            .api
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;
        self.get_window_info(hwnd)
    }

    /// Get specified window information
    pub fn get_window_info(&self, hwnd: HWND) -> Result<WindowInfo> {
        if !self.api.is_window(hwnd) {
            return Err(anyhow::anyhow!("Invalid window handle"));
        }

        // Get window title
        let title = self.api.get_window_title(hwnd).unwrap_or_default();

        // Get window position
        let frame = self
            .api
            .get_window_rect(hwnd)
            .ok_or_else(|| anyhow::anyhow!("Failed to get window rect"))?;

        // Get monitor work area
        let work_area = self
            .api
            .get_monitor_work_area(hwnd)
            .ok_or_else(|| anyhow::anyhow!("Failed to get monitor work area"))?;

        debug!(
            "Window info: hwnd={:?}, title={}, frame={:?}, work_area={:?}",
            hwnd, title, frame, work_area
        );

        Ok(WindowInfo {
            hwnd,
            title,
            frame,
            work_area,
        })
    }

    /// Get debug info string
    pub fn get_debug_info(&self) -> Result<String> {
        let info = self.get_foreground_window_info()?;

        Ok(format!(
            "Window: {}\nID: {:?}\nPosition: [{}, {}]\nSize: {} x {}\nMonitor: [{} x {}]",
            info.title,
            info.hwnd,
            info.frame.x,
            info.frame.y,
            info.frame.width,
            info.frame.height,
            info.work_area.width,
            info.work_area.height
        ))
    }

    /// Set window position and size
    pub fn set_window_frame(&self, hwnd: HWND, frame: &WindowFrame) -> Result<()> {
        self.api.ensure_window_restored(hwnd)?;
        self.api
            .set_window_pos(hwnd, frame.x, frame.y, frame.width, frame.height)?;

        debug!(
            "Window moved to: x={}, y={}, width={}, height={}",
            frame.x, frame.y, frame.width, frame.height
        );

        Ok(())
    }

    /// Move window to center
    pub fn move_to_center(&self, hwnd: HWND) -> Result<()> {
        let info = self.get_window_info(hwnd)?;

        let new_x = info.work_area.x + (info.work_area.width - info.frame.width) / 2;
        let new_y = info.work_area.y + (info.work_area.height - info.frame.height) / 2;

        let new_frame =
            WindowFrame::new(new_x, new_y, info.frame.width, info.frame.height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// Move window to edge
    pub fn move_to_edge(&self, hwnd: HWND, edge: Edge) -> Result<()> {
        let info = self.get_window_info(hwnd)?;

        let (new_x, new_y) = match edge {
            Edge::Left => (info.work_area.x, info.frame.y),
            Edge::Right => (
                info.work_area.x + info.work_area.width - info.frame.width,
                info.frame.y,
            ),
            Edge::Top => (info.frame.x, info.work_area.y),
            Edge::Bottom => (
                info.frame.x,
                info.work_area.y + info.work_area.height - info.frame.height,
            ),
        };

        let new_frame =
            WindowFrame::new(new_x, new_y, info.frame.width, info.frame.height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// Set window to half screen
    pub fn set_half_screen(&self, hwnd: HWND, edge: Edge) -> Result<()> {
        let info = self.get_window_info(hwnd)?;

        let (new_x, new_y, new_width, new_height) = match edge {
            Edge::Left => (
                info.work_area.x,
                info.work_area.y,
                info.work_area.width / 2,
                info.work_area.height,
            ),
            Edge::Right => {
                let width = info.work_area.width / 2;
                (
                    info.work_area.x + info.work_area.width - width,
                    info.work_area.y,
                    width,
                    info.work_area.height,
                )
            }
            Edge::Top => (
                info.work_area.x,
                info.work_area.y,
                info.work_area.width,
                info.work_area.height / 2,
            ),
            Edge::Bottom => {
                let height = info.work_area.height / 2;
                (
                    info.work_area.x,
                    info.work_area.y + info.work_area.height - height,
                    info.work_area.width,
                    height,
                )
            }
        };

        let new_frame = WindowFrame::new(new_x, new_y, new_width, new_height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// Loop adjust window width
    pub fn loop_width(&self, hwnd: HWND, align: Alignment) -> Result<()> {
        const WIDTH_RATIOS: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        let info = self.get_window_info(hwnd)?;
        let current_ratio = info.frame.width as f32 / info.work_area.width as f32;

        // Find next ratio
        let mut next_ratio = WIDTH_RATIOS[0];
        for (i, ratio) in WIDTH_RATIOS.iter().enumerate() {
            if (current_ratio - ratio).abs() < 0.01 {
                next_ratio = WIDTH_RATIOS[(i + 1) % WIDTH_RATIOS.len()];
                break;
            }
        }

        let new_width = (info.work_area.width as f32 * next_ratio) as i32;
        let new_x = match align {
            Alignment::Left => info.work_area.x,
            Alignment::Right => info.work_area.x + info.work_area.width - new_width,
            _ => info.frame.x,
        };

        let new_frame =
            WindowFrame::new(new_x, info.frame.y, new_width, info.frame.height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// Loop adjust window height
    pub fn loop_height(&self, hwnd: HWND, align: Alignment) -> Result<()> {
        const HEIGHT_RATIOS: [f32; 3] = [0.75, 0.5, 0.25];

        let info = self.get_window_info(hwnd)?;
        let current_ratio = info.frame.height as f32 / info.work_area.height as f32;

        // Find next ratio
        let mut next_ratio = HEIGHT_RATIOS[0];
        for (i, ratio) in HEIGHT_RATIOS.iter().enumerate() {
            if (current_ratio - ratio).abs() < 0.01 {
                next_ratio = HEIGHT_RATIOS[(i + 1) % HEIGHT_RATIOS.len()];
                break;
            }
        }

        let new_height = (info.work_area.height as f32 * next_ratio) as i32;
        let new_y = match align {
            Alignment::Top => info.work_area.y,
            Alignment::Bottom => info.work_area.y + info.work_area.height - new_height,
            _ => info.frame.y,
        };

        let new_frame =
            WindowFrame::new(info.frame.x, new_y, info.frame.width, new_height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// Set fixed ratio window (centered) with loop support
    /// Automatically cycles through scales: 100% -> 90% -> 70% -> 50% -> 100%
    pub fn set_fixed_ratio(
        &self,
        hwnd: HWND,
        ratio: f32,
        _scale_index: usize, // Kept for API compatibility, but auto-detected
    ) -> Result<()> {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

        let info = self.get_window_info(hwnd)?;

        // Calculate base size based on the smaller side of work area
        let base_size = std::cmp::min(info.work_area.width, info.work_area.height);
        let base_width = (base_size as f32 * ratio) as i32;
        let base_height = base_size;

        // Calculate current scale based on window size
        let current_width_ratio = info.frame.width as f32 / base_width as f32;
        let current_height_ratio = info.frame.height as f32 / base_height as f32;
        let current_scale = (current_width_ratio + current_height_ratio) / 2.0;

        // Find next scale (loop through SCALES array)
        let mut next_scale = SCALES[0];
        for (i, scale) in SCALES.iter().enumerate() {
            if (current_scale - scale).abs() < 0.05 {
                // Found current scale, move to next
                next_scale = SCALES[(i + 1) % SCALES.len()];
                break;
            }
        }

        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;

        // Center
        let new_x = info.work_area.x + (info.work_area.width - new_width) / 2;
        let new_y = info.work_area.y + (info.work_area.height - new_height) / 2;

        let new_frame = WindowFrame::new(new_x, new_y, new_width, new_height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// Set native ratio window (based on screen ratio) with loop support
    /// Automatically cycles through scales: 100% -> 90% -> 70% -> 50% -> 100%
    pub fn set_native_ratio(&self, hwnd: HWND, _scale_index: usize) -> Result<()> {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

        let info = self.get_window_info(hwnd)?;

        // Calculate base size based on screen aspect ratio
        let screen_ratio = info.work_area.width as f32 / info.work_area.height as f32;
        let base_size = std::cmp::min(info.work_area.width, info.work_area.height);
        let base_width = (base_size as f32 * screen_ratio) as i32;
        let base_height = base_size;

        // Calculate current scale based on window size
        let current_width_ratio = info.frame.width as f32 / base_width as f32;
        let current_height_ratio = info.frame.height as f32 / base_height as f32;
        let current_scale = (current_width_ratio + current_height_ratio) / 2.0;

        // Find next scale (loop through SCALES array)
        let mut next_scale = SCALES[0];
        for (i, scale) in SCALES.iter().enumerate() {
            if (current_scale - scale).abs() < 0.05 {
                // Found current scale, move to next
                next_scale = SCALES[(i + 1) % SCALES.len()];
                break;
            }
        }

        let new_width = (base_width as f32 * next_scale) as i32;
        let new_height = (base_height as f32 * next_scale) as i32;

        // Center
        let new_x = info.work_area.x + (info.work_area.width - new_width) / 2;
        let new_y = info.work_area.y + (info.work_area.height - new_height) / 2;

        let new_frame = WindowFrame::new(new_x, new_y, new_width, new_height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// Minimize window
    pub fn minimize_window(&self, hwnd: HWND) -> Result<()> {
        self.api.minimize_window(hwnd)
    }

    /// Maximize window
    pub fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        self.api.maximize_window(hwnd)
    }

    /// Restore window
    pub fn restore_window(&self, hwnd: HWND) -> Result<()> {
        self.api.restore_window(hwnd)
    }

    /// Close window
    pub fn close_window(&self, hwnd: HWND) -> Result<()> {
        self.api.close_window(hwnd)
    }

    /// Toggle topmost state
    pub fn toggle_topmost(&self, hwnd: HWND) -> Result<bool> {
        // Get current state (by checking if operation succeeds)
        let current = self.api.is_window(hwnd);
        if !current {
            return Err(anyhow::anyhow!("Invalid window handle"));
        }

        // Toggle state - simplified here, should actually query current state
        let new_state = true; // Assume setting to topmost
        self.api.set_topmost(hwnd, new_state)?;
        Ok(new_state)
    }

    /// Set transparency
    pub fn set_opacity(&self, hwnd: HWND, opacity: u8) -> Result<()> {
        self.api.set_opacity(hwnd, opacity)
    }
}

/// Features requiring real Windows API (cross-monitor movement, window switching, etc.)
impl RealWindowManager {
    /// Move window to another monitor
    pub fn move_to_monitor(
        &self,
        hwnd: HWND,
        direction: MonitorDirection,
    ) -> Result<()> {
        unsafe {
            // Get all monitors
            let monitors = self.get_all_monitors();
            if monitors.len() < 2 {
                debug!("Only one monitor, nothing to do");
                return Ok(());
            }

            // Get current window's monitor index
            let current_monitor_index =
                self.get_current_monitor_index(hwnd, &monitors)?;

            // Calculate target monitor index
            let target_index = match direction {
                MonitorDirection::Next => (current_monitor_index + 1) % monitors.len(),
                MonitorDirection::Prev => {
                    if current_monitor_index == 0 {
                        monitors.len() - 1
                    } else {
                        current_monitor_index - 1
                    }
                }
                MonitorDirection::Index(idx) => {
                    let idx = idx as usize;
                    if idx >= monitors.len() {
                        return Err(anyhow::anyhow!("Invalid monitor index: {}", idx));
                    }
                    idx
                }
            };

            let target_monitor = &monitors[target_index];
            let current_monitor = &monitors[current_monitor_index];

            // Get current window info
            let info = self.get_window_info(hwnd)?;

            // Calculate relative position ratio
            let rel_x =
                (info.frame.x - current_monitor.x) as f32 / current_monitor.width as f32;
            let rel_y = (info.frame.y - current_monitor.y) as f32
                / current_monitor.height as f32;
            let rel_width = info.frame.width as f32 / current_monitor.width as f32;
            let rel_height = info.frame.height as f32 / current_monitor.height as f32;

            // Calculate new position (maintain relative position and size ratio)
            let new_x = target_monitor.x + (rel_x * target_monitor.width as f32) as i32;
            let new_y = target_monitor.y + (rel_y * target_monitor.height as f32) as i32;
            let new_width = (rel_width * target_monitor.width as f32) as i32;
            let new_height = (rel_height * target_monitor.height as f32) as i32;

            let new_frame = WindowFrame::new(new_x, new_y, new_width, new_height);
            self.set_window_frame(hwnd, &new_frame)?;

            debug!(
                "Moved window from monitor {} to monitor {}: {:?}",
                current_monitor_index, target_index, new_frame
            );

            Ok(())
        }
    }

    /// Get all monitor information
    unsafe fn get_all_monitors(&self) -> Vec<MonitorInfo> {
        use windows::Win32::Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
        };

        struct EnumData {
            monitors: Vec<MonitorInfo>,
        }

        unsafe extern "system" fn enum_callback(
            hmonitor: HMONITOR,
            _hdc: HDC,
            _rect: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let data = &mut *(lparam.0 as *mut EnumData);

            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };

            if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
                let work_area = &monitor_info.rcWork;
                data.monitors.push(MonitorInfo {
                    x: work_area.left,
                    y: work_area.top,
                    width: work_area.right - work_area.left,
                    height: work_area.bottom - work_area.top,
                });
            }

            BOOL(1) // Continue enumeration
        }

        let mut data = EnumData {
            monitors: Vec::new(),
        };

        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(enum_callback),
            LPARAM(&mut data as *mut _ as isize),
        );

        data.monitors
    }

    /// Get the index of the monitor where the window is currently located
    unsafe fn get_current_monitor_index(
        &self,
        hwnd: HWND,
        monitors: &[MonitorInfo],
    ) -> Result<usize> {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect)?;

        let window_center_x = rect.left + (rect.right - rect.left) / 2;
        let window_center_y = rect.top + (rect.bottom - rect.top) / 2;

        for (i, monitor) in monitors.iter().enumerate() {
            if window_center_x >= monitor.x
                && window_center_x < monitor.x + monitor.width
                && window_center_y >= monitor.y
                && window_center_y < monitor.y + monitor.height
            {
                return Ok(i);
            }
        }

        // Default to first monitor
        Ok(0)
    }

    /// Switch to next window of same process (Alt+` function)
    pub fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        unsafe {
            let current_hwnd = GetForegroundWindow();
            if current_hwnd.0.is_null() {
                return Err(anyhow::anyhow!("No foreground window"));
            }

            // Get current window's process ID
            let current_pid = self.get_window_process_id(current_hwnd)?;
            debug!("Current window PID: {}", current_pid);

            // Get all visible windows of this process
            let windows = self.get_process_visible_windows(current_pid);
            if windows.len() < 2 {
                debug!("Only one window in process, nothing to switch");
                return Ok(());
            }

            // Sort by Z-Order (from front to back)
            let sorted_windows = self.sort_windows_by_zorder(windows);

            // Find current window index
            let current_index = sorted_windows
                .iter()
                .position(|&hwnd| hwnd == current_hwnd)
                .unwrap_or(0);

            // Switch to next window
            let next_index = (current_index + 1) % sorted_windows.len();
            let next_hwnd = sorted_windows[next_index];

            debug!(
                "Switching from {:?} to {:?} (index {} -> {})",
                current_hwnd, next_hwnd, current_index, next_index
            );

            self.activate_window(next_hwnd)?;

            Ok(())
        }
    }

    /// Get window process ID
    unsafe fn get_window_process_id(&self, hwnd: HWND) -> Result<u32> {
        let mut pid: u32 = 0;
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
            hwnd,
            Some(&mut pid),
        );

        if pid == 0 {
            return Err(anyhow::anyhow!("Failed to get process ID"));
        }

        Ok(pid)
    }

    /// Get all visible windows of specified process
    fn get_process_visible_windows(&self, target_pid: u32) -> Vec<HWND> {
        use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, IsWindowVisible};

        struct EnumData {
            target_pid: u32,
            windows: Vec<HWND>,
        }

        unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let data = &mut *(lparam.0 as *mut EnumData);

            // Check if window is visible
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1); // Continue enumeration
            }

            // Get window process ID
            let mut pid: u32 = 0;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                hwnd,
                Some(&mut pid),
            );

            if pid == data.target_pid {
                data.windows.push(hwnd);
            }

            BOOL(1) // Continue enumeration
        }

        unsafe {
            let mut data = EnumData {
                target_pid,
                windows: Vec::new(),
            };

            let _ =
                EnumWindows(Some(enum_callback), LPARAM(&mut data as *mut _ as isize));

            data.windows
        }
    }

    /// Sort windows by Z-Order (from front to back)
    fn sort_windows_by_zorder(&self, windows: Vec<HWND>) -> Vec<HWND> {
        use windows::Win32::UI::WindowsAndMessaging::GetWindow;

        unsafe {
            // Get Z-Order positions of all windows
            // Method: Start from topmost window and traverse, recording position of each window
            let mut zorder_map: std::collections::HashMap<isize, usize> =
                std::collections::HashMap::new();

            let mut hwnd = GetWindow(
                HWND(std::ptr::null_mut()),
                windows::Win32::UI::WindowsAndMessaging::GW_HWNDFIRST,
            );

            let mut z_index: usize = 0;
            while let Ok(h) = hwnd {
                if windows.contains(&h) {
                    zorder_map.insert(h.0 as isize, z_index);
                }
                hwnd =
                    GetWindow(h, windows::Win32::UI::WindowsAndMessaging::GW_HWNDNEXT);
                z_index += 1;
            }

            // Sort by Z-Order
            let mut sorted = windows;
            sorted.sort_by_key(|hwnd| {
                zorder_map
                    .get(&(hwnd.0 as isize))
                    .copied()
                    .unwrap_or(usize::MAX)
            });

            sorted
        }
    }

    /// Activate window (switch to foreground)
    unsafe fn activate_window(&self, hwnd: HWND) -> Result<()> {
        use windows::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, SetForegroundWindow, ShowWindow, SW_RESTORE,
        };

        // If window is minimized, restore it first
        if IsIconic(hwnd).as_bool() {
            ShowWindow(hwnd, SW_RESTORE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to restore window: {}", e))?;
        }

        // Switch to foreground
        let _ = BringWindowToTop(hwnd);

        SetForegroundWindow(hwnd)
            .ok()
            .map_err(|e| anyhow::anyhow!("Failed to set foreground window: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::MockWindowApi;
    use super::*;

    fn test_hwnd(value: usize) -> HWND {
        HWND(value as *mut core::ffi::c_void)
    }

    #[test]
    fn test_window_manager_creation() {
        let api = MockWindowApi::new();
        let wm = WindowManager::with_api(api);

        // Verify creation success
        assert!(wm.api().is_window(test_hwnd(0)) == false);
    }

    #[test]
    fn test_get_window_info() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set test data
        api.set_window_rect(hwnd, WindowFrame::new(100, 200, 800, 600));
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        let wm = WindowManager::with_api(api);
        let info = wm.get_window_info(hwnd).unwrap();

        assert_eq!(info.frame.x, 100);
        assert_eq!(info.frame.y, 200);
        assert_eq!(info.frame.width, 800);
        assert_eq!(info.frame.height, 600);
    }

    #[test]
    fn test_move_to_center() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set test data - 800x600 window on 1920x1080 monitor
        api.set_window_rect(hwnd, WindowFrame::new(0, 0, 800, 600));
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        let wm = WindowManager::with_api(api);
        wm.move_to_center(hwnd).unwrap();

        // Verify window position (1920-800)/2 = 560, (1080-600)/2 = 240
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 560);
        assert_eq!(frame.y, 240);
    }

    #[test]
    fn test_move_to_edge() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test left edge
        wm.move_to_edge(hwnd, Edge::Left).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 0);

        // Test right edge
        wm.move_to_edge(hwnd, Edge::Right).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 1920 - 800);
    }

    #[test]
    fn test_set_half_screen() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        let wm = WindowManager::with_api(api);

        // Test left half screen
        wm.set_half_screen(hwnd, Edge::Left).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 0);
        assert_eq!(frame.y, 0);
        assert_eq!(frame.width, 960); // 1920 / 2
        assert_eq!(frame.height, 1080);

        // Test right half screen
        wm.set_half_screen(hwnd, Edge::Right).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 960);
        assert_eq!(frame.width, 960);
    }

    #[test]
    fn test_loop_width() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set all data before creating WindowManager
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );
        api.set_window_rect(hwnd, WindowFrame::new(0, 0, 960, 600));

        let wm = WindowManager::with_api(api);

        // Test cycle from 50%
        wm.loop_width(hwnd, Alignment::Left).unwrap();

        let frame = wm.api().get_window_rect(hwnd).unwrap();
        // 50% -> 40% = 768
        assert_eq!(frame.width, 768);
    }

    #[test]
    fn test_set_fixed_ratio() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        // Set all data before creating WindowManager
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );
        // Need to set an initial window size first
        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);

        // Test 4:3 ratio, 100% scale
        wm.set_fixed_ratio(hwnd, 4.0 / 3.0, 0).unwrap();

        let frame = wm.api().get_window_rect(hwnd).unwrap();
        // Based on smaller side 1080, 4:3 ratio, width = 1080 * 4/3 = 1440
        assert_eq!(frame.width, 1440);
        assert_eq!(frame.height, 1080);
    }

    #[test]
    fn test_window_state_operations() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);

        // Test minimize
        wm.minimize_window(hwnd).unwrap();
        assert!(wm.api().is_iconic(hwnd));

        // Test restore
        wm.restore_window(hwnd).unwrap();
        assert!(!wm.api().is_iconic(hwnd));

        // Test maximize
        wm.maximize_window(hwnd).unwrap();
        assert!(wm.api().is_zoomed(hwnd));
    }

    #[test]
    fn test_close_window() {
        let api = MockWindowApi::new();
        let hwnd = test_hwnd(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);
        assert!(wm.api().is_window(hwnd));

        wm.close_window(hwnd).unwrap();

        // Window should be removed
        assert!(!wm.api().is_window(hwnd));
    }
}
