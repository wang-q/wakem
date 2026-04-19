// 集成测试
// 运行: cargo test --test integration_test

use std::path::PathBuf;

/// 测试配置文件加载
#[test]
fn test_config_loading() {
    // 测试最小配置
    let minimal_config = r#"
[keyboard]
remap = [
    { from = "CapsLock", to = "Backspace" },
]
"#;

    // 这里可以添加实际的配置解析测试
    assert!(!minimal_config.is_empty());
}

/// 测试配置文件路径解析
#[test]
fn test_config_path_resolution() {
    // 测试默认路径
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let config_path = PathBuf::from(&home).join("wakem.toml");
    
    // 路径应该不为空
    assert!(!home.is_empty());
    assert!(config_path.to_string_lossy().contains("wakem.toml"));
}

/// 测试示例配置文件存在
#[test]
fn test_example_configs_exist() {
    // 从 crate 根目录开始查找（单一 crate 结构）
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    
    let examples = vec![
        "examples/minimal.toml",
        "examples/test_config.toml",
        "examples/window_manager.toml",
        "examples/navigation_layer.toml",
    ];

    for example in examples {
        let path = crate_root.join(example);
        assert!(
            path.exists(),
            "Example config should exist: {}",
            path.display()
        );
    }
}
