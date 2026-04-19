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
        rules.extend(self.keyboard.layers.iter().filter_map(|(name, layer)| {
            self.parse_layer_mappings(name, layer).ok()
        }).flatten());
        rules.extend(self.window.shortcuts.iter().filter_map(|(k, v)| {
            parse_window_shortcut(k, v).ok()
        }));
        rules
    }

    /// 解析层的映射规则
    fn parse_layer_mappings(&self, layer_name: &str, layer: &LayerConfig) -> anyhow::Result<Vec<MappingRule>> {
        use crate::types::{Action, KeyAction, Trigger, ModifierState};

        let mut rules = Vec::new();

        // 解析激活键
        let activation_key = parse_key(&layer.activation_key)?;
        let activation_trigger = Trigger::key(activation_key.0, activation_key.1);

        // 根据模式创建层切换动作
        let layer_action = match layer.mode {
            LayerMode::Hold => Action::key(KeyAction::Press {
                scan_code: activation_key.0,
                virtual_key: activation_key.1,
            }),
            LayerMode::Toggle => Action::key(KeyAction::Click {
                scan_code: activation_key.0,
                virtual_key: activation_key.1,
            }),
        };

        // 添加层激活规则
        rules.push(MappingRule::new(activation_trigger, layer_action));

        // 解析层内的映射
        for (from, to) in &layer.mappings {
            if let Ok(from_key) = parse_key(from) {
                // 检查是否是窗口管理动作
                if let Ok(window_action) = parse_window_action(to) {
                    let trigger = Trigger::key(from_key.0, from_key.1);
                    let action = Action::window(window_action);
                    rules.push(MappingRule::new(trigger, action));
                } else if let Ok(to_key) = parse_key(to) {
                    let trigger = Trigger::key(from_key.0, from_key.1);
                    let action = Action::key(KeyAction::click(to_key.0, to_key.1));
                    rules.push(MappingRule::new(trigger, action));
                }
            }
        }

        Ok(rules)
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
    /// 窗口管理快捷键（借鉴 mrw）
    #[serde(default)]
    pub shortcuts: HashMap<String, String>,
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
/// 格式: "CapsLock" -> "Ctrl+Alt+Win" (映射为修饰键组合)
fn parse_key_mapping(from: &str, to: &str) -> anyhow::Result<MappingRule> {
    use crate::types::{Action, KeyAction, Trigger};

    let from_key = parse_key(from)?;

    // 检查是否是窗口管理动作
    if let Ok(window_action) = parse_window_action(to) {
        let trigger = Trigger::key(from_key.0, from_key.1);
        let action = Action::window(window_action);
        return Ok(MappingRule::new(trigger, action));
    }

    // 检查目标是否是带修饰键的快捷键（如 "Ctrl+Alt+Win"）
    if to.contains('+') && !to.contains("->") {
        // 解析修饰键组合（如 "Ctrl+Alt+Win"）
        let modifiers = parse_modifier_combo(to)?;
        let trigger = Trigger::key(from_key.0, from_key.1);
        // 创建发送修饰键按下/释放的动作序列
        let action = create_modifier_press_release_action(&modifiers);
        return Ok(MappingRule::new(trigger, action));
    }

    let to_key = parse_key(to)?;

    let trigger = Trigger::key(from_key.0, from_key.1);
    let action = Action::key(KeyAction::click(to_key.0, to_key.1));

    Ok(MappingRule::new(trigger, action))
}

/// 解析纯修饰键组合（如 "Ctrl+Alt+Win"）
fn parse_modifier_combo(s: &str) -> anyhow::Result<crate::types::ModifierState> {
    use crate::types::ModifierState;

    let mut modifiers = ModifierState::default();
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.ctrl = true,
            "alt" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "win" | "meta" | "command" => modifiers.meta = true,
            _ => {
                // 如果不是已知的修饰键，返回错误
                return Err(anyhow::anyhow!("Unknown modifier: {}", part));
            }
        }
    }

    Ok(modifiers)
}

/// 创建修饰键按下和释放的动作序列
/// 当按下 CapsLock 时，发送 Ctrl+Alt+Win 的按下，释放时发送释放
fn create_modifier_press_release_action(modifiers: &crate::types::ModifierState) -> crate::types::Action {
    use crate::types::{Action, KeyAction};

    let mut actions = Vec::new();

    // 按下修饰键（按特定顺序：Ctrl -> Alt -> Win -> Shift）
    if modifiers.ctrl {
        actions.push(Action::key(KeyAction::press(0x1D, 0x11))); // Ctrl
    }
    if modifiers.alt {
        actions.push(Action::key(KeyAction::press(0x38, 0x12))); // Alt
    }
    if modifiers.meta {
        actions.push(Action::key(KeyAction::press(0x5B, 0x5B))); // Win (Left)
    }
    if modifiers.shift {
        actions.push(Action::key(KeyAction::press(0x2A, 0x10))); // Shift
    }

    // 立即释放修饰键（逆序）
    if modifiers.shift {
        actions.push(Action::key(KeyAction::release(0x2A, 0x10))); // Shift
    }
    if modifiers.meta {
        actions.push(Action::key(KeyAction::release(0x5B, 0x5B))); // Win
    }
    if modifiers.alt {
        actions.push(Action::key(KeyAction::release(0x38, 0x12))); // Alt
    }
    if modifiers.ctrl {
        actions.push(Action::key(KeyAction::release(0x1D, 0x11))); // Ctrl
    }

    Action::Sequence(actions)
}

/// 解析窗口管理快捷键
/// 格式: "Ctrl+Alt+C" -> "Center"
fn parse_window_shortcut(from: &str, to: &str) -> anyhow::Result<MappingRule> {
    use crate::types::{Action, Trigger};

    // 解析快捷键（如 "Ctrl+Alt+C"）
    let trigger = parse_shortcut_trigger(from)?;

    // 解析窗口管理动作
    let window_action = parse_window_action(to)?;
    let action = Action::window(window_action);

    Ok(MappingRule::new(trigger, action))
}

/// 解析快捷键触发器
/// 格式: "Ctrl+Alt+C", "Ctrl+Alt+Win+Left"
fn parse_shortcut_trigger(shortcut: &str) -> anyhow::Result<crate::types::Trigger> {
    use crate::types::{ModifierState, Trigger};

    let parts: Vec<&str> = shortcut.split('+').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty shortcut"));
    }

    let mut modifiers = ModifierState::new();
    let mut key_name = "";

    for part in &parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers.ctrl = true,
            "alt" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "win" | "meta" | "command" | "cmd" => modifiers.meta = true,
            _ => key_name = part,
        }
    }

    if key_name.is_empty() {
        return Err(anyhow::anyhow!("No key specified in shortcut: {}", shortcut));
    }

    let key = parse_key(key_name)?;
    Ok(Trigger::key_with_modifiers(key.0, key.1, modifiers))
}

/// 解析窗口管理动作
/// 格式: "Center", "MoveToEdge(Left)", "HalfScreen(Right)", "FixedRatio(1.333, 0)"
fn parse_window_action(action_str: &str) -> anyhow::Result<crate::types::WindowAction> {
    use crate::types::{Alignment, Edge, MonitorDirection, WindowAction};

    let action_str = action_str.trim();

    // 简单动作（无参数）
    match action_str {
        "Center" => return Ok(WindowAction::Center),
        "SwitchToNextWindow" => return Ok(WindowAction::SwitchToNextWindow),
        "Minimize" => return Ok(WindowAction::Minimize),
        "Maximize" => return Ok(WindowAction::Maximize),
        "Restore" => return Ok(WindowAction::Restore),
        "Close" => return Ok(WindowAction::Close),
        "ToggleTopmost" => return Ok(WindowAction::ToggleTopmost),
        _ => {}
    }

    // 带参数的动作
    if let Some((name, params)) = action_str.split_once('(') {
        let params = params.trim_end_matches(')');
        let param_list: Vec<&str> = params.split(',').map(|s| s.trim()).collect();

        match name.trim() {
            "MoveToEdge" => {
                let edge = parse_edge(param_list.get(0).unwrap_or(&""))?;
                Ok(WindowAction::MoveToEdge(edge))
            }
            "HalfScreen" => {
                let edge = parse_edge(param_list.get(0).unwrap_or(&""))?;
                Ok(WindowAction::HalfScreen(edge))
            }
            "LoopWidth" => {
                let align = parse_alignment(param_list.get(0).unwrap_or(&""))?;
                Ok(WindowAction::LoopWidth(align))
            }
            "LoopHeight" => {
                let align = parse_alignment(param_list.get(0).unwrap_or(&""))?;
                Ok(WindowAction::LoopHeight(align))
            }
            "FixedRatio" => {
                let ratio = param_list
                    .get(0)
                    .unwrap_or(&"1.333")
                    .parse::<f32>()?;
                let scale_index = param_list
                    .get(1)
                    .unwrap_or(&"0")
                    .parse::<usize>()?;
                Ok(WindowAction::FixedRatio { ratio, scale_index })
            }
            "NativeRatio" => {
                let scale_index = param_list
                    .get(0)
                    .unwrap_or(&"0")
                    .parse::<usize>()?;
                Ok(WindowAction::NativeRatio { scale_index })
            }
            "MoveToMonitor" => {
                let direction = parse_monitor_direction(param_list.get(0).unwrap_or(&""))?;
                Ok(WindowAction::MoveToMonitor(direction))
            }
            "Move" => {
                let x = param_list.get(0).unwrap_or(&"0").parse::<i32>()?;
                let y = param_list.get(1).unwrap_or(&"0").parse::<i32>()?;
                Ok(WindowAction::Move { x, y })
            }
            "Resize" => {
                let width = param_list.get(0).unwrap_or(&"800").parse::<i32>()?;
                let height = param_list.get(1).unwrap_or(&"600").parse::<i32>()?;
                Ok(WindowAction::Resize { width, height })
            }
            "SetOpacity" => {
                let opacity = param_list.get(0).unwrap_or(&"255").parse::<u8>()?;
                Ok(WindowAction::SetOpacity { opacity })
            }
            _ => Err(anyhow::anyhow!("Unknown window action: {}", name)),
        }
    } else {
        Err(anyhow::anyhow!("Invalid window action format: {}", action_str))
    }
}

/// 解析边缘参数
fn parse_edge(s: &str) -> anyhow::Result<crate::types::Edge> {
    use crate::types::Edge;

    match s.trim().to_lowercase().as_str() {
        "left" => Ok(Edge::Left),
        "right" => Ok(Edge::Right),
        "top" => Ok(Edge::Top),
        "bottom" => Ok(Edge::Bottom),
        _ => Err(anyhow::anyhow!("Unknown edge: {}", s)),
    }
}

/// 解析对齐参数
fn parse_alignment(s: &str) -> anyhow::Result<crate::types::Alignment> {
    use crate::types::Alignment;

    match s.trim().to_lowercase().as_str() {
        "left" => Ok(Alignment::Left),
        "right" => Ok(Alignment::Right),
        "top" => Ok(Alignment::Top),
        "bottom" => Ok(Alignment::Bottom),
        "center" => Ok(Alignment::Center),
        _ => Err(anyhow::anyhow!("Unknown alignment: {}", s)),
    }
}

/// 解析显示器方向参数
fn parse_monitor_direction(s: &str) -> anyhow::Result<crate::types::MonitorDirection> {
    use crate::types::MonitorDirection;

    match s.trim().to_lowercase().as_str() {
        "next" => Ok(MonitorDirection::Next),
        "prev" | "previous" => Ok(MonitorDirection::Prev),
        s => {
            if let Ok(index) = s.parse::<i32>() {
                Ok(MonitorDirection::Index(index))
            } else {
                Err(anyhow::anyhow!("Unknown monitor direction: {}", s))
            }
        }
    }
}

/// 解析键名到扫描码和虚拟键码
pub fn parse_key(name: &str) -> anyhow::Result<(u16, u16)> {
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

/// 解析配置文件路径
/// 如果提供了路径，使用提供的路径；否则使用默认路径
pub fn resolve_config_file_path(path: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    if let Some(p) = path {
        return Some(p.to_path_buf());
    }

    // 尝试默认路径
    let home = std::env::var("USERPROFILE").ok()?;
    let home_path = std::path::PathBuf::from(home);
    
    // 优先检查 wakem.toml
    let config_file = home_path.join("wakem.toml");
    if config_file.exists() {
        return Some(config_file);
    }
    
    // 检查 .config/wakem/config.toml
    let config_dir = home_path.join(".config").join("wakem");
    let config_file = config_dir.join("config.toml");
    if config_file.exists() {
        return Some(config_file);
    }
    
    // 返回默认路径（即使不存在）
    Some(home_path.join("wakem.toml"))
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

[keyboard.layers.navigate]
activation_key = "CapsLock"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"
"#;

        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.log_level, "debug");
        assert!(config.tray_icon);
        assert!(config.keyboard.remap.contains_key("CapsLock"));
        assert!(config.keyboard.layers.contains_key("navigate"));
    }

    #[test]
    fn test_parse_key() {
        assert_eq!(parse_key("capslock").unwrap(), (0x3A, 0x14));
        assert_eq!(parse_key("a").unwrap(), (0x1E, 0x41));
        assert_eq!(parse_key("1").unwrap(), (0x02, 0x31));
    }

    #[test]
    fn test_parse_key_mapping_with_modifiers() {
        // 测试 CapsLock -> Ctrl+Alt+Win 的映射
        let rule = parse_key_mapping("CapsLock", "Ctrl+Alt+Win").unwrap();
        
        // 验证触发器是 CapsLock
        if let crate::types::Trigger::Key { scan_code, virtual_key, .. } = &rule.trigger {
            assert_eq!(*scan_code, Some(0x3A));
            assert_eq!(*virtual_key, Some(0x14));
        } else {
            panic!("Expected Key trigger");
        }
        
        // 验证动作是 Sequence（包含修饰键按下/释放）
        if let crate::types::Action::Sequence(actions) = &rule.action {
            // 应该有 6 个动作：Ctrl按下、Alt按下、Win按下、Win释放、Alt释放、Ctrl释放
            assert_eq!(actions.len(), 6);
            
            // 验证第一个动作是 Ctrl 按下
            if let crate::types::Action::Key(crate::types::KeyAction::Press { virtual_key, .. }) = &actions[0] {
                assert_eq!(*virtual_key, 0x11); // VK_CONTROL
            } else {
                panic!("Expected Ctrl Press as first action, got {:?}", actions[0]);
            }
            
            // 验证第二个动作是 Alt 按下
            if let crate::types::Action::Key(crate::types::KeyAction::Press { virtual_key, .. }) = &actions[1] {
                assert_eq!(*virtual_key, 0x12); // VK_MENU (Alt)
            } else {
                panic!("Expected Alt Press as second action, got {:?}", actions[1]);
            }
            
            // 验证第三个动作是 Win 按下
            if let crate::types::Action::Key(crate::types::KeyAction::Press { virtual_key, .. }) = &actions[2] {
                assert_eq!(*virtual_key, 0x5B); // VK_LWIN
            } else {
                panic!("Expected Win Press as third action, got {:?}", actions[2]);
            }
            
            // 验证第四、五、六个动作是释放
            if let crate::types::Action::Key(crate::types::KeyAction::Release { virtual_key, .. }) = &actions[3] {
                assert_eq!(*virtual_key, 0x5B); // VK_LWIN release
            } else {
                panic!("Expected Win Release as fourth action, got {:?}", actions[3]);
            }
        } else {
            panic!("Expected Sequence action for modifier combo, got {:?}", rule.action);
        }
    }

    #[test]
    fn test_parse_modifier_combo() {
        // 测试解析修饰键组合
        let modifiers = parse_modifier_combo("Ctrl+Alt+Win").unwrap();
        assert!(modifiers.ctrl);
        assert!(modifiers.alt);
        assert!(modifiers.meta);
        assert!(!modifiers.shift);

        // 测试不同顺序
        let modifiers = parse_modifier_combo("Shift+Ctrl").unwrap();
        assert!(modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.meta);
        assert!(modifiers.shift);

        // 测试大小写不敏感
        let modifiers = parse_modifier_combo("ctrl+ALT+win").unwrap();
        assert!(modifiers.ctrl);
        assert!(modifiers.alt);
        assert!(modifiers.meta);
    }
}
