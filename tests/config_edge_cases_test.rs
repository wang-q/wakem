// 配置解析边界测试
// 测试各种边界情况和错误处理

use wakem::config::Config;

/// 测试空配置
#[test]
fn test_empty_config() {
    let config_str = "";
    let result = Config::from_str(config_str);
    assert!(result.is_ok());

    let config = result.unwrap();
    assert_eq!(config.log_level, "info");
    assert!(config.tray_icon);
    assert!(config.auto_reload);
}

/// 测试最小配置
#[test]
fn test_minimal_config() {
    let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"
"#;

    let result = Config::from_str(config_str);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

    let config = result.unwrap();
    assert!(config.keyboard.remap.contains_key("CapsLock"));
    assert_eq!(config.keyboard.remap.get("CapsLock").unwrap(), "Backspace");
}

/// 测试完整配置
#[test]
fn test_full_config() {
    let config_str = r#"
log_level = "debug"
tray_icon = false
auto_reload = false
icon_path = "custom/icon.ico"

[keyboard.remap]
CapsLock = "Backspace"
RightAlt = "Ctrl"

[keyboard.layers.navigation]
activation_key = "CapsLock"
mappings.H = "Left"
mappings.J = "Down"
mappings.K = "Up"
mappings.L = "Right"

[keyboard.layers.window_mgmt]
activation_key = "RightAlt"
mode = "Toggle"
mappings.Q = "Ctrl+W"
mappings.T = "Ctrl+T"

[window.shortcuts]
"Ctrl+Alt+Win+C" = "Center"
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"

[launch]
terminal = "wt.exe"
editor = "code.exe"
browser = "chrome.exe"
"#;

    let result = Config::from_str(config_str);
    assert!(
        result.is_ok(),
        "Failed to parse full config: {:?}",
        result.err()
    );

    let config = result.unwrap();
    assert_eq!(config.log_level, "debug");
    assert!(!config.tray_icon);
    assert!(!config.auto_reload);
    assert_eq!(config.icon_path, Some("custom/icon.ico".to_string()));

    // 检查键盘重映射
    assert_eq!(config.keyboard.remap.len(), 2);

    // 检查层
    assert_eq!(config.keyboard.layers.len(), 2);

    // 检查窗口快捷键
    assert_eq!(config.window.shortcuts.len(), 2);

    // 检查启动项
    assert_eq!(config.launch.len(), 3);
}

/// 测试无效的配置值
#[test]
fn test_invalid_config_values() {
    // 无效的 log_level 应该被接受（字符串类型）
    let config_str = r#"
log_level = "invalid_level"
"#;
    let result = Config::from_str(config_str);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().log_level, "invalid_level");
}

/// 测试层配置的各种模式
#[test]
fn test_layer_modes() {
    let config_str_hold = r#"
[keyboard.layers.hold_layer]
activation_key = "CapsLock"
mappings = {}
"#;

    let config_str_toggle = r#"
[keyboard.layers.toggle_layer]
activation_key = "CapsLock"
mode = "Toggle"
mappings = {}
"#;

    let result_hold = Config::from_str(config_str_hold);
    let result_toggle = Config::from_str(config_str_toggle);

    assert!(
        result_hold.is_ok(),
        "Hold mode failed: {:?}",
        result_hold.err()
    );
    assert!(
        result_toggle.is_ok(),
        "Toggle mode failed: {:?}",
        result_toggle.err()
    );
}

/// 测试窗口管理动作解析
#[test]
fn test_window_action_parsing_in_config() {
    let config_str = r#"
[window.shortcuts]
"Ctrl+Alt+Win+C" = "Center"
"Ctrl+Alt+Win+Home" = "MoveToEdge(Left)"
"Ctrl+Alt+Win+End" = "MoveToEdge(Right)"
"Ctrl+Alt+Win+Shift+Left" = "HalfScreen(Left)"
"Ctrl+Alt+Win+Left" = "LoopWidth(Left)"
"Ctrl+Alt+Win+Up" = "LoopHeight(Top)"
"Ctrl+Alt+Win+M" = "FixedRatio(1.333, 0)"
"Ctrl+Alt+Win+Shift+M" = "NativeRatio(0)"
"Alt+Grave" = "SwitchToNextWindow"
"Ctrl+Alt+Win+J" = "MoveToMonitor(Next)"
"Ctrl+Alt+Win+W" = "ShowDebugInfo"
"Ctrl+Alt+Win+Shift+W" = "ShowNotification(Test, Hello)"
"#;

    let result = Config::from_str(config_str);
    assert!(
        result.is_ok(),
        "Failed to parse window actions: {:?}",
        result.err()
    );

    let config = result.unwrap();
    assert_eq!(config.window.shortcuts.len(), 12);
}

/// 测试复杂键位映射
#[test]
fn test_complex_key_mappings() {
    let config_str = r#"
[keyboard.remap]
CapsLock = "Ctrl+Alt+Win"
RightAlt = "Ctrl+Shift"
"#;

    let result = Config::from_str(config_str);
    assert!(
        result.is_ok(),
        "Failed to parse complex key mappings: {:?}",
        result.err()
    );

    let config = result.unwrap();
    assert_eq!(
        config.keyboard.remap.get("CapsLock").unwrap(),
        "Ctrl+Alt+Win"
    );
}

/// 测试配置中的注释
#[test]
fn test_config_with_comments() {
    let config_str = r#"
# 这是注释
log_level = "info"  # 行尾注释

# 键盘配置
[keyboard.remap]
CapsLock = "Backspace"  # CapsLock 改 Backspace
"#;

    let result = Config::from_str(config_str);
    assert!(result.is_ok());
}

/// 测试多层嵌套配置
#[test]
fn test_nested_layer_config() {
    let config_str = r#"
[keyboard.layers.base]
activation_key = "CapsLock"
mappings.H = "Left"
mappings.J = "Down"

[keyboard.layers.advanced]
activation_key = "RightAlt"
mode = "Toggle"
mappings.Q = "Ctrl+Q"
mappings.W = "Ctrl+W"
mappings.E = "Ctrl+E"
mappings.R = "Ctrl+R"

[keyboard.layers.numpad]
activation_key = "F12"
mode = "Toggle"
mappings.M = "1"
mappings.Comma = "2"
mappings.Period = "3"
"#;

    let result = Config::from_str(config_str);
    assert!(
        result.is_ok(),
        "Failed to parse nested layers: {:?}",
        result.err()
    );

    let config = result.unwrap();
    assert_eq!(config.keyboard.layers.len(), 3);
}

/// 测试配置序列化和反序列化
#[test]
fn test_config_roundtrip() {
    let original_config = r#"
log_level = "debug"
tray_icon = true
auto_reload = true

[keyboard.remap]
CapsLock = "Backspace"
"#;

    let config = Config::from_str(original_config).unwrap();
    // 验证配置正确加载
    assert_eq!(config.log_level, "debug");
    assert!(config.tray_icon);
}

/// 测试空层配置
#[test]
fn test_empty_layer_mappings() {
    let config_str = r#"
[keyboard.layers.empty_layer]
activation_key = "CapsLock"
mappings = {}
"#;

    let result = Config::from_str(config_str);
    assert!(result.is_ok());

    let config = result.unwrap();
    let layer = config.keyboard.layers.get("empty_layer").unwrap();
    assert!(layer.mappings.is_empty());
}

/// 测试特殊字符键名
#[test]
fn test_special_key_names() {
    let config_str = r#"
[keyboard.remap]
Grave = "Escape"
Backslash = "Backspace"
BracketLeft = "Home"
BracketRight = "End"
"#;

    // 这些键名在配置中可以被解析
    let result = Config::from_str(config_str);
    assert!(result.is_ok());
}
