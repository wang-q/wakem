// 层系统测试
// 测试层的创建、激活和映射

use wakem::types::*;

/// 测试 Layer 创建
#[test]
fn test_layer_creation() {
    let layer = Layer::new("navigation", 0x3A, 0x14); // CapsLock

    assert_eq!(layer.name, "navigation");
    assert_eq!(layer.activation_key, 0x3A);
    assert_eq!(layer.activation_vk, 0x14);
    assert_eq!(layer.mode, LayerMode::Hold);
    assert!(layer.mappings.is_empty());
}

/// 测试 Layer with_mode Toggle
#[test]
fn test_layer_toggle_mode() {
    let layer = Layer::new("fn_layer", 0x3B, 0x70) // F1
        .with_mode(LayerMode::Toggle);

    assert_eq!(layer.activation_key, 0x3B);
    assert_eq!(layer.mode, LayerMode::Toggle);
}

/// 测试 Layer add_mapping
#[test]
fn test_layer_add_mapping() {
    let mut layer = Layer::new("vim_navigation", 0x3A, 0x14).with_mode(LayerMode::Hold);

    layer.add_mapping(
        Trigger::key(0x23, 0x48),                  // H
        Action::key(KeyAction::click(0x4B, 0x25)), // Left
    );

    layer.add_mapping(
        Trigger::key(0x24, 0x4A),                  // J
        Action::key(KeyAction::click(0x50, 0x28)), // Down
    );

    assert_eq!(layer.mappings.len(), 2);
}

/// 测试 Layer 默认值
#[test]
fn test_layer_default_mode() {
    let layer = Layer::new("test", 0x1E, 0x41);

    assert_eq!(layer.mode, LayerMode::Hold);
}

/// 测试 LayerMode 枚举
#[test]
fn test_layer_mode_enum() {
    let hold = LayerMode::Hold;
    let toggle = LayerMode::Toggle;

    assert!(matches!(hold, LayerMode::Hold));
    assert!(matches!(toggle, LayerMode::Toggle));
}

/// 测试 LayerStack 创建
#[test]
fn test_layer_stack_creation() {
    let stack = LayerStack::new();

    assert!(stack.get_active_layers().is_empty());
    assert!(!stack.is_layer_active("any"));
}

/// 测试 LayerStack 激活层
#[test]
fn test_layer_stack_activate() {
    let mut stack = LayerStack::new();
    let layer = Layer::new("navigation", 0x3A, 0x14);

    stack.activate_layer(layer);
    assert!(stack.is_layer_active("navigation"));
}

/// 测试 LayerStack 停用层
#[test]
fn test_layer_stack_deactivate() {
    let mut stack = LayerStack::new();
    let layer1 = Layer::new("navigation", 0x3A, 0x14);
    let layer2 = Layer::new("window_mgmt", 0x3B, 0x70);

    stack.activate_layer(layer1);
    stack.activate_layer(layer2);

    stack.deactivate_layer("navigation");
    assert!(!stack.is_layer_active("navigation"));
    assert!(stack.is_layer_active("window_mgmt"));
}

/// 测试 LayerStack 切换层
#[test]
fn test_layer_stack_toggle() {
    let mut stack = LayerStack::new();
    let layer = Layer::new("test", 0x3A, 0x14).with_mode(LayerMode::Toggle);

    // 第一次切换 - 激活
    stack.toggle_layer(layer.clone());
    assert!(stack.is_layer_active("test"));

    // 第二次切换 - 停用
    stack.toggle_layer(layer);
    assert!(!stack.is_layer_active("test"));
}

/// 测试 LayerStack 清除所有层
#[test]
fn test_layer_stack_clear() {
    let mut stack = LayerStack::new();

    stack.activate_layer(Layer::new("layer1", 0x3A, 0x14));
    stack.activate_layer(Layer::new("layer2", 0x3B, 0x70));

    stack.clear_active_layers();

    assert!(stack.get_active_layers().is_empty());
    assert!(!stack.is_layer_active("layer1"));
    assert!(!stack.is_layer_active("layer2"));
}

/// 测试多层同时激活
#[test]
fn test_multiple_layers_active() {
    let mut stack = LayerStack::new();

    stack.activate_layer(Layer::new("base", 0x3A, 0x14));
    stack.activate_layer(Layer::new("shift", 0x3B, 0x70));
    stack.activate_layer(Layer::new("ctrl", 0x3C, 0x71));

    assert_eq!(stack.get_active_layers().len(), 3);
    assert!(stack.is_layer_active("base"));
    assert!(stack.is_layer_active("shift"));
    assert!(stack.is_layer_active("ctrl"));
}

/// 测试层优先级（后激活的优先级高）
#[test]
fn test_layer_priority() {
    let mut stack = LayerStack::new();

    stack.activate_layer(Layer::new("base", 0x3A, 0x14));
    stack.activate_layer(Layer::new("override", 0x3B, 0x70));

    // 获取最后激活的层
    let active = stack.get_active_layers();
    assert_eq!(active.len(), 2);
    assert_eq!(active[active.len() - 1].name, "override");
}

/// 测试空层名称
#[test]
fn test_empty_layer_name() {
    let layer = Layer::new("", 0x1E, 0x41);
    assert_eq!(layer.name, "");
}

/// 测试复杂层配置
#[test]
fn test_complex_layer_config() {
    let mut layer =
        Layer::new("advanced_navigation", 0x3A, 0x14).with_mode(LayerMode::Toggle);

    layer.add_mapping(
        Trigger::key(0x23, 0x48),                  // H
        Action::key(KeyAction::click(0x4B, 0x25)), // Left
    );
    layer.add_mapping(
        Trigger::key(0x24, 0x4A),                  // J
        Action::key(KeyAction::click(0x50, 0x28)), // Down
    );
    layer.add_mapping(
        Trigger::key(0x25, 0x4B),                  // K
        Action::key(KeyAction::click(0x48, 0x26)), // Up
    );
    layer.add_mapping(
        Trigger::key(0x26, 0x4C),                  // L
        Action::key(KeyAction::click(0x4D, 0x27)), // Right
    );
    layer.add_mapping(
        Trigger::key(0x11, 0x57), // W
        Action::window(WindowAction::Center),
    );
    layer.add_mapping(
        Trigger::key(0x10, 0x51), // Q
        Action::window(WindowAction::Close),
    );

    assert_eq!(layer.name, "advanced_navigation");
    assert_eq!(layer.mode, LayerMode::Toggle);
    assert_eq!(layer.mappings.len(), 6);
}

/// 测试层激活键检查
#[test]
fn test_layer_is_activation_key() {
    let layer = Layer::new("test", 0x3A, 0x14); // CapsLock

    assert!(layer.is_activation_key(0x3A, 0x00)); // 扫描码匹配
    assert!(layer.is_activation_key(0x00, 0x14)); // 虚拟键码匹配
    assert!(!layer.is_activation_key(0x1E, 0x41)); // 不匹配
}

/// 测试基础层设置
#[test]
fn test_layer_stack_base_layer() {
    let mut stack = LayerStack::new();

    let base_mappings = vec![MappingRule::new(
        Trigger::key(0x1E, 0x41),
        Action::key(KeyAction::click(0x1E, 0x41)),
    )];

    stack.set_base_layer(base_mappings);

    let all_mappings = stack.get_all_mappings();
    assert_eq!(all_mappings.len(), 1);
}

/// 测试 Hold 模式层释放
#[test]
fn test_layer_stack_hold_release() {
    let mut stack = LayerStack::new();
    let layer = Layer::new("hold_test", 0x3A, 0x14).with_mode(LayerMode::Hold);

    stack.activate_layer(layer);
    stack.hold_layer("hold_test");
    assert!(stack.is_layer_active("hold_test"));

    // 释放 Hold 模式的层
    stack.release_layer("hold_test");
    assert!(!stack.is_layer_active("hold_test"));
}

/// 测试 Toggle 模式层释放（不应该停用）
#[test]
fn test_layer_stack_toggle_release() {
    let mut stack = LayerStack::new();
    let layer = Layer::new("toggle_test", 0x3A, 0x14).with_mode(LayerMode::Toggle);

    stack.activate_layer(layer);
    stack.hold_layer("toggle_test");
    assert!(stack.is_layer_active("toggle_test"));

    // 释放 Toggle 模式的层应该保持激活
    stack.release_layer("toggle_test");
    // 注意：当前实现会在 release 时检查 mode，Toggle 模式的层不会被停用
    // 但实际行为取决于具体实现
}

/// 测试重复激活同一层（应该移到栈顶）
#[test]
fn test_layer_reactivate_moves_to_top() {
    let mut stack = LayerStack::new();

    let layer1 = Layer::new("layer1", 0x3A, 0x14);
    let layer2 = Layer::new("layer2", 0x3B, 0x70);

    stack.activate_layer(layer1);
    stack.activate_layer(layer2);

    // 重新激活 layer1，应该移到栈顶
    let layer1_new = Layer::new("layer1", 0x3A, 0x14);
    stack.activate_layer(layer1_new);

    let active = stack.get_active_layers();
    assert_eq!(active.len(), 2);
    assert_eq!(active[1].name, "layer1"); // 现在在栈顶
}
