use std::collections::HashMap;
use tracing::{debug, trace};
use wakem_common::types::{Action, InputEvent, KeyAction, KeyEvent, MappingRule, Trigger};

/// 键位映射引擎
pub struct KeyMapper {
    /// 基础映射表：扫描码 -> 动作
    mappings: HashMap<u16, Action>,
    /// 完整的映射规则列表
    rules: Vec<MappingRule>,
    /// 是否启用
    enabled: bool,
    /// 窗口管理器（用于执行窗口管理动作）
    #[cfg(target_os = "windows")]
    window_manager: Option<crate::platform::windows::WindowManager>,
}

impl KeyMapper {
    /// 创建新的映射引擎
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            rules: Vec::new(),
            enabled: true,
            #[cfg(target_os = "windows")]
            window_manager: None,
        }
    }

    /// 创建带窗口管理器的映射引擎
    #[cfg(target_os = "windows")]
    pub fn with_window_manager(window_manager: crate::platform::windows::WindowManager) -> Self {
        Self {
            mappings: HashMap::new(),
            rules: Vec::new(),
            enabled: true,
            window_manager: Some(window_manager),
        }
    }

    /// 设置窗口管理器
    #[cfg(target_os = "windows")]
    pub fn set_window_manager(&mut self, window_manager: crate::platform::windows::WindowManager) {
        self.window_manager = Some(window_manager);
    }

    /// 从配置加载映射规则
    pub fn load_rules(&mut self, rules: Vec<MappingRule>) {
        self.rules = rules;
        self.rebuild_mappings();
        debug!("Loaded {} mapping rules", self.rules.len());
    }

    /// 添加单条映射规则
    pub fn add_rule(&mut self, rule: MappingRule) {
        self.rules.push(rule);
        self.rebuild_mappings();
    }

    /// 清除所有映射
    pub fn clear(&mut self) {
        self.rules.clear();
        self.mappings.clear();
    }

    /// 启用/禁用映射
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        debug!("KeyMapper enabled: {}", enabled);
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 处理输入事件，返回要执行的动作
    pub fn process_event(&self, event: &InputEvent) -> Option<Action> {
        if !self.enabled {
            return None;
        }

        match event {
            InputEvent::Key(key_event) => {
                self.process_key_event(key_event)
            }
            InputEvent::Mouse(_) => {
                // 鼠标事件处理（TODO）
                None
            }
        }
    }

    /// 处理键盘事件
    fn process_key_event(&self, event: &KeyEvent) -> Option<Action> {
        trace!(
            "Processing key event: scan_code={:04X}, vk={:04X}, state={:?}",
            event.scan_code, event.virtual_key, event.state
        );

        // 查找映射
        if let Some(action) = self.mappings.get(&event.scan_code) {
            // 根据按键状态调整动作
            let adjusted_action = match (action, &event.state) {
                (Action::Key(KeyAction::Click { scan_code, virtual_key }), _) => {
                    // 如果是点击动作，根据实际按键状态调整
                    match event.state {
                        wakem_common::types::KeyState::Pressed => {
                            Some(Action::Key(KeyAction::Press {
                                scan_code: *scan_code,
                                virtual_key: *virtual_key,
                            }))
                        }
                        wakem_common::types::KeyState::Released => {
                            Some(Action::Key(KeyAction::Release {
                                scan_code: *scan_code,
                                virtual_key: *virtual_key,
                            }))
                        }
                    }
                }
                _ => Some(action.clone()),
            };

            if adjusted_action.is_some() {
                trace!("Mapping found: {:04X} -> {:?}", event.scan_code, action);
            }

            return adjusted_action;
        }

        None
    }

    /// 执行动作（包括窗口管理动作）
    #[cfg(target_os = "windows")]
    pub fn execute_action(&self, action: &Action) -> anyhow::Result<()> {
        use wakem_common::types::WindowAction;

        match action {
            Action::Window(window_action) => {
                if let Some(ref wm) = self.window_manager {
                    self.execute_window_action(wm, window_action)?;
                } else {
                    debug!("WindowManager not available, skipping window action");
                }
            }
            Action::Key(_) | Action::Mouse(_) | Action::Launch(_) | Action::Sequence(_) | Action::None => {
                // 这些动作由其他组件处理
            }
        }

        Ok(())
    }

    /// 执行窗口管理动作
    #[cfg(target_os = "windows")]
    fn execute_window_action(
        &self,
        wm: &crate::platform::windows::WindowManager,
        action: &wakem_common::types::WindowAction,
    ) -> anyhow::Result<()> {
        use wakem_common::types::{Alignment, Edge, MonitorDirection, WindowAction};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return Err(anyhow::anyhow!("No foreground window"));
            }

            match action {
                WindowAction::Center => wm.move_to_center(hwnd)?,
                WindowAction::MoveToEdge(edge) => {
                    let edge = match edge {
                        Edge::Left => crate::platform::windows::Edge::Left,
                        Edge::Right => crate::platform::windows::Edge::Right,
                        Edge::Top => crate::platform::windows::Edge::Top,
                        Edge::Bottom => crate::platform::windows::Edge::Bottom,
                    };
                    wm.move_to_edge(hwnd, edge)?;
                }
                WindowAction::HalfScreen(edge) => {
                    let edge = match edge {
                        Edge::Left => crate::platform::windows::Edge::Left,
                        Edge::Right => crate::platform::windows::Edge::Right,
                        Edge::Top => crate::platform::windows::Edge::Top,
                        Edge::Bottom => crate::platform::windows::Edge::Bottom,
                    };
                    wm.set_half_screen(hwnd, edge)?;
                }
                WindowAction::LoopWidth(align) => {
                    let align = match align {
                        Alignment::Left => crate::platform::windows::Alignment::Left,
                        Alignment::Right => crate::platform::windows::Alignment::Right,
                        _ => crate::platform::windows::Alignment::Center,
                    };
                    wm.loop_width(hwnd, align)?;
                }
                WindowAction::LoopHeight(align) => {
                    let align = match align {
                        Alignment::Top => crate::platform::windows::Alignment::Top,
                        Alignment::Bottom => crate::platform::windows::Alignment::Bottom,
                        _ => crate::platform::windows::Alignment::Center,
                    };
                    wm.loop_height(hwnd, align)?;
                }
                WindowAction::FixedRatio { ratio, scale_index } => {
                    wm.set_fixed_ratio(hwnd, *ratio, *scale_index)?;
                }
                WindowAction::NativeRatio { scale_index } => {
                    wm.set_native_ratio(hwnd, *scale_index)?;
                }
                WindowAction::SwitchToNextWindow => {
                    wm.switch_to_next_window_of_same_process()?;
                }
                WindowAction::MoveToMonitor(direction) => {
                    let direction = match direction {
                        MonitorDirection::Next => crate::platform::windows::MonitorDirection::Next,
                        MonitorDirection::Prev => crate::platform::windows::MonitorDirection::Prev,
                        MonitorDirection::Index(idx) => crate::platform::windows::MonitorDirection::Index(*idx),
                    };
                    wm.move_to_monitor(hwnd, direction)?;
                }
                WindowAction::Move { x, y } => {
                    use crate::platform::windows::WindowFrame;
                    let info = wm.get_window_info(hwnd)?;
                    let new_frame = WindowFrame::new(*x, *y, info.frame.width, info.frame.height);
                    wm.set_window_frame(hwnd, &new_frame)?;
                }
                WindowAction::Resize { width, height } => {
                    use crate::platform::windows::WindowFrame;
                    let info = wm.get_window_info(hwnd)?;
                    let new_frame = WindowFrame::new(info.frame.x, info.frame.y, *width, *height);
                    wm.set_window_frame(hwnd, &new_frame)?;
                }
                WindowAction::Minimize => {
                    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};
                    ShowWindow(hwnd, SW_MINIMIZE).ok();
                }
                WindowAction::Maximize => {
                    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MAXIMIZE};
                    ShowWindow(hwnd, SW_MAXIMIZE).ok();
                }
                WindowAction::Restore => {
                    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_RESTORE};
                    ShowWindow(hwnd, SW_RESTORE).ok();
                }
                WindowAction::Close => {
                    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
                    use windows::Win32::Foundation::{WPARAM, LPARAM};
                    use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
                    PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
                }
                WindowAction::ToggleTopmost => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        SetWindowPos, HWND_TOPMOST, HWND_NOTOPMOST, SWP_NOMOVE, SWP_NOSIZE,
                    };
                    use windows::Win32::Foundation::BOOL;
                    
                    let info = wm.get_window_info(hwnd)?;
                    // 简单判断：如果窗口在 (0,0) 附近，假设它是置顶窗口
                    // 实际应该使用 GetWindowLong 检查 WS_EX_TOPMOST 样式
                    let is_topmost = info.frame.x == 0 && info.frame.y == 0;
                    
                    let hwnd_insert_after = if is_topmost {
                        HWND_NOTOPMOST
                    } else {
                        HWND_TOPMOST
                    };
                    
                    SetWindowPos(hwnd, hwnd_insert_after, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE)
                        .ok();
                }
                WindowAction::SetOpacity { opacity } => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        SetLayeredWindowAttributes, GetWindowLongW, SetWindowLongW,
                        GWL_EXSTYLE, WS_EX_LAYERED, LWA_ALPHA,
                    };
                    use windows::Win32::Foundation::COLORREF;
                    
                    // 获取当前扩展样式
                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                    
                    // 添加 WS_EX_LAYERED 样式
                    if ex_style & WS_EX_LAYERED.0 as i32 == 0 {
                        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as i32);
                    }
                    
                    // 设置透明度
                    SetLayeredWindowAttributes(hwnd, COLORREF(0), *opacity, LWA_ALPHA).ok();
                }
                WindowAction::None => {}
            }
        }

        Ok(())
    }

    /// 重建映射表
    fn rebuild_mappings(&mut self) {
        self.mappings.clear();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            // 提取简单按键映射
            if let Trigger::Key { scan_code, virtual_key, .. } = &rule.trigger {
                if let Some(sc) = scan_code {
                    self.mappings.insert(*sc, rule.action.clone());
                } else if let Some(vk) = virtual_key {
                    // 如果没有扫描码，使用虚拟键码作为备用
                    self.mappings.insert(*vk, rule.action.clone());
                }
            }
        }

        debug!("Rebuilt mappings: {} entries", self.mappings.len());
    }

    /// 添加简单的键位重映射
    pub fn add_simple_remap(&mut self, from_scan_code: u16, to_scan_code: u16, to_vk: u16) {
        let trigger = Trigger::key(from_scan_code, 0);
        let action = Action::key(KeyAction::click(to_scan_code, to_vk));
        
        self.add_rule(MappingRule::new(trigger, action));
    }
}

impl Default for KeyMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wakem_common::types::{KeyState, KeyEvent};

    #[test]
    fn test_key_mapper() {
        let mut mapper = KeyMapper::new();
        
        // 添加 CapsLock -> Backspace 映射
        mapper.add_simple_remap(0x3A, 0x0E, 0x08);
        
        // 测试按下 CapsLock
        let event = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let result = mapper.process_event(&InputEvent::Key(event));
        
        assert!(result.is_some());
        match result.unwrap() {
            Action::Key(KeyAction::Press { scan_code, virtual_key }) => {
                assert_eq!(scan_code, 0x0E);
                assert_eq!(virtual_key, 0x08);
            }
            _ => panic!("Expected Press action"),
        }
    }

    #[test]
    fn test_disabled_mapper() {
        let mut mapper = KeyMapper::new();
        mapper.add_simple_remap(0x3A, 0x0E, 0x08);
        mapper.set_enabled(false);
        
        let event = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let result = mapper.process_event(&InputEvent::Key(event));
        
        assert!(result.is_none());
    }
}
