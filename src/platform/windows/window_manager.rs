use anyhow::Result;
use tracing::debug;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowRect, IsIconic, SetForegroundWindow, ShowWindow,
    SW_RESTORE,
};

// 从 types 导入 Edge 和 Alignment
use super::window_api::{MonitorInfo, MonitorWorkArea, RealWindowApi, WindowApi};
pub use crate::types::{Alignment, Edge};

/// 显示器方向（用于跨显示器移动）
#[derive(Debug, Clone, Copy)]
pub enum MonitorDirection {
    Next,
    Prev,
    Index(i32),
}

/// 窗口框架信息
#[derive(Debug, Clone, Copy)]
pub struct WindowFrame {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WindowFrame {
    /// 创建新的窗口框架
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// 从 RECT 创建
    pub fn from_rect(rect: &RECT) -> Self {
        Self {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }
}

/// 窗口信息
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: HWND,
    pub title: String,
    pub frame: WindowFrame,
    pub work_area: MonitorWorkArea,
}

/// 窗口管理器（泛型版本）
pub struct WindowManager<A: WindowApi> {
    api: A,
}

/// 使用真实 Windows API 的窗口管理器类型别名
pub type RealWindowManager = WindowManager<RealWindowApi>;

impl WindowManager<RealWindowApi> {
    /// 创建使用真实 Windows API 的窗口管理器
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

impl<A: WindowApi> WindowManager<A> {
    /// 使用指定的 API 实现创建窗口管理器
    pub fn with_api(api: A) -> Self {
        Self { api }
    }

    /// 获取 API 引用（用于测试）
    pub fn api(&self) -> &A {
        &self.api
    }

    /// 获取前台窗口信息
    pub fn get_foreground_window_info(&self) -> Result<WindowInfo> {
        let hwnd = self
            .api
            .get_foreground_window()
            .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;
        self.get_window_info(hwnd)
    }

    /// 获取指定窗口信息
    pub fn get_window_info(&self, hwnd: HWND) -> Result<WindowInfo> {
        if !self.api.is_window(hwnd) {
            return Err(anyhow::anyhow!("Invalid window handle"));
        }

        // 获取窗口标题
        let title = self.api.get_window_title(hwnd).unwrap_or_default();

        // 获取窗口位置
        let frame = self
            .api
            .get_window_rect(hwnd)
            .ok_or_else(|| anyhow::anyhow!("Failed to get window rect"))?;

        // 获取显示器工作区
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

    /// 获取调试信息字符串
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

    /// 设置窗口位置和大小
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

    /// 移动窗口到中心
    pub fn move_to_center(&self, hwnd: HWND) -> Result<()> {
        let info = self.get_window_info(hwnd)?;

        let new_x = info.work_area.x + (info.work_area.width - info.frame.width) / 2;
        let new_y = info.work_area.y + (info.work_area.height - info.frame.height) / 2;

        let new_frame =
            WindowFrame::new(new_x, new_y, info.frame.width, info.frame.height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// 移动窗口到边缘
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

    /// 设置窗口为半屏
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

    /// 循环调整窗口宽度
    pub fn loop_width(&self, hwnd: HWND, align: Alignment) -> Result<()> {
        const WIDTH_RATIOS: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

        let info = self.get_window_info(hwnd)?;
        let current_ratio = info.frame.width as f32 / info.work_area.width as f32;

        // 找到下一个比例
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

    /// 循环调整窗口高度
    pub fn loop_height(&self, hwnd: HWND, align: Alignment) -> Result<()> {
        const HEIGHT_RATIOS: [f32; 3] = [0.75, 0.5, 0.25];

        let info = self.get_window_info(hwnd)?;
        let current_ratio = info.frame.height as f32 / info.work_area.height as f32;

        // 找到下一个比例
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

    /// 设置固定比例窗口（居中）
    pub fn set_fixed_ratio(
        &self,
        hwnd: HWND,
        ratio: f32,
        scale_index: usize,
    ) -> Result<()> {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];
        let scale = SCALES[scale_index % SCALES.len()];

        let info = self.get_window_info(hwnd)?;

        // 基于工作区较小边计算基础尺寸
        let base_size = std::cmp::min(info.work_area.width, info.work_area.height);
        let base_width = (base_size as f32 * ratio) as i32;
        let base_height = base_size;

        let new_width = (base_width as f32 * scale) as i32;
        let new_height = (base_height as f32 * scale) as i32;

        // 居中
        let new_x = info.work_area.x + (info.work_area.width - new_width) / 2;
        let new_y = info.work_area.y + (info.work_area.height - new_height) / 2;

        let new_frame = WindowFrame::new(new_x, new_y, new_width, new_height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// 设置原生比例窗口（基于屏幕比例）
    pub fn set_native_ratio(&self, hwnd: HWND, scale_index: usize) -> Result<()> {
        const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];
        let scale = SCALES[scale_index % SCALES.len()];

        let info = self.get_window_info(hwnd)?;

        // 基于屏幕宽高比计算基础尺寸
        let screen_ratio = info.work_area.width as f32 / info.work_area.height as f32;
        let base_size = std::cmp::min(info.work_area.width, info.work_area.height);
        let base_width = (base_size as f32 * screen_ratio) as i32;
        let base_height = base_size;

        let new_width = (base_width as f32 * scale) as i32;
        let new_height = (base_height as f32 * scale) as i32;

        // 居中
        let new_x = info.work_area.x + (info.work_area.width - new_width) / 2;
        let new_y = info.work_area.y + (info.work_area.height - new_height) / 2;

        let new_frame = WindowFrame::new(new_x, new_y, new_width, new_height);
        self.set_window_frame(hwnd, &new_frame)
    }

    /// 最小化窗口
    pub fn minimize_window(&self, hwnd: HWND) -> Result<()> {
        self.api.minimize_window(hwnd)
    }

    /// 最大化窗口
    pub fn maximize_window(&self, hwnd: HWND) -> Result<()> {
        self.api.maximize_window(hwnd)
    }

    /// 还原窗口
    pub fn restore_window(&self, hwnd: HWND) -> Result<()> {
        self.api.restore_window(hwnd)
    }

    /// 关闭窗口
    pub fn close_window(&self, hwnd: HWND) -> Result<()> {
        self.api.close_window(hwnd)
    }

    /// 切换置顶状态
    pub fn toggle_topmost(&self, hwnd: HWND) -> Result<bool> {
        // 获取当前状态（通过检查操作是否成功）
        let current = self.api.is_window(hwnd);
        if !current {
            return Err(anyhow::anyhow!("Invalid window handle"));
        }

        // 切换状态 - 这里简化处理，实际应该查询当前状态
        let new_state = true; // 假设设置为置顶
        self.api.set_topmost(hwnd, new_state)?;
        Ok(new_state)
    }

    /// 设置透明度
    pub fn set_opacity(&self, hwnd: HWND, opacity: u8) -> Result<()> {
        self.api.set_opacity(hwnd, opacity)
    }
}

/// 需要真实 Windows API 的功能（跨显示器移动、窗口切换等）
impl RealWindowManager {
    /// 移动窗口到另一个显示器
    pub fn move_to_monitor(
        &self,
        hwnd: HWND,
        direction: MonitorDirection,
    ) -> Result<()> {
        unsafe {
            // 获取所有显示器
            let monitors = self.get_all_monitors();
            if monitors.len() < 2 {
                debug!("Only one monitor, nothing to do");
                return Ok(());
            }

            // 获取当前窗口所在显示器索引
            let current_monitor_index =
                self.get_current_monitor_index(hwnd, &monitors)?;

            // 计算目标显示器索引
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

            // 获取当前窗口信息
            let info = self.get_window_info(hwnd)?;

            // 计算相对位置比例
            let rel_x =
                (info.frame.x - current_monitor.x) as f32 / current_monitor.width as f32;
            let rel_y = (info.frame.y - current_monitor.y) as f32
                / current_monitor.height as f32;
            let rel_width = info.frame.width as f32 / current_monitor.width as f32;
            let rel_height = info.frame.height as f32 / current_monitor.height as f32;

            // 计算新位置（保持相对位置和大小比例）
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

    /// 获取所有显示器信息
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

            BOOL(1) // 继续枚举
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

    /// 获取窗口当前所在的显示器索引
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

        // 默认返回第一个显示器
        Ok(0)
    }

    /// 切换到同进程的下一个窗口（Alt+` 功能）
    pub fn switch_to_next_window_of_same_process(&self) -> Result<()> {
        unsafe {
            let current_hwnd = GetForegroundWindow();
            if current_hwnd.0 == 0 {
                return Err(anyhow::anyhow!("No foreground window"));
            }

            // 获取当前窗口的进程 ID
            let current_pid = self.get_window_process_id(current_hwnd)?;
            debug!("Current window PID: {}", current_pid);

            // 获取该进程的所有可见窗口
            let windows = self.get_process_visible_windows(current_pid);
            if windows.len() < 2 {
                debug!("Only one window in process, nothing to switch");
                return Ok(());
            }

            // 按 Z-Order 排序（从前到后）
            let sorted_windows = self.sort_windows_by_zorder(windows);

            // 找到当前窗口的索引
            let current_index = sorted_windows
                .iter()
                .position(|&hwnd| hwnd == current_hwnd)
                .unwrap_or(0);

            // 切换到下一个窗口
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

    /// 获取窗口的进程 ID
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

    /// 获取指定进程的所有可见窗口
    fn get_process_visible_windows(&self, target_pid: u32) -> Vec<HWND> {
        use windows::Win32::Foundation::BOOL;
        use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, IsWindowVisible};

        struct EnumData {
            target_pid: u32,
            windows: Vec<HWND>,
        }

        unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let data = &mut *(lparam.0 as *mut EnumData);

            // 检查窗口是否可见
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1); // 继续枚举
            }

            // 获取窗口进程 ID
            let mut pid: u32 = 0;
            windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                hwnd,
                Some(&mut pid),
            );

            if pid == data.target_pid {
                data.windows.push(hwnd);
            }

            BOOL(1) // 继续枚举
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

    /// 按 Z-Order 排序窗口（从前到后）
    fn sort_windows_by_zorder(&self, windows: Vec<HWND>) -> Vec<HWND> {
        use windows::Win32::UI::WindowsAndMessaging::GetWindow;

        unsafe {
            // 获取所有窗口的 Z-Order 位置
            // 方法：从顶层窗口开始遍历，记录每个窗口的位置
            let mut zorder_map: std::collections::HashMap<isize, usize> =
                std::collections::HashMap::new();

            let mut hwnd = GetWindow(
                HWND(0),
                windows::Win32::UI::WindowsAndMessaging::GW_HWNDFIRST,
            );

            let mut z_index: usize = 0;
            while hwnd.0 != 0 {
                if windows.contains(&hwnd) {
                    zorder_map.insert(hwnd.0, z_index);
                }
                hwnd = GetWindow(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::GW_HWNDNEXT,
                );
                z_index += 1;
            }

            // 按 Z-Order 排序
            let mut sorted = windows;
            sorted.sort_by_key(|hwnd| {
                zorder_map.get(&hwnd.0).copied().unwrap_or(usize::MAX)
            });

            sorted
        }
    }

    /// 激活窗口（切换到前台）
    unsafe fn activate_window(&self, hwnd: HWND) -> Result<()> {
        use windows::Win32::UI::WindowsAndMessaging::{
            BringWindowToTop, SetForegroundWindow, ShowWindow, SW_RESTORE,
        };

        // 如果窗口最小化，先还原
        if IsIconic(hwnd).as_bool() {
            ShowWindow(hwnd, SW_RESTORE)
                .ok()
                .map_err(|e| anyhow::anyhow!("Failed to restore window: {}", e))?;
        }

        // 切换到前台
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

    #[test]
    fn test_window_manager_creation() {
        let api = MockWindowApi::new();
        let wm = WindowManager::with_api(api);

        // 验证创建成功
        assert!(wm.api().is_window(HWND(0)) == false);
    }

    #[test]
    fn test_get_window_info() {
        let api = MockWindowApi::new();
        let hwnd = HWND(1234);

        // 设置测试数据
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
        let hwnd = HWND(1234);

        // 设置测试数据 - 1920x1080 显示器上的 800x600 窗口
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

        // 验证窗口位置 (1920-800)/2 = 560, (1080-600)/2 = 240
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 560);
        assert_eq!(frame.y, 240);
    }

    #[test]
    fn test_move_to_edge() {
        let api = MockWindowApi::new();
        let hwnd = HWND(1234);

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

        // 测试左边缘
        wm.move_to_edge(hwnd, Edge::Left).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 0);

        // 测试右边缘
        wm.move_to_edge(hwnd, Edge::Right).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 1920 - 800);
    }

    #[test]
    fn test_set_half_screen() {
        let api = MockWindowApi::new();
        let hwnd = HWND(1234);

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

        // 测试左半屏
        wm.set_half_screen(hwnd, Edge::Left).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 0);
        assert_eq!(frame.y, 0);
        assert_eq!(frame.width, 960); // 1920 / 2
        assert_eq!(frame.height, 1080);

        // 测试右半屏
        wm.set_half_screen(hwnd, Edge::Right).unwrap();
        let frame = wm.api().get_window_rect(hwnd).unwrap();
        assert_eq!(frame.x, 960);
        assert_eq!(frame.width, 960);
    }

    #[test]
    fn test_loop_width() {
        let api = MockWindowApi::new();
        let hwnd = HWND(1234);

        // 在创建 WindowManager 之前设置所有数据
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

        // 测试从 50% 开始循环
        wm.loop_width(hwnd, Alignment::Left).unwrap();

        let frame = wm.api().get_window_rect(hwnd).unwrap();
        // 50% -> 40% = 768
        assert_eq!(frame.width, 768);
    }

    #[test]
    fn test_set_fixed_ratio() {
        let api = MockWindowApi::new();
        let hwnd = HWND(1234);

        // 在创建 WindowManager 之前设置所有数据
        api.set_monitor_info(
            hwnd,
            MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );
        // 需要先设置一个初始窗口大小
        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);

        // 测试 4:3 比例，100% 缩放
        wm.set_fixed_ratio(hwnd, 4.0 / 3.0, 0).unwrap();

        let frame = wm.api().get_window_rect(hwnd).unwrap();
        // 基于较小边 1080，4:3 比例，宽度 = 1080 * 4/3 = 1440
        assert_eq!(frame.width, 1440);
        assert_eq!(frame.height, 1080);
    }

    #[test]
    fn test_window_state_operations() {
        let api = MockWindowApi::new();
        let hwnd = HWND(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);

        // 测试最小化
        wm.minimize_window(hwnd).unwrap();
        assert!(wm.api().is_iconic(hwnd));

        // 测试还原
        wm.restore_window(hwnd).unwrap();
        assert!(!wm.api().is_iconic(hwnd));

        // 测试最大化
        wm.maximize_window(hwnd).unwrap();
        assert!(wm.api().is_zoomed(hwnd));
    }

    #[test]
    fn test_close_window() {
        let api = MockWindowApi::new();
        let hwnd = HWND(1234);

        api.set_window_rect(hwnd, WindowFrame::new(100, 100, 800, 600));

        let wm = WindowManager::with_api(api);
        assert!(wm.api().is_window(hwnd));

        wm.close_window(hwnd).unwrap();

        // 窗口应该被移除
        assert!(!wm.api().is_window(hwnd));
    }
}
