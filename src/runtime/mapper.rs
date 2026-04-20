use crate::types::{
    Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState, MappingRule,
    Trigger,
};
use std::collections::HashMap;
use tracing::{debug, trace};

#[cfg(target_os = "windows")]
use crate::platform::windows::window_manager::RealWindowManager;

/// Context感知映射规则
///
/// 根据当前窗口的属性（进程名、窗口类、标题等）来选择不同的映射表。
/// 这使得同一个按键在不同应用中可以有不同的行为。
///
/// # 示例
///
/// ```ignore
/// // 在 VSCode 中将 CapsLock 映射为 Ctrl，但在其他应用中映射为 Esc
/// let rule = ContextMappingRule {
///     context: ContextCondition::new()
///         .with_process_name("Code.exe"),
///     mappings: {
///         let mut map = HashMap::new();
///         map.insert(0x3A, Action::Key(KeyAction::Press { scan_code: None, virtual_key: 0x11 }));
///         map
///     },
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ContextMappingRule {
    /// Context condition（决定何时应用此规则）
    pub context: ContextCondition,
    /// 映射表：扫描码 -> 动作（当条件满足时使用）
    pub mappings: HashMap<u16, Action>,
}

/// 键位映射引擎
///
/// wakem 的核心组件，负责：
/// - 管理基础键位映射规则
/// - 支持上下文感知映射（根据当前窗口动态切换）
/// - 处理输入事件并返回相应的动作
/// - 执行各种动作类型（键盘、鼠标、窗口、启动程序等）
///
/// # 架构说明
///
/// 映射引擎采用分层处理策略：
/// 1. **基础映射** - 始终生效的全局规则
/// 2. **上下文映射** - 仅在特定条件下生效的规则（优先级更高）
/// 3. **层管理器** - 由 LayerManager 提供，支持临时覆盖
///
/// # 使用示例
///
/// ```ignore
/// use wakem::runtime::KeyMapper;
///
/// // 创建映射引擎
/// let mut mapper = KeyMapper::new();
///
/// // 加载配置中的规则
/// let rules = config.get_all_rules();
/// mapper.load_rules(rules);
///
/// // 处理输入事件
/// if let Some(action) = mapper.process_event(&input_event) {
///     mapper.execute_action(&action)?;
/// }
/// ```
///
/// # 性能特性
///
/// - 使用 HashMap 实现快速查找（O(1) 平均复杂度）
/// - 上下文条件缓存以减少重复计算
/// - 支持热重载配置而不重启服务
pub struct KeyMapper {
    /// 基础映射表：扫描码 -> 动作
    ///
    /// 存储全局性的键位映射规则，这些规则始终生效。
    /// 键是扫描码（硬件相关），值是要执行的动作。
    mappings: HashMap<u16, Action>,

    /// 完整的映射规则列表
    ///
    /// 保留原始规则用于调试和序列化。
    rules: Vec<MappingRule>,

    /// Context感知映射规则列表
    ///
    /// 这些规则仅在满足特定上下文条件时生效，
    /// 例如：只在 VSCode 中将 CapsLock 映射为 Ctrl。
    context_rules: Vec<ContextMappingRule>,

    /// 是否启用映射引擎
    ///
    /// 当为 false 时，所有输入事件直接透传，不进行任何映射。
    enabled: bool,

    /// Window manager（用于执行窗口管理动作）
    #[cfg(target_os = "windows")]
    window_manager: Option<RealWindowManager>,

    /// Tray图标（用于显示通知）
    #[cfg(target_os = "windows")]
    tray_icon: Option<crate::platform::windows::TrayIcon>,

    /// Window preset管理器（用于保存/加载窗口预设）
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
    pub fn with_window_manager(window_manager: RealWindowManager) -> Self {
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

    /// 设置窗口预设管理器
    #[cfg(target_os = "windows")]
    pub fn set_window_preset_manager(
        &mut self,
        manager: crate::platform::windows::WindowPresetManager,
    ) {
        self.window_preset_manager = Some(manager);
    }

    /// 从配置加载映射规则
    pub fn load_rules(&mut self, rules: Vec<MappingRule>) {
        self.rules = rules;
        self.rebuild_mappings();
        debug!("Loaded {} mapping rules", self.rules.len());
    }

    /// Process input event（带上下文感知）
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

    /// Execute action（包括窗口管理动作）
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
        wm: &RealWindowManager,
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
                        GetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST,
                        HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
                    };

                    // 正确判断：检查 WS_EX_TOPMOST 扩展窗口样式
                    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                    let is_topmost = (ex_style & WS_EX_TOPMOST.0 as i32) != 0;

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
    fn test_key_mapper_basic() {
        let mapper = KeyMapper::new();

        // Test创建成功
        assert!(mapper.enabled);
    }

    #[test]
    fn test_key_mapper_disabled() {
        let mut mapper = KeyMapper::new();

        // 禁用映射器
        mapper.enabled = false;

        // Test事件处理返回 None
        let event = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
        let result = mapper.process_event_with_context(&InputEvent::Key(event), None);

        assert!(result.is_none());
    }

    #[test]
    fn test_key_mapper_load_rules() {
        let mut mapper = KeyMapper::new();

        // 创建简单的映射规则
        let rules = vec![MappingRule::new(
            Trigger::key(0x3A, 0x14),                  // CapsLock
            Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
        )];

        mapper.load_rules(rules);

        // 验证规则已加载
        assert_eq!(mapper.rules.len(), 1);
    }
}
