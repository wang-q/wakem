use super::{Action, InputEvent, ModifierState};
use serde::{Deserialize, Serialize};

/// 映射规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRule {
    /// 规则名称（可选）
    pub name: Option<String>,
    /// 触发条件
    pub trigger: Trigger,
    /// 执行的动作
    pub action: Action,
    /// 上下文条件（可选）
    pub context: Option<ContextCondition>,
    /// 是否启用
    pub enabled: bool,
}

impl MappingRule {
    pub fn new(trigger: Trigger, action: Action) -> Self {
        Self {
            name: None,
            trigger,
            action,
            context: None,
            enabled: true,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_context(mut self, context: ContextCondition) -> Self {
        self.context = Some(context);
        self
    }

    /// 检查输入事件是否匹配此规则
    pub fn matches(&self, event: &InputEvent, context: &ContextInfo) -> bool {
        if !self.enabled {
            return false;
        }

        // 检查上下文条件
        if let Some(ref cond) = self.context {
            if !cond.matches(context) {
                return false;
            }
        }

        // 检查触发条件
        self.trigger.matches(event)
    }
}

/// 触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    /// 键盘按键
    Key {
        scan_code: Option<u16>,
        virtual_key: Option<u16>,
        modifiers: ModifierState,
    },
    /// 鼠标按钮
    MouseButton {
        button: super::MouseButton,
        modifiers: ModifierState,
    },
    /// 鼠标手势（简化版）
    MouseGesture {
        button: super::MouseButton,
        direction: GestureDirection,
    },
    /// 热字符串（文本扩展）
    HotString { trigger: String },
    /// 组合触发（多个按键按顺序）
    Chord(Vec<Trigger>),
    /// 定时触发
    Timer { interval_ms: u64 },
    /// 总是触发
    Always,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GestureDirection {
    Up,
    Down,
    Left,
    Right,
    Circle,
}

impl Trigger {
    /// 检查输入事件是否匹配此触发条件
    pub fn matches(&self, event: &InputEvent) -> bool {
        match (self, event) {
            (
                Trigger::Key {
                    scan_code,
                    virtual_key,
                    modifiers,
                },
                InputEvent::Key(e),
            ) => {
                // 检查扫描码
                if let Some(sc) = scan_code {
                    if *sc != e.scan_code {
                        return false;
                    }
                }
                // 检查虚拟键码
                if let Some(vk) = virtual_key {
                    if *vk != e.virtual_key {
                        return false;
                    }
                }
                // 检查修饰键
                // 注意：这里应该比较修饰键是否匹配，简化处理
                true
            }
            (
                Trigger::MouseButton { button, .. },
                InputEvent::Mouse(e),
            ) => {
                // 检查鼠标按钮按下
                e.is_button_down(*button)
            }
            _ => false,
        }
    }

    /// 创建简单的按键触发器
    pub fn key(scan_code: u16, virtual_key: u16) -> Self {
        Self::Key {
            scan_code: Some(scan_code),
            virtual_key: Some(virtual_key),
            modifiers: ModifierState::default(),
        }
    }

    /// 创建带修饰键的触发器
    pub fn key_with_modifiers(
        scan_code: u16,
        virtual_key: u16,
        modifiers: ModifierState,
    ) -> Self {
        Self::Key {
            scan_code: Some(scan_code),
            virtual_key: Some(virtual_key),
            modifiers,
        }
    }
}

/// 上下文条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextCondition {
    /// 窗口类名匹配（支持通配符）
    pub window_class: Option<String>,
    /// 进程名匹配（支持通配符）
    pub process_name: Option<String>,
    /// 窗口标题匹配（支持通配符）
    pub window_title: Option<String>,
}

impl ContextCondition {
    pub fn new() -> Self {
        Self {
            window_class: None,
            process_name: None,
            window_title: None,
        }
    }

    pub fn with_window_class(mut self, class: impl Into<String>) -> Self {
        self.window_class = Some(class.into());
        self
    }

    pub fn with_process_name(mut self, name: impl Into<String>) -> Self {
        self.process_name = Some(name.into());
        self
    }

    pub fn with_window_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = Some(title.into());
        self
    }

    /// 检查当前上下文是否匹配
    pub fn matches(&self, context: &ContextInfo) -> bool {
        if let Some(ref pattern) = self.window_class {
            if !wildcard_match(&context.window_class, pattern) {
                return false;
            }
        }
        if let Some(ref pattern) = self.process_name {
            if !wildcard_match(&context.process_name, pattern) {
                return false;
            }
        }
        if let Some(ref pattern) = self.window_title {
            if !wildcard_match(&context.window_title, pattern) {
                return false;
            }
        }
        true
    }
}

/// 上下文信息（当前活动窗口等）
#[derive(Debug, Clone, Default)]
pub struct ContextInfo {
    pub window_class: String,
    pub process_name: String,
    pub process_path: String,
    pub window_title: String,
    pub window_handle: isize, // HWND
}

/// 简单的通配符匹配（* 匹配任意字符，? 匹配单个字符）
fn wildcard_match(text: &str, pattern: &str) -> bool {
    // 简化实现，实际应该使用更复杂的匹配算法
    if pattern == "*" || pattern.is_empty() {
        return true;
    }
    if pattern.contains('*') || pattern.contains('?') {
        // TODO: 实现完整的通配符匹配
        text.to_lowercase().contains(&pattern.replace('*', "").to_lowercase())
    } else {
        text.to_lowercase() == pattern.to_lowercase()
    }
}
