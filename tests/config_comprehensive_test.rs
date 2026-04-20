// Config 完整测试 - 补充配置解析器的边界条件和完整场景测试

use wakem::config::{wildcard_match, Config, WindowPreset};
use wakem::types::{Action, Trigger, WindowAction};

// ==================== TOML 解析完整性测试 ====================

/// 测试完整配置文件（包含所有配置项）
#[test]
fn test_parse_complete_config() {
    let config_str = r#"
log_level = "debug"
tray_icon = false
auto_reload = false

[keyboard.remap]
CapsLock = "Backspace"
Escape = "CapsLock"

[keyboard.layers.navigate]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"

[keyboard.layers.symbols]
activation_key = "Space"
mode = "Toggle"

[keyboard.layers.symbols.mappings]
A = "1"

[window.shortcuts]
"Ctrl+Alt+C" = "Center"

[mouse.wheel]
speed = 5
invert = true
acceleration = true

[launch]
F1 = "notepad.exe"

[network]
enabled = true
instance_id = 5

[macros]
test_macro = []

[macro_bindings]
F5 = "test_macro"
"#;

    let config: Config = toml::from_str(config_str).unwrap();

    // 验证主要配置项正确解析
    assert_eq!(config.log_level, "debug");
    assert!(!config.tray_icon);
    assert!(!config.auto_reload);
    assert_eq!(config.keyboard.remap.len(), 2);
    assert_eq!(config.keyboard.layers.len(), 2);
    assert_eq!(config.window.shortcuts.len(), 1);
    assert_eq!(config.mouse.wheel.speed, 5);
    assert!(config.mouse.wheel.invert);
    assert!(config.mouse.wheel.acceleration);
    assert_eq!(config.launch.len(), 1);
    assert!(config.network.enabled);
    assert_eq!(config.network.instance_id, 5);
    assert!(config.macros.contains_key("test_macro"));
}

/// 测试最小化配置（使用默认值）
#[test]
fn test_parse_minimal_config() {
    let config_str = r#"
"#;

    let config: Config = toml::from_str(config_str).unwrap();

    // 验证默认值
    assert_eq!(config.log_level, "info");
    assert!(config.tray_icon);
    assert!(config.auto_reload);
    assert!(config.icon_path.is_none());
    assert!(config.keyboard.remap.is_empty());
    assert!(config.keyboard.layers.is_empty());
    assert!(config.window.shortcuts.is_empty());
    assert!(config.launch.is_empty());
    assert!(!config.network.enabled);
    assert_eq!(config.network.instance_id, 0);
    assert!(config.network.auth_key.is_none());
    assert!(config.macros.is_empty());
}

// ==================== 键名解析全面测试 ====================

/// 测试所有字母键 A-Z
#[test]
fn test_parse_all_letter_keys() {
    use wakem::config::parse_key;

    let letter_keys = vec![
        ("a", (0x1E, 0x41)),
        ("b", (0x30, 0x42)),
        ("c", (0x2E, 0x43)),
        ("d", (0x20, 0x44)),
        ("e", (0x12, 0x45)),
        ("f", (0x21, 0x46)),
        ("g", (0x22, 0x47)),
        ("h", (0x23, 0x48)),
        ("i", (0x17, 0x49)),
        ("j", (0x24, 0x4A)),
        ("k", (0x25, 0x4B)),
        ("l", (0x26, 0x4C)),
        ("m", (0x32, 0x4D)),
        ("n", (0x31, 0x4E)),
        ("o", (0x18, 0x4F)),
        ("p", (0x19, 0x50)),
        ("q", (0x10, 0x51)),
        ("r", (0x13, 0x52)),
        ("s", (0x1F, 0x53)),
        ("t", (0x14, 0x54)),
        ("u", (0x16, 0x55)),
        ("v", (0x2F, 0x56)),
        ("w", (0x11, 0x57)),
        ("x", (0x2D, 0x58)),
        ("y", (0x15, 0x59)),
        ("z", (0x2C, 0x5A)),
    ];

    for (name, expected) in letter_keys {
        let result = parse_key(name).unwrap();
        assert_eq!(result, expected, "键 '{}' 的扫描码/虚拟键码不匹配", name);
    }
}

/// 测试所有数字键 0-9
#[test]
fn test_parse_all_number_keys() {
    use wakem::config::parse_key;

    let number_keys = vec![
        ("0", (0x0B, 0x30)),
        ("1", (0x02, 0x31)),
        ("2", (0x03, 0x32)),
        ("3", (0x04, 0x33)),
        ("4", (0x05, 0x34)),
        ("5", (0x06, 0x35)),
        ("6", (0x07, 0x36)),
        ("7", (0x08, 0x37)),
        ("8", (0x09, 0x38)),
        ("9", (0x0A, 0x39)),
    ];

    for (name, expected) in number_keys {
        let result = parse_key(name).unwrap();
        assert_eq!(result, expected, "数字键 '{}' 不匹配", name);
    }
}

/// 测试功能键 F1-F12
#[test]
fn test_parse_function_keys_f1_f12() {
    use wakem::config::parse_key;

    let func_keys = vec![
        ("f1", (0x3B, 0x70)),
        ("f2", (0x3C, 0x71)),
        ("f3", (0x3D, 0x72)),
        ("f4", (0x3E, 0x73)),
        ("f5", (0x3F, 0x74)),
        ("f6", (0x40, 0x75)),
        ("f7", (0x41, 0x76)),
        ("f8", (0x42, 0x77)),
        ("f9", (0x43, 0x78)),
        ("f10", (0x44, 0x79)),
        ("f11", (0x57, 0x7A)),
        ("f12", (0x58, 0x7B)),
    ];

    for (name, expected) in func_keys {
        let result = parse_key(name).unwrap();
        assert_eq!(result, expected, "功能键 '{}' 不匹配", name);
    }

    // 大写测试
    let result = parse_key("F1").unwrap();
    assert_eq!(result, (0x3B, 0x70));
}

/// 测试特殊键（CapsLock, Enter, Escape 等）
#[test]
fn test_parse_special_keys() {
    use wakem::config::parse_key;

    let special_keys = vec![
        ("capslock", (0x3A, 0x14)),
        ("backspace", (0x0E, 0x08)),
        ("enter", (0x1C, 0x0D)),
        ("escape", (0x01, 0x1B)),
        ("space", (0x39, 0x20)),
        ("tab", (0x0F, 0x09)),
        ("delete", (0x53, 0x2E)),
        ("insert", (0x52, 0x2D)),
        ("home", (0x47, 0x24)),
        ("end", (0x4F, 0x23)),
        ("pageup", (0x49, 0x21)),
        ("pagedown", (0x51, 0x22)),
    ];

    for (name, expected) in special_keys {
        let result = parse_key(name).unwrap();
        assert_eq!(result, expected, "特殊键 '{}' 不匹配", name);
    }
}

/// 测试方向键和导航键
#[test]
fn test_parse_navigation_keys() {
    use wakem::config::parse_key;

    let nav_keys = vec![
        ("left", (0x4B, 0x25)),
        ("right", (0x4D, 0x27)),
        ("up", (0x48, 0x26)),
        ("down", (0x50, 0x28)),
    ];

    for (name, expected) in nav_keys {
        let result = parse_key(name).unwrap();
        assert_eq!(result, expected, "导航键 '{}' 不匹配", name);
    }
}

/// 测试修饰键（LCtrl, RAlt, LWin 等）
#[test]
fn test_parse_modifier_keys() {
    use wakem::config::parse_key;

    let modifier_keys = vec![
        ("lshift", (0x2A, 0xA0)),
        ("rshift", (0x36, 0xA1)),
        ("lctrl", (0x1D, 0xA2)),
        ("rctrl", (0xE01D, 0xA3)),
        ("lalt", (0x38, 0xA4)),
        ("ralt", (0xE038, 0xA5)),
        ("lwin", (0xE05B, 0x5B)),
        ("rwin", (0xE05C, 0x5C)),
    ];

    for (name, expected) in modifier_keys {
        let result = parse_key(name).unwrap();
        assert_eq!(result, expected, "修饰键 '{}' 不匹配", name);
    }
}

/// 测试未知键名的错误处理
#[test]
fn test_parse_unknown_key_error() {
    use wakem::config::parse_key;

    let unknown_keys = vec!["unknown", "invalid", "xyz", "key123", "", " "];

    for name in unknown_keys {
        let result = parse_key(name);
        assert!(result.is_err(), "未知键名 '{}' 应该返回错误", name);
    }
}

/// 测试大小写不敏感
#[test]
fn test_parse_key_case_insensitive() {
    use wakem::config::parse_key;

    let cases = vec![
        ("CapsLock", (0x3A, 0x14)),
        ("CAPSLOCK", (0x3A, 0x14)),
        ("capslock", (0x3A, 0x14)),
        ("Enter", (0x1C, 0x0D)),
        ("ENTER", (0x1C, 0x0D)),
        ("enter", (0x1C, 0x0D)),
        ("Space", (0x39, 0x20)),
        ("SPACE", (0x39, 0x20)),
        ("space", (0x39, 0x20)),
    ];

    for (name, expected) in cases {
        let result = parse_key(name).unwrap();
        assert_eq!(result, expected, "'{}' 大小写不敏感测试失败", name);
    }
}

// ==================== 映射规则解析测试 ====================

/// 测试简单键位映射（通过配置加载验证）
#[test]
fn test_parse_simple_key_mapping_via_config() {
    let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert!(config.keyboard.remap.contains_key("CapsLock"));
}

/// 测试键到窗口动作的映射（通过配置加载验证）
#[test]
fn test_parse_key_to_window_action_via_config() {
    let config_str = r#"
[keyboard.remap]
F1 = "Center"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert!(config.keyboard.remap.contains_key("F1"));
}

/// 测试键到修饰键组合的映射（通过配置加载验证）
#[test]
fn test_parse_key_to_modifier_combo_via_config() {
    let config_str = r#"
[keyboard.remap]
CapsLock = "Ctrl+Alt+Win"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert!(config.keyboard.remap.contains_key("CapsLock"));
}

/// 测试快捷键到窗口动作的映射（通过配置加载验证）
#[test]
fn test_parse_shortcut_to_window_action_via_config() {
    let config_str = r#"
[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert_eq!(config.window.shortcuts.len(), 1);
}

// ==================== 窗口预设和通配符测试 ====================

/// 测试按进程名匹配
#[test]
fn test_window_preset_match_by_process_name() {
    let preset = WindowPreset {
        name: "Chrome".to_string(),
        process_name: Some("chrome.exe".to_string()),
        executable_path: None,
        title_pattern: None,
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    assert!(preset.matches("chrome.exe", None, "Google Chrome"));
    assert!(!preset.matches("firefox.exe", None, "Firefox"));
}

/// 测试按可执行路径匹配
#[test]
fn test_window_preset_match_by_executable_path() {
    let preset = WindowPreset {
        name: "VS Code".to_string(),
        process_name: None,
        executable_path: Some("C:\\Program Files\\VS Code\\Code.exe".to_string()),
        title_pattern: None,
        x: 100,
        y: 50,
        width: 1200,
        height: 800,
    };

    assert!(preset.matches(
        "Code.exe",
        Some("C:\\Program Files\\VS Code\\Code.exe"),
        "Visual Studio Code"
    ));
    assert!(!preset.matches(
        "Code.exe",
        Some("C:\\Other\\Path\\Code.exe"),
        "Visual Studio Code"
    ));
}

/// 测试按窗口标题模式匹配
#[test]
fn test_window_preset_match_by_title_pattern() {
    let preset = WindowPreset {
        name: "Editor".to_string(),
        process_name: None,
        executable_path: None,
        title_pattern: Some("*Visual Studio Code*".to_string()),
        x: 0,
        y: 0,
        width: 800,
        height: 600,
    };

    assert!(preset.matches(
        "Code.exe",
        None,
        "Visual Studio Code - project/src/main.rs"
    ));
    assert!(!preset.matches("notepad.exe", None, "Untitled - Notepad"));
}

/// 测试 * 通配符
#[test]
fn test_wildcard_match_star() {
    assert!(wildcard_match("test.exe", "*.exe"));
    assert!(wildcard_match("file.txt", "*.txt"));
    assert!(wildcard_match("document.pdf", "*.pdf"));
    assert!(wildcard_match("any.txt", "*"));
    assert!(wildcard_match("", "*"));
    assert!(!wildcard_match("test.exe", "*.txt"));
}

/// 测试 ? 通配符
#[test]
fn test_wildcard_match_question_mark() {
    assert!(wildcard_match("abc", "a?c"));
    assert!(wildcard_match("abc", "???"));
    assert!(!wildcard_match("ab", "a?c")); // 太短
    assert!(!wildcard_match("abcd", "a?c")); // 太长
}

/// 测试复杂通配符模式
#[test]
fn test_wildcard_match_complex_patterns() {
    assert!(wildcard_match("test_file_name.txt", "test*.txt"));
    assert!(wildcard_match("file123.pdf", "file???.pdf"));
    assert!(wildcard_match("abc123def", "abc*def"));
    assert!(wildcard_match("a.b.c", "a.*.c"));
}

/// 测试通配符边界情况
#[test]
fn test_wildcard_match_edge_cases() {
    // 空字符串
    assert!(wildcard_match("", ""));
    assert!(wildcard_match("", "*"));

    // 多个连续 *
    assert!(wildcard_match("test", "**test**"));
    assert!(wildcard_match("test", "***"));

    // 大小写不敏感
    assert!(wildcard_match("TEST.EXE", "*.exe"));
    assert!(wildcard_match("File.TXT", "file.*"));

    // 特殊字符
    assert!(wildcard_match("test-file_v1.2.txt", "test-*.txt"));
}

// ==================== 配置默认值验证 ====================

/// 测试 KeyboardConfig 默认值
#[test]
fn test_keyboard_config_default() {
    let config = wakem::config::KeyboardConfig::default();
    assert!(config.remap.is_empty());
    assert!(config.layers.is_empty());
    assert!(config.context_mappings.is_empty());
}

/// 测试 MouseConfig 默认值
#[test]
fn test_mouse_config_default() {
    let config = wakem::config::MouseConfig::default();
    assert!(config.button_remap.is_empty());
    assert_eq!(config.wheel.speed, 3);
    assert!(!config.wheel.invert);
    assert!(!config.wheel.acceleration);
    assert!((config.wheel.acceleration_multiplier - 2.0).abs() < 0.001);
}

/// 测试 NetworkConfig 默认值
#[test]
fn test_network_config_default() {
    let config = wakem::config::NetworkConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.instance_id, 0);
    assert!(config.auth_key.is_none());
}

/// 测试 WindowConfig 默认值
#[test]
fn test_window_config_default() {
    let config = wakem::config::WindowConfig::default();
    // 注意：WindowSwitchConfig 和 auto_apply_preset 使用了 #[serde(default)]
    // 但 Default trait 不会使用这些 serde 默认值
    assert!(!config.switch.ignore_minimal);
    assert!(!config.switch.only_current_desktop);
    assert!(config.positions.is_empty());
    assert!(config.shortcuts.is_empty());
    assert!(config.presets.is_empty());
    // auto_apply_preset 在 serde 反序列化时默认为 true，
    // 但 Default trait 中默认为 false（bool 的默认值）
    assert!(!config.auto_apply_preset);
}
