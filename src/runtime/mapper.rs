use crate::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, MappingRule,
    Trigger,
};
use std::collections::HashMap;
use tracing::{debug, trace};

/// 上下文感知映射规则
#[derive(Debug, Clone)]
pub struct ContextMappingRule {
    /// 上下文条件
    pub context: ContextCondition,
    /// 映射表：扫描码 -> 动作
    pub mappings: HashMap<u16, Action>,
}

/// 键位映射引擎
pub struct KeyMapper {
    /// 基础映射表：扫描码 -> 动作
    mappings: HashMap<u16, Action>,
    /// 完整的映射规则列表
    rules: Vec<MappingRule>,
    /// 上下文感知映射规则列表
    context_rules: Vec<ContextMappingRule>,
    /// 是否启用
    enabled: bool,
    /// 窗口管理器（用于执行窗口管理动作）
    #[cfg(target_os = "windows")]
    window_manager: Option<crate::platform::windows::WindowManager>,
    /// 托盘图标（用于显示通知）
    #[cfg(target_os = "windows")]
    tray_icon: Option<crate::platform::windows::TrayIcon>,
    /// 窗口预设管理器（用于保存/加载窗口预设）
    #[cfg(target_os = "windows")]
    window_preset_manager: Option<crate::platform::windows::WindowPresetManager>,
}

impl KeyMapper {
    /// 创建新的映射引擎
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            #[cfg(target_os = "windows")]
            window_manager: None,
            #[cfg(target_os = "windows")]
            tray_icon: None,
            #[cfg(target_os = "windows")]
            window_preset_manager: None,
        }
    }

    /// 创建带窗口管理器的映射引擎
    #[cfg(target_os = "windows")]
    pub fn with_window_manager(
        window_manager: crate::platform::windows::WindowManager,
    ) -> Self {
        Self {
            mappings: HashMap::new(),
            rules: Vec::new(),
            context_rules: Vec::new(),
            enabled: true,
            window_manager: Some(window_manager),
            tray_icon: None,
            window_preset_manager: Some(
                crate::platform::windows::WindowPresetManager::new(),
            ),
        }
    }

    /// 设置窗口管理器
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub fn set_window_manager(
        &mut self,
        window_manager: crate::platform::windows::WindowManager,
    ) {
        self.window_manager = Some(window_manager);
    }

    /// 设置托盘图标
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub fn set_tray_icon(&mut self, tray_icon: crate::platform::windows::TrayIcon) {
        self.tray_icon = Some(tray_icon);
    }

    /// 设置窗口预设管理器
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub fn set_window_preset_manager(
        &mut self,
        manager: crate::platform::windows::WindowPresetManager,
    ) {
        self.window_preset_manager = Some(manager);
    }

    /// 获取窗口预设管理器的可变引用
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub fn window_preset_manager_mut(
        &mut self,
    ) -> Option<&mut crate::platform::windows::WindowPresetManager> {
        self.window_preset_manager.as_mut()
    }

    /// 从配置加载映射规则
    pub fn load_rules(&mut self, rules: Vec<MappingRule>) {
        self.rules = rules;
        self.rebuild_mappings();
        debug!("Loaded {} mapping rules", self.rules.len());
    }

    /// 添加单条映射规则
    #[allow(dead_code)]
    pub fn add_rule(&mut self, rule: MappingRule) {
        self.rules.push(rule);
        self.rebuild_mappings();
    }

    /// 清除所有映射
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.rules.clear();
        self.mappings.clear();
    }

    /// 启用/禁用映射
    #[allow(dead_code)]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        debug!("KeyMapper enabled: {}", enabled);
    }

    /// 检查是否启用
    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 处理输入事件，返回要执行的动作
    #[allow(dead_code)]
    pub fn process_event(&self, event: &InputEvent) -> Option<Action> {
        self.process_event_with_context(event, None)
    }

    /// 处理输入事件（带上下文感知）
    pub fn process_event_with_context(
        &self,
        event: &InputEvent,
        context: Option<&crate::platform::windows::WindowContext>,
    ) -> Option<Action> {
        if !self.enabled {
            return None;
        }

        match event {
            InputEvent::Key(key_event) => {
                self.process_key_event_with_context(key_event, context)
            }
            InputEvent::Mouse(_) => {
                // 鼠标事件处理（TODO）
                None
            }
        }
    }

    /// 处理键盘事件
    #[allow(dead_code)]
    fn process_key_event(&self, event: &KeyEvent) -> Option<Action> {
        self.process_key_event_with_context(event, None)
    }

    /// 处理键盘事件（带上下文感知）
    fn process_key_event_with_context(
        &self,
        event: &KeyEvent,
        context: Option<&crate::platform::windows::WindowContext>,
    ) -> Option<Action> {
        trace!(
            "Processing key event: scan_code={:04X}, vk={:04X}, state={:?}",
            event.scan_code,
            event.virtual_key,
            event.state
        );

        // 1. 首先检查上下文特定规则（优先级高）
        if let Some(ctx) = context {
            for rule in &self.context_rules {
                // 检查上下文是否匹配
                if rule.context.matches(
                    &ctx.process_name,
                    &ctx.window_class,
                    &ctx.window_title,
                    Some(&ctx.executable_path),
                ) {
                    // 在匹配的上下文中查找映射
                    if let Some(action) = rule.mappings.get(&event.scan_code) {
                        let adjusted_action =
                            self.adjust_action_for_key_state(action, event);
                        if adjusted_action.is_some() {
                            trace!(
                                "Context mapping found: {:04X} -> {:?} (context: {:?})",
                                event.scan_code,
                                action,
                                rule.context
                            );
                        }
                        return adjusted_action;
                    }
                }
            }
        }

        // 2. 检查基础映射（全局规则）
        if let Some(action) = self.mappings.get(&event.scan_code) {
            let adjusted_action = self.adjust_action_for_key_state(action, event);
            if adjusted_action.is_some() {
                trace!(
                    "Base mapping found: {:04X} -> {:?}",
                    event.scan_code,
                    action
                );
            }
            return adjusted_action;
        }

        None
    }

    /// 根据按键状态调整动作
    fn adjust_action_for_key_state(
        &self,
        action: &Action,
        event: &KeyEvent,
    ) -> Option<Action> {
        match (action, &event.state) {
            (
                Action::Key(KeyAction::Click {
                    scan_code,
                    virtual_key,
                }),
                _,
            ) => {
                // 如果是点击动作，根据实际按键状态调整
                match event.state {
                    KeyState::Pressed => Some(Action::Key(KeyAction::Press {
                        scan_code: *scan_code,
                        virtual_key: *virtual_key,
                    })),
                    KeyState::Released => Some(Action::Key(KeyAction::Release {
                        scan_code: *scan_code,
                        virtual_key: *virtual_key,
                    })),
                }
            }
            _ => Some(action.clone()),
        }
    }

    /// 执行动作（包括窗口管理动作）
    #[cfg(target_os = "windows")]
    pub fn execute_action(&mut self, action: &Action) -> anyhow::Result<()> {
        match action {
            Action::Window(window_action) => {
                if let Some(ref wm) = self.window_manager {
                    Self::execute_window_action_internal(
                        wm,
                        self.tray_icon.as_mut(),
                        self.window_preset_manager.as_mut(),
                        window_action,
                    )?;
                } else {
                    debug!("WindowManager not available, skipping window action");
                }
            }
            Action::Key(_)
            | Action::Mouse(_)
            | Action::Launch(_)
            | Action::Sequence(_)
            | Action::System(_)
            | Action::Delay { .. }
            | Action::None => {
                // 这些动作由其他组件处理
            }
        }

        Ok(())
    }

    /// 执行窗口管理动作（内部静态方法，避免借用冲突）
    #[cfg(target_os = "windows")]
    fn execute_window_action_internal(
        wm: &crate::platform::windows::WindowManager,
        tray_icon: Option<&mut crate::platform::windows::TrayIcon>,
        preset_manager: Option<&mut crate::platform::windows::WindowPresetManager>,
        action: &crate::types::WindowAction,
    ) -> anyhow::Result<()> {
        use crate::types::{MonitorDirection, WindowAction};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return Err(anyhow::anyhow!("No foreground window"));
            }

            match action {
                WindowAction::Center => wm.move_to_center(hwnd)?,
                WindowAction::MoveToEdge(edge) => {
                    wm.move_to_edge(hwnd, *edge)?;
                }
                WindowAction::HalfScreen(edge) => {
                    wm.set_half_screen(hwnd, *edge)?;
                }
                WindowAction::LoopWidth(align) => {
                    wm.loop_width(hwnd, *align)?;
                }
                WindowAction::LoopHeight(align) => {
                    wm.loop_height(hwnd, *align)?;
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
                        MonitorDirection::Next => {
                            crate::platform::windows::MonitorDirection::Next
                        }
                        MonitorDirection::Prev => {
                            crate::platform::windows::MonitorDirection::Prev
                        }
                        MonitorDirection::Index(idx) => {
                            crate::platform::windows::MonitorDirection::Index(*idx)
                        }
                    };
                    wm.move_to_monitor(hwnd, direction)?;
                }
                WindowAction::Move { x, y } => {
                    use crate::platform::windows::WindowFrame;
                    let info = wm.get_window_info(hwnd)?;
                    let new_frame =
                        WindowFrame::new(*x, *y, info.frame.width, info.frame.height);
                    wm.set_window_frame(hwnd, &new_frame)?;
                }
                WindowAction::Resize { width, height } => {
                    use crate::platform::windows::WindowFrame;
                    let info = wm.get_window_info(hwnd)?;
                    let new_frame =
                        WindowFrame::new(info.frame.x, info.frame.y, *width, *height);
                    wm.set_window_frame(hwnd, &new_frame)?;
                }
                WindowAction::Minimize => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        ShowWindow, SW_MINIMIZE,
                    };
                    let _ = ShowWindow(hwnd, SW_MINIMIZE);
                }
                WindowAction::Maximize => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        ShowWindow, SW_MAXIMIZE,
                    };
                    let _ = ShowWindow(hwnd, SW_MAXIMIZE);
                }
                WindowAction::Restore => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        ShowWindow, SW_RESTORE,
                    };
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }
                WindowAction::Close => {
                    use windows::Win32::Foundation::{LPARAM, WPARAM};
                    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
                    use windows::Win32::UI::WindowsAndMessaging::WM_CLOSE;
                    PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)).ok();
                }
                WindowAction::ToggleTopmost => {
                    use windows::Win32::UI::WindowsAndMessaging::{
                        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE,
                        SWP_NOSIZE,
                    };

                    let info = wm.get_window_info(hwnd)?;
                    // 简单判断：如果窗口在 (0,0) 附近，假设它是置顶窗口
                    // 实际应该使用 GetWindowLong 检查 WS_EX_TOPMOST 样式
                    let is_topmost = info.frame.x == 0 && info.frame.y == 0;

                    let hwnd_insert_after = if is_topmost {
                        HWND_NOTOPMOST
                    } else {
                        HWND_TOPMOST
                    };

                    SetWindowPos(
                        hwnd,
                        hwnd_insert_after,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE,
                    )
                    .ok();
                }
                WindowAction::SetOpacity { opacity } => {
                    use windows::Win32::Foundation::COLORREF;
                    use windows::Win32::UI::WindowsAndMessaging::{
                        GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW,
                        GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
                    };

                    // 获取当前扩展样式
                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);

                    // 添加 WS_EX_LAYERED 样式
                    if ex_style & WS_EX_LAYERED.0 as i32 == 0 {
                        SetWindowLongW(
                            hwnd,
                            GWL_EXSTYLE,
                            ex_style | WS_EX_LAYERED.0 as i32,
                        );
                    }

                    // 设置透明度
                    SetLayeredWindowAttributes(hwnd, COLORREF(0), *opacity, LWA_ALPHA)
                        .ok();
                }
                WindowAction::ShowDebugInfo => {
                    // 显示调试信息对话框
                    match wm.get_debug_info() {
                        Ok(info) => {
                            use windows::core::HSTRING;
                            use windows::Win32::UI::WindowsAndMessaging::{
                                MessageBoxW, MB_ICONINFORMATION, MB_OK,
                            };

                            let title = HSTRING::from("wakem - Debug Info");
                            let message = HSTRING::from(&info);

                            MessageBoxW(
                                None,
                                &message,
                                &title,
                                MB_OK | MB_ICONINFORMATION,
                            );
                        }
                        Err(e) => {
                            debug!("Failed to get debug info: {}", e);
                        }
                    }
                }
                WindowAction::ShowNotification { title, message } => {
                    // 使用托盘图标显示通知
                    if let Some(tray) = tray_icon {
                        if let Err(e) = tray.show_notification(title, message) {
                            debug!("Failed to show notification: {}", e);
                        }
                    } else {
                        debug!("Tray icon not available, cannot show notification");
                    }
                }
                WindowAction::SavePreset { name } => {
                    if let Some(pm) = preset_manager {
                        match pm.get_foreground_window_info() {
                            Ok((hwnd, _title, process_name, executable_path)) => {
                                if let Err(e) = pm.save_preset(
                                    name,
                                    hwnd,
                                    process_name,
                                    executable_path,
                                    None, // 不使用标题模式，使用进程名匹配
                                ) {
                                    debug!("Failed to save preset '{}': {}", name, e);
                                } else {
                                    debug!("Saved preset '{}' for current window", name);
                                    // 显示通知
                                    if let Some(tray) = tray_icon {
                                        let _ = tray.show_notification(
                                            "wakem",
                                            &format!("已保存预设 '{}'", name),
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Failed to get foreground window info: {}", e);
                            }
                        }
                    } else {
                        debug!("WindowPresetManager not available, cannot save preset");
                    }
                }
                WindowAction::LoadPreset { name } => {
                    if let Some(pm) = preset_manager {
                        if let Err(e) = pm.load_preset(name, hwnd) {
                            debug!("Failed to load preset '{}': {}", name, e);
                        } else {
                            debug!("Loaded preset '{}' for current window", name);
                        }
                    } else {
                        debug!("WindowPresetManager not available, cannot load preset");
                    }
                }
                WindowAction::ApplyPreset => {
                    if let Some(pm) = preset_manager {
                        match pm.apply_preset_for_window(hwnd) {
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
            if let Trigger::Key {
                scan_code,
                virtual_key,
                ..
            } = &rule.trigger
            {
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

    /// 加载上下文感知映射规则
    pub fn load_context_rules(
        &mut self,
        context_mappings: &[crate::config::ContextMapping],
    ) {
        self.context_rules.clear();

        for mapping in context_mappings {
            let mut rule_mappings = HashMap::new();

            // 解析每个映射字符串
            for (from, to) in &mapping.mappings {
                if let Ok((scan_code, action)) = Self::parse_context_mapping(from, to) {
                    rule_mappings.insert(scan_code, action);
                }
            }

            if !rule_mappings.is_empty() {
                self.context_rules.push(ContextMappingRule {
                    context: mapping.context.clone(),
                    mappings: rule_mappings,
                });
            }
        }

        debug!("Loaded {} context mapping rules", self.context_rules.len());
    }

    /// 解析上下文映射字符串
    fn parse_context_mapping(from: &str, to: &str) -> anyhow::Result<(u16, Action)> {
        use crate::config::parse_key;
        use crate::types::KeyAction;

        // 解析源键
        let from_key = parse_key(from)?;

        // 解析目标动作
        // 先尝试解析为键位
        if let Ok(to_key) = parse_key(to) {
            return Ok((
                from_key.0,
                Action::key(KeyAction::click(to_key.0, to_key.1)),
            ));
        }

        // 尝试解析为窗口动作
        if let Ok(window_action) = crate::config::parse_window_action(to) {
            return Ok((from_key.0, Action::window(window_action)));
        }

        Err(anyhow::anyhow!(
            "Failed to parse mapping: {} -> {}",
            from,
            to
        ))
    }

    /// 添加简单的键位重映射
    #[allow(dead_code)]
    pub fn add_simple_remap(
        &mut self,
        from_scan_code: u16,
        to_scan_code: u16,
        to_vk: u16,
    ) {
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
            Action::Key(KeyAction::Press {
                scan_code,
                virtual_key,
            }) => {
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
