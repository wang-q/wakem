// 集成测试 - 边界条件和负面测试

use wakem::config::{parse_key, wildcard_match, Config};
use wakem::types::*;

/// 测试空配置加载
#[test]
fn test_empty_config_loading() {
    let config: Config = toml::from_str("").unwrap();
    assert!(config.keyboard.remap.is_empty());
    assert!(config.keyboard.layers.is_empty());
}

/// 测试超长键名
#[test]
fn test_very_long_key_name() {
    let long_name = "A".repeat(10000);
    let result = parse_key(&long_name);
    assert!(result.is_err());
}

/// 测试 Unicode 键名
#[test]
fn test_unicode_and_special_characters() {
    let unicode_names = ["日本語", "中文", "🎉"];
    for name in &unicode_names {
        let result = parse_key(name);
        assert!(result.is_err(), "Unicode 键名 '{}' 应该返回错误", name);
    }
}

/// 测试通配符空字符串
#[test]
fn test_wildcard_empty_string() {
    assert!(wildcard_match("", ""));
    assert!(wildcard_match("", "*"));
    assert!(!wildcard_match("test", ""));
}

/// 测试通配符多个星号
#[test]
fn test_wildcard_multiple_stars() {
    assert!(wildcard_match("test", "**"));
    assert!(wildcard_match("test", "***"));
    assert!(wildcard_match("", "**"));
}

/// 测试空宏
#[test]
fn test_empty_macro_operations() {
    let macro_def = Macro {
        name: "empty".to_string(),
        steps: vec![],
        created_at: None,
        description: None,
    };
    assert_eq!(macro_def.step_count(), 0);
    assert_eq!(macro_def.total_delay(), 0);
}

/// 测试 Action::None 行为
#[test]
fn test_action_none_behavior() {
    let action = Action::None;
    assert!(action.is_none());
}

/// 测试嵌套动作序列
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

/// 测试极端延迟值
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

/// 测试所有修饰键组合
#[test]
fn test_all_modifier_combinations() {
    let mut modifiers = ModifierState::default();
    assert!(modifiers.is_empty());

    modifiers.shift = true;
    assert!(!modifiers.is_empty());

    modifiers.ctrl = true;
    modifiers.alt = true;
    modifiers.meta = true;
    assert!(modifiers.shift && modifiers.ctrl && modifiers.alt && modifiers.meta);
}

/// 测试配置极端数值
#[test]
fn test_config_extreme_values() {
    let config_str = r#"
[mouse.wheel]
speed = 1000000
"#;
    let config: Config = toml::from_str(config_str).unwrap();
    assert_eq!(config.mouse.wheel.speed, 1000000);
}

/// 测试规则启用/禁用
#[test]
fn test_rule_enable_disable_toggle() {
    let rule = MappingRule::new(
        Trigger::key(0x3A, 0x14),
        Action::key(KeyAction::click(0x0E, 0x08)),
    );
    assert!(rule.enabled);
}

/// 测试层模式差异
#[test]
fn test_layer_mode_differences() {
    let hold_layer = Layer::new("hold", 0x3A, 0x14).with_mode(LayerMode::Hold);
    let toggle_layer = Layer::new("toggle", 0x39, 0x20).with_mode(LayerMode::Toggle);

    assert!(matches!(hold_layer.mode, LayerMode::Hold));
    assert!(matches!(toggle_layer.mode, LayerMode::Toggle));
}
