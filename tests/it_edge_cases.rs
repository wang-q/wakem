// Integration Tests - Edge Cases and Negative Tests

use wakem::config::{parse_key, wildcard_match, Config};
use wakem::types::*;

/// Test empty config loading
#[test]
fn test_empty_config_loading() {
    let config: Config = toml::from_str("").unwrap();
    assert!(config.keyboard.remap.is_empty());
    assert!(config.keyboard.layers.is_empty());
}

/// Test very long key name
#[test]
fn test_very_long_key_name() {
    let long_name = "A".repeat(10000);
    let result = parse_key(&long_name);
    assert!(result.is_err());
}

/// Test Unicode key names
#[test]
fn test_unicode_and_special_characters() {
    let unicode_names = ["日本語", "中文", "🎉"];
    for name in &unicode_names {
        let result = parse_key(name);
        assert!(
            result.is_err(),
            "Unicode key名 '{}' should return error",
            name
        );
    }
}

/// Test wildcard empty string
#[test]
fn test_wildcard_empty_string() {
    assert!(wildcard_match("", ""));
    assert!(wildcard_match("", "*"));
    assert!(!wildcard_match("test", ""));
}

/// Test wildcard multiple stars
#[test]
fn test_wildcard_multiple_stars() {
    assert!(wildcard_match("test", "**"));
    assert!(wildcard_match("test", "***"));
    assert!(wildcard_match("", "**"));
}

/// Test empty macro
#[test]
fn test_empty_macro_operations() {
    let macro_def = Macro {
        name: "empty".to_string(),
        steps: vec![],
        created_at: None,
        description: None,
    };
    assert_eq!(macro_def.steps.len(), 0);
}

/// Test nested action sequences
#[test]
fn test_nested_action_sequences() {
    let inner_seq = Action::sequence(vec![
        Action::key(KeyAction::click(0x01, 0x1B)),
        Action::key(KeyAction::click(0x0E, 0x08)),
    ]);

    let outer_seq = Action::sequence(vec![inner_seq, Action::delay(100)]);

    if let Action::Sequence(actions) = outer_seq {
        assert_eq!(actions.len(), 2);
    }
}

/// Test extreme delay values
#[test]
fn test_extreme_delay_values() {
    let zero_delay = Action::delay(0);
    if let Action::Delay { milliseconds } = zero_delay {
        assert_eq!(milliseconds, 0);
    }

    let max_delay = Action::delay(3600000);
    if let Action::Delay { milliseconds } = max_delay {
        assert_eq!(milliseconds, 3600000);
    }
}

/// Test config extreme values
#[test]
fn test_config_extreme_values() {
    let config_str = r#"
[mouse.wheel]
speed = 1000000
"#;
    let config: Config = toml::from_str(config_str).unwrap();
    assert_eq!(config.mouse.wheel.speed, 1000000);
}

/// Test rule enable/disable toggle
#[test]
fn test_rule_enable_disable_toggle() {
    let rule = MappingRule::new(
        Trigger::key(0x3A, 0x14),
        Action::key(KeyAction::click(0x0E, 0x08)),
    );
    assert!(rule.enabled);
}

/// Test layer mode differences
#[test]
fn test_layer_mode_differences() {
    let hold_layer = Layer::new("hold", 0x3A, 0x14).with_mode(LayerMode::Hold);
    let toggle_layer = Layer::new("toggle", 0x39, 0x20).with_mode(LayerMode::Toggle);

    assert!(matches!(hold_layer.mode, LayerMode::Hold));
    assert!(matches!(toggle_layer.mode, LayerMode::Toggle));
}
