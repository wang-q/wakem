// Integration Tests - Core Functionality End-to-End Tests

use wakem::config::{parse_key, parse_window_action, wildcard_match, Config};
use wakem::types::*;

/// Test config load and save roundtrip
#[test]
fn test_config_load_save_roundtrip() {
    let config_str = r#"
log_level = "debug"
tray_icon = false
auto_reload = true

[keyboard.remap]
CapsLock = "Backspace"
Escape = "CapsLock"

[keyboard.layers.vim]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.vim.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"

[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"Ctrl+Alt+Left" = "HalfScreen(Left)"

[launch]
F1 = "notepad.exe"

[network]
enabled = true
instance_id = 1
"#;

    let config: Config = toml::from_str(config_str).expect("Failed to parse config");

    assert_eq!(config.log_level, "debug");
    assert!(!config.tray_icon);
    assert!(config.auto_reload);
    assert_eq!(config.keyboard.remap.len(), 2);
    assert_eq!(config.keyboard.layers.len(), 1);
    assert_eq!(config.window.shortcuts.len(), 2);
    assert_eq!(config.launch.len(), 1);
    assert!(config.network.enabled);
    assert_eq!(config.network.instance_id, 1);

    let serialized =
        toml::to_string_pretty(&config).expect("Failed to serialize config");
    let config2: Config =
        toml::from_str(&serialized).expect("Failed to re-parse config");

    assert_eq!(config.log_level, config2.log_level);
    assert_eq!(config.tray_icon, config2.tray_icon);
    assert_eq!(config.auto_reload, config2.auto_reload);
}

/// Test config validation
#[test]
fn test_config_validation_comprehensive() {
    let valid_config = r#"
[keyboard.remap]
A = "B"
"#;
    let config: Config = toml::from_str(valid_config).unwrap();
    assert!(config.validate().is_ok());

    let invalid_log_level = r#"
log_level = "invalid"
"#;
    let config: Config = toml::from_str(invalid_log_level).unwrap();
    assert!(config.validate().is_err());

    let invalid_instance = r#"
[network]
instance_id = 256
"#;
    let config: Config = toml::from_str(invalid_instance).unwrap();
    assert!(config.validate().is_err());
}

/// Test key name parsing consistency
#[test]
fn test_parse_key_consistency() {
    for ch in 'a'..='z' {
        let name = ch.to_string();
        let key_info =
            parse_key(&name).unwrap_or_else(|_| panic!("Failed to parse key: {}", name));
        assert!(key_info.scan_code > 0 && key_info.scan_code <= 0xFF);
        assert!(key_info.virtual_key > 0 && key_info.virtual_key <= 0xFF);
    }

    for ch in '0'..='9' {
        let name = ch.to_string();
        let result = parse_key(&name);
        assert!(result.is_ok(), "Failed to parse digit key: {}", name);
    }
}

/// Test window action parsing
#[test]
fn test_parse_window_action_comprehensive() {
    assert!(matches!(
        parse_window_action("Center").unwrap(),
        WindowAction::Center
    ));
    assert!(matches!(
        parse_window_action("MoveToEdge(Left)").unwrap(),
        WindowAction::MoveToEdge(Edge::Left)
    ));
    assert!(matches!(
        parse_window_action("HalfScreen(Right)").unwrap(),
        WindowAction::HalfScreen(Edge::Right)
    ));
}

/// Test wildcard matching
#[test]
fn test_wildcard_match_real_world_patterns() {
    assert!(wildcard_match("document.pdf", "*.pdf"));
    assert!(wildcard_match("chrome.exe", "*.exe"));
    assert!(wildcard_match("Google Chrome - Google Search", "*Chrome*"));
    assert!(!wildcard_match("document.txt", "*.pdf"));
}

/// Test modifier key state
#[test]
fn test_modifier_state_from_internal_vk() {
    let shift_result = ModifierState::from_internal_vk(0x10, true);
    assert!(shift_result.is_some());
    let (state, pressed) = shift_result.unwrap();
    assert!(state.shift);
    assert!(pressed);

    assert!(ModifierState::from_internal_vk(0x41, true).is_none());
}

/// Test action creation from input event
#[test]
fn test_action_from_input_event() {
    let key_press = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
    if let Action::Key(key_action) = Action::from_input_event(&key_press).unwrap() {
        assert!(matches!(key_action, KeyAction::Press { .. }));
    }

    let key_release = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Released));
    if let Action::Key(key_action) = Action::from_input_event(&key_release).unwrap() {
        assert!(matches!(key_action, KeyAction::Release { .. }));
    }
}
