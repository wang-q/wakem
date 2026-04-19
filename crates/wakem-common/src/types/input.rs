use super::{DeviceType, KeyState, ModifierState, Timestamp, now};
use serde::{Deserialize, Serialize};

/// 键盘事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    /// 扫描码（硬件相关）
    pub scan_code: u16,
    /// 虚拟键码（Windows VK_*) 
    pub virtual_key: u16,
    /// 按键状态
    pub state: KeyState,
    /// 修饰键状态
    pub modifiers: ModifierState,
    /// 设备类型
    pub device_type: DeviceType,
    /// 时间戳
    pub timestamp: Timestamp,
    /// 是否来自物理设备（而非模拟输入）
    pub is_injected: bool,
}

impl KeyEvent {
    pub fn new(scan_code: u16, virtual_key: u16, state: KeyState) -> Self {
        Self {
            scan_code,
            virtual_key,
            state,
            modifiers: ModifierState::default(),
            device_type: DeviceType::Keyboard,
            timestamp: now(),
            is_injected: false,
        }
    }

    pub fn with_modifiers(mut self, modifiers: ModifierState) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn injected(mut self) -> Self {
        self.is_injected = true;
        self
    }

    /// 检查是否是修饰键
    pub fn is_modifier(&self) -> bool {
        matches!(
            self.virtual_key,
            0x10 | 0xA0 | 0xA1 | // Shift
            0x11 | 0xA2 | 0xA3 | // Ctrl
            0x12 | 0xA4 | 0xA5 | // Alt
            0x5B | 0x5C          // Win
        )
    }

    /// 获取修饰键的标识（如果是修饰键）
    pub fn modifier_identifier(&self) -> Option<&'static str> {
        match self.virtual_key {
            0x10 | 0xA0 | 0xA1 => Some("Shift"),
            0x11 | 0xA2 | 0xA3 => Some("Control"),
            0x12 | 0xA4 | 0xA5 => Some("Alt"),
            0x5B | 0x5C => Some("Meta"),
            _ => None,
        }
    }
}

/// 鼠标按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1, // 侧键1
    X2, // 侧键2
}

/// 鼠标事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseEvent {
    /// 事件类型
    pub event_type: MouseEventType,
    /// X 坐标（屏幕坐标）
    pub x: i32,
    /// Y 坐标（屏幕坐标）
    pub y: i32,
    /// 修饰键状态
    pub modifiers: ModifierState,
    /// 时间戳
    pub timestamp: Timestamp,
    /// 是否来自物理设备
    pub is_injected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseEventType {
    /// 鼠标移动
    Move,
    /// 按钮按下
    ButtonDown(MouseButton),
    /// 按钮释放
    ButtonUp(MouseButton),
    /// 滚轮滚动（正值向上，负值向下）
    Wheel(i32),
    /// 水平滚轮（正值向右，负值向左）
    HWheel(i32),
}

impl MouseEvent {
    pub fn new(event_type: MouseEventType, x: i32, y: i32) -> Self {
        Self {
            event_type,
            x,
            y,
            modifiers: ModifierState::default(),
            timestamp: now(),
            is_injected: false,
        }
    }

    pub fn with_modifiers(mut self, modifiers: ModifierState) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn injected(mut self) -> Self {
        self.is_injected = true;
        self
    }

    /// 检查是否是按钮按下事件
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        matches!(&self.event_type, MouseEventType::ButtonDown(b) if *b == button)
    }

    /// 检查是否是按钮释放事件
    pub fn is_button_up(&self, button: MouseButton) -> bool {
        matches!(&self.event_type, MouseEventType::ButtonUp(b) if *b == button)
    }
}

/// 输入事件（键盘或鼠标）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

impl InputEvent {
    pub fn timestamp(&self) -> Timestamp {
        match self {
            InputEvent::Key(e) => e.timestamp,
            InputEvent::Mouse(e) => e.timestamp,
        }
    }

    pub fn is_injected(&self) -> bool {
        match self {
            InputEvent::Key(e) => e.is_injected,
            InputEvent::Mouse(e) => e.is_injected,
        }
    }
}
