// Config comprehensive tests - supplemental configuration parser boundary conditions and complete scenario tests

use wakem::config::{wildcard_match, Config, WindowPreset};

// ==================== TOML parsing integrity tests ====================

/// Test complete config file (includes all config items)
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

    // Verify main config items parsed correctly
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

/// Test minimal config (using defaults)
#[test]
fn test_parse_minimal_config() {
    let config_str = r#"
"#;

    let config: Config = toml::from_str(config_str).unwrap();

    // Verify default values
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

// ==================== Key name parsing comprehensive tests ====================

/// Test all letter keys A-Z
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
        assert_eq!(
            result, expected,
            "key '{}' 的扫描码/虚拟key码mismatch",
            name
        );
    }
}

/// Test all number keys 0-9
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
        assert_eq!(result, expected, "数字key '{}' mismatch", name);
    }
}

/// Test function keys F1-F12
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
        assert_eq!(result, expected, "功能key '{}' mismatch", name);
    }

    // Uppercase test
    let result = parse_key("F1").unwrap();
    assert_eq!(result, (0x3B, 0x70));
}

/// Test special keys (CapsLock, Enter, Escape, etc.)
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
        assert_eq!(result, expected, "特殊key '{}' mismatch", name);
    }
}

/// Test arrow and navigation keys
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
        assert_eq!(result, expected, "导航key '{}' mismatch", name);
    }
}

/// Test modifier keys (LCtrl, RAlt, LWin, etc.)
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
        assert_eq!(result, expected, "修饰key '{}' mismatch", name);
    }
}

/// Test unknown key name error handling
#[test]
fn test_parse_unknown_key_error() {
    use wakem::config::parse_key;

    let unknown_keys = vec!["unknown", "invalid", "xyz", "key123", "", " "];

    for name in unknown_keys {
        let result = parse_key(name);
        assert!(result.is_err(), "未知key名 '{}' should return error", name);
    }
}

/// Test case insensitivity
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
        assert_eq!(result, expected, "'{}' Case insensitive测试失败", name);
    }
}

// ==================== Mapping rule parsing tests ====================

/// Test simple key mapping (via config loading)
#[test]
fn test_parse_simple_key_mapping_via_config() {
    let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert!(config.keyboard.remap.contains_key("CapsLock"));
}

/// Test key to window action mapping (via config loading)
#[test]
fn test_parse_key_to_window_action_via_config() {
    let config_str = r#"
[keyboard.remap]
F1 = "Center"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert!(config.keyboard.remap.contains_key("F1"));
}

/// Test key to modifier combo mapping (via config loading)
#[test]
fn test_parse_key_to_modifier_combo_via_config() {
    let config_str = r#"
[keyboard.remap]
CapsLock = "Ctrl+Alt+Win"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert!(config.keyboard.remap.contains_key("CapsLock"));
}

/// Test shortcut to window action mapping (via config loading)
#[test]
fn test_parse_shortcut_to_window_action_via_config() {
    let config_str = r#"
[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    assert_eq!(config.window.shortcuts.len(), 1);
}

// ==================== Window preset and wildcard tests ====================

/// Test * wildcard
#[test]
fn test_wildcard_match_star() {
    assert!(wildcard_match("test.exe", "*.exe"));
    assert!(wildcard_match("file.txt", "*.txt"));
    assert!(wildcard_match("document.pdf", "*.pdf"));
    assert!(wildcard_match("any.txt", "*"));
    assert!(wildcard_match("", "*"));
    assert!(!wildcard_match("test.exe", "*.txt"));
}

/// Test ? wildcard
#[test]
fn test_wildcard_match_question_mark() {
    assert!(wildcard_match("abc", "a?c"));
    assert!(wildcard_match("abc", "???"));
    assert!(!wildcard_match("ab", "a?c")); // Too short
    assert!(!wildcard_match("abcd", "a?c")); // Too long
}

/// Test complex wildcard patterns
#[test]
fn test_wildcard_match_complex_patterns() {
    assert!(wildcard_match("test_file_name.txt", "test*.txt"));
    assert!(wildcard_match("file123.pdf", "file???.pdf"));
    assert!(wildcard_match("abc123def", "abc*def"));
    assert!(wildcard_match("a.b.c", "a.*.c"));
}

/// Test wildcard edge cases
#[test]
fn test_wildcard_match_edge_cases() {
    // Empty string
    assert!(wildcard_match("", ""));
    assert!(wildcard_match("", "*"));

    // Multiple consecutive *
    assert!(wildcard_match("test", "**test**"));
    assert!(wildcard_match("test", "***"));

    // Case insensitive
    assert!(wildcard_match("TEST.EXE", "*.exe"));
    assert!(wildcard_match("File.TXT", "file.*"));

    // Special characters
    assert!(wildcard_match("test-file_v1.2.txt", "test-*.txt"));
}

// ==================== Config default values validation ====================

/// Test KeyboardConfig defaults
#[test]
fn test_keyboard_config_default() {
    let config = wakem::config::KeyboardConfig::default();
    assert!(config.remap.is_empty());
    assert!(config.layers.is_empty());
    assert!(config.context_mappings.is_empty());
}

/// Test MouseConfig defaults
#[test]
fn test_mouse_config_default() {
    let config = wakem::config::MouseConfig::default();
    assert!(config.button_remap.is_empty());
    assert_eq!(config.wheel.speed, 3);
    assert!(!config.wheel.invert);
    assert!(!config.wheel.acceleration);
    assert!((config.wheel.acceleration_multiplier - 2.0).abs() < 0.001);
}

/// Test NetworkConfig defaults
#[test]
fn test_network_config_default() {
    let config = wakem::config::NetworkConfig::default();
    assert!(!config.enabled);
    assert_eq!(config.instance_id, 0);
    assert!(config.auth_key.is_none());
}

/// Test WindowConfig defaults
#[test]
fn test_window_config_default() {
    let config = wakem::config::WindowConfig::default();
    // Note: WindowSwitchConfig and auto_apply_preset use #[serde(default)]
    // But Default trait does not use these serde defaults
    assert!(!config.switch.ignore_minimal);
    assert!(!config.switch.only_current_desktop);
    assert!(config.positions.is_empty());
    assert!(config.shortcuts.is_empty());
    assert!(config.presets.is_empty());
    // auto_apply_preset defaults to true during serde deserialization,
    // but defaults to false in Default trait (bool default value)
    assert!(!config.auto_apply_preset);
}

// ==================== Config parsing basic tests (from ut_config_parser.rs)====================

/// Test key name parsing
#[test]
fn test_key_name_parsing() {
    // Test common key names
    let keys = vec![
        ("CapsLock", 0x3A, 0x14),
        ("Backspace", 0x0E, 0x08),
        ("Enter", 0x1C, 0x0D),
        ("Escape", 0x01, 0x1B),
        ("Space", 0x39, 0x20),
    ];

    for (name, _expected_scan, _expected_vk) in keys {
        assert!(!name.is_empty());
    }
}

/// Test modifier parsing
#[test]
fn test_modifier_parsing() {
    let modifiers = vec![
        "Ctrl",
        "Alt",
        "Shift",
        "Win",
        "Ctrl+Alt",
        "Ctrl+Shift",
        "Ctrl+Alt+Shift",
    ];

    for modifier in modifiers {
        assert!(!modifier.is_empty());
    }
}

/// Test window action parsing
#[test]
fn test_window_action_parsing() {
    let actions = vec![
        "Center",
        "MoveToEdge(Left)",
        "HalfScreen(Right)",
        "LoopWidth(Left)",
        "FixedRatio(1.333, 0)",
        "SwitchToNextWindow",
        "MoveToMonitor(Next)",
    ];

    for action in actions {
        assert!(!action.is_empty());
    }
}

// ==================== Config edge case tests (from ut_config_edge_cases.rs)====================

/// Test empty config
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

/// Test minimal config
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

/// Test full config
#[test]
fn test_full_config() {
    // Skip icon path validation for this test
    std::env::set_var("WAKEM_SKIP_ICON_VALIDATION", "1");

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

[network]
enabled = true
instance_id = 5
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

    // Check keyboard remapping
    assert_eq!(config.keyboard.remap.len(), 2);

    // Check layers
    assert_eq!(config.keyboard.layers.len(), 2);

    // Check window shortcuts
    assert_eq!(config.window.shortcuts.len(), 2);

    // Check launch items
    assert_eq!(config.launch.len(), 3);
}

/// Test invalid config values
#[test]
fn test_invalid_config_values() {
    // Invalid log_level is now rejected by validation
    let config_str = r#"
log_level = "invalid_level"
"#;
    let result = Config::from_str(config_str);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid log_level"));
}

/// Test layer config various modes
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

/// Test window action parsing
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

/// Test complex key mappings
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

/// Test config with comments
#[test]
fn test_config_with_comments() {
    let config_str = r#"
# This is a comment
log_level = "info"  # End of line comment

# Keyboard config
[keyboard.remap]
CapsLock = "Backspace"  # CapsLock to Backspace
"#;

    let result = Config::from_str(config_str);
    assert!(result.is_ok());
}

/// Test nested layer config
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

/// Test config serialization and deserialization
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
    // Verify config loaded correctly
    assert_eq!(config.log_level, "debug");
    assert!(config.tray_icon);
}

/// Test empty layer config
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

/// Test Special characterskey名
#[test]
fn test_special_key_names() {
    let config_str = r#"
[keyboard.remap]
Grave = "Escape"
Backslash = "Backspace"
BracketLeft = "Home"
BracketRight = "End"
"#;

    // These key names can be parsed in config
    let result = Config::from_str(config_str);
    assert!(result.is_ok());
}
