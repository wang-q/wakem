use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::types::{MappingRule, ModifierState};

/// 全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 日志级别
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// 是否显示系统托盘图标
    #[serde(default = "default_true")]
    pub tray_icon: bool,
    /// 是否自动重新加载配置
    #[serde(default = "default_true")]
    pub auto_reload: bool,
    /// 键盘映射规则
    #[serde(default)]
    pub keyboard: KeyboardConfig,
    /// 窗口设置
    #[serde(default)]
    pub window: WindowConfig,
    /// 鼠标设置
    #[serde(default)]
    pub mouse: MouseConfig,
    /// 启动项设置
    #[serde(default)]
    pub launch: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            tray_icon: true,
            auto_reload: true,
            keyboard: KeyboardConfig::default(),
            window: WindowConfig::default(),
            mouse: MouseConfig::default(),
            launch: HashMap::new(),
        }
    }
}

impl Config {
    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
    }

    /// 从字符串解析配置
    pub fn from_str(content: &str) -> anyhow::Result<Self> {
        let config: Config = toml::from_str(content)?;
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 获取所有映射规则
    pub fn get_all_rules(&self) -> Vec<MappingRule> {
        let mut rules = Vec::new();
        rules.extend(self.keyboard.remap.iter().filter_map(|(k, v)| {
            parse_key_mapping(k, v).ok()
        }));
        rules
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_true() -> bool {
    true
}

/// 键盘配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyboardConfig {
    /// 键位重映射（简单映射）
    #[serde(default)]
    pub remap: HashMap<String, String>,
    /// 快捷键层
    #[serde(default)]
    pub layers: HashMap<String, LayerConfig>,
}

/// 层配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    /// 层激活键
    pub activation_key: String,
    /// 层内映射
    pub mappings: HashMap<String, String>,
    /// 层激活模式
    #[serde(default)]
    pub mode: LayerMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LayerMode {
    /// 按住激活，释放退出
    #[default]
    Hold,
    /// 切换模式（按一次进入，再按一次退出）
    Toggle,
}

/// 窗口配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowConfig {
    /// 窗口切换设置
    #[serde(default)]
    pub switch: WindowSwitchConfig,
    /// 窗口位置预设
    #[serde(default)]
    pub positions: HashMap<String, WindowPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WindowSwitchConfig {
    /// 是否忽略最小化窗口
    #[serde(default = "default_true")]
    pub ignore_minimal: bool,
    /// 是否只在当前虚拟桌面切换
    #[serde(default = "default_true")]
    pub only_current_desktop: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// 鼠标配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MouseConfig {
    /// 按钮重映射
    #[serde(default)]
    pub button_remap: HashMap<String, String>,
    /// 滚轮设置
    #[serde(default)]
    pub wheel: WheelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WheelConfig {
    /// 滚轮速度
    #[serde(default = "default_wheel_speed")]
    pub speed: i32,
    /// 是否反转滚轮方向
    #[serde(default)]
    pub invert: bool,
}

fn default_wheel_speed() -> i32 {
    3
}

/// 解析简单的键位映射配置
/// 格式: "CapsLock" -> "Backspace"
fn parse_key_mapping(from: &str, to: &str) -> anyhow::Result<MappingRule> {
    use crate::types::{Action, KeyAction, Trigger};

    let from_key = parse_key(from)?;
    let to_key = parse_key(to)?;

    let trigger = Trigger::key(from_key.0, from_key.1);
    let action = Action::key(KeyAction::click(to_key.0, to_key.1));

    Ok(MappingRule::new(trigger, action))
}

/// 解析键名到扫描码和虚拟键码
fn parse_key(name: &str) -> anyhow::Result<(u16, u16)> {
    // 常见键名映射
    let result = match name.to_lowercase().as_str() {
        "capslock" | "caps" => (0x3A, 0x14),
        "backspace" => (0x0E, 0x08),
        "enter" | "return" => (0x1C, 0x0D),
        "escape" | "esc" => (0x01, 0x1B),
        "space" => (0x39, 0x20),
        "tab" => (0x0F, 0x09),
        "left" => (0x4B, 0x25),
        "up" => (0x48, 0x26),
        "right" => (0x4D, 0x27),
        "down" => (0x50, 0x28),
        "home" => (0x47, 0x24),
        "end" => (0x4F, 0x23),
        "pageup" => (0x49, 0x21),
        "pagedown" => (0x51, 0x22),
        "delete" | "del" => (0x53, 0x2E),
        "insert" | "ins" => (0x52, 0x2D),
        "lshift" => (0x2A, 0xA0),
        "rshift" => (0x36, 0xA1),
        "lctrl" | "lcontrol" => (0x1D, 0xA2),
        "rctrl" | "rcontrol" => (0xE01D, 0xA3),
        "lalt" => (0x38, 0xA4),
        "ralt" => (0xE038, 0xA5),
        "lwin" | "lmeta" => (0xE05B, 0x5B),
        "rwin" | "rmeta" => (0xE05C, 0x5C),
        // 字母键
        "a" => (0x1E, 0x41),
        "b" => (0x30, 0x42),
        "c" => (0x2E, 0x43),
        "d" => (0x20, 0x44),
        "e" => (0x12, 0x45),
        "f" => (0x21, 0x46),
        "g" => (0x22, 0x47),
        "h" => (0x23, 0x48),
        "i" => (0x17, 0x49),
        "j" => (0x24, 0x4A),
        "k" => (0x25, 0x4B),
        "l" => (0x26, 0x4C),
        "m" => (0x32, 0x4D),
        "n" => (0x31, 0x4E),
        "o" => (0x18, 0x4F),
        "p" => (0x19, 0x50),
        "q" => (0x10, 0x51),
        "r" => (0x13, 0x52),
        "s" => (0x1F, 0x53),
        "t" => (0x14, 0x54),
        "u" => (0x16, 0x55),
        "v" => (0x2F, 0x56),
        "w" => (0x11, 0x57),
        "x" => (0x2D, 0x58),
        "y" => (0x15, 0x59),
        "z" => (0x2C, 0x5A),
        // 数字键
        "0" => (0x0B, 0x30),
        "1" => (0x02, 0x31),
        "2" => (0x03, 0x32),
        "3" => (0x04, 0x33),
        "4" => (0x05, 0x34),
        "5" => (0x06, 0x35),
        "6" => (0x07, 0x36),
        "7" => (0x08, 0x37),
        "8" => (0x09, 0x38),
        "9" => (0x0A, 0x39),
        // F1-F12
        "f1" => (0x3B, 0x70),
        "f2" => (0x3C, 0x71),
        "f3" => (0x3D, 0x72),
        "f4" => (0x3E, 0x73),
        "f5" => (0x3F, 0x74),
        "f6" => (0x40, 0x75),
        "f7" => (0x41, 0x76),
        "f8" => (0x42, 0x77),
        "f9" => (0x43, 0x78),
        "f10" => (0x44, 0x79),
        "f11" => (0x57, 0x7A),
        "f12" => (0x58, 0x7B),
        _ => {
            return Err(anyhow::anyhow!("Unknown key name: {}", name));
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parse() {
        let config_str = r#"
log_level = "debug"
tray_icon = true
auto_reload = true

[keyboard.remap]
CapsLock = "Backspace"

[[keyboard.layers]]
name = "navigate"
activation_key = "CapsLock"
"#;

        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.log_level, "debug");
        assert!(config.tray_icon);
        assert!(config.keyboard.remap.contains_key("CapsLock"));
    }

    #[test]
    fn test_parse_key() {
        assert_eq!(parse_key("capslock").unwrap(), (0x3A, 0x14));
        assert_eq!(parse_key("a").unwrap(), (0x1E, 0x41));
        assert_eq!(parse_key("1").unwrap(), (0x02, 0x31));
    }
}
