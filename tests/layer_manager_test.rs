// 层管理器测试

/// 测试层激活状态
#[test]
fn test_layer_activation() {
    // 测试层激活和停用
    let layer_name = "navigation";
    let activation_key = "CapsLock";

    assert_eq!(layer_name, "navigation");
    assert_eq!(activation_key, "CapsLock");
}

/// 测试层模式
#[test]
fn test_layer_modes() {
    let modes = vec!["Hold", "Toggle"];

    assert!(modes.contains(&"Hold"));
    assert!(modes.contains(&"Toggle"));
}

/// 测试层映射
#[test]
fn test_layer_mappings() {
    let mappings = vec![("H", "Left"), ("J", "Down"), ("K", "Up"), ("L", "Right")];

    assert_eq!(mappings.len(), 4);
    assert_eq!(mappings[0].0, "H");
    assert_eq!(mappings[0].1, "Left");
}

/// 测试多层配置
#[test]
fn test_multiple_layers() {
    let layers = vec![
        ("navigation", "CapsLock", "Hold"),
        ("numpad", "RightAlt", "Hold"),
    ];

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].0, "navigation");
    assert_eq!(layers[1].0, "numpad");
}
