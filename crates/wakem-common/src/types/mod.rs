pub mod action;
pub mod input;
pub mod layer;
pub mod mapping;

pub use action::*;
pub use input::*;
pub use layer::*;
pub use mapping::*;

use serde::{Deserialize, Serialize};

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Keyboard,
    Mouse,
}

/// 按键状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyState {
    Pressed,
    Released,
}

/// 修饰键状态
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool, // Windows 键 / Command 键
}

impl ModifierState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.meta
    }

    pub fn from_virtual_key(key: u16, pressed: bool) -> Option<(Self, bool)> {
        let mut state = Self::new();
        match key {
            0x10 | 0xA0 | 0xA1 => state.shift = pressed, // VK_SHIFT, VK_LSHIFT, VK_RSHIFT
            0x11 | 0xA2 | 0xA3 => state.ctrl = pressed,  // VK_CONTROL, VK_LCONTROL, VK_RCONTROL
            0x12 | 0xA4 | 0xA5 => state.alt = pressed,   // VK_MENU, VK_LMENU, VK_RMENU
            0x5B | 0x5C => state.meta = pressed,         // VK_LWIN, VK_RWIN
            _ => return None,
        }
        Some((state, pressed))
    }

    pub fn merge(&mut self, other: &ModifierState) {
        self.shift |= other.shift;
        self.ctrl |= other.ctrl;
        self.alt |= other.alt;
        self.meta |= other.meta;
    }
}

/// 时间戳（毫秒）
pub type Timestamp = u64;

/// 获取当前时间戳
pub fn now() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
