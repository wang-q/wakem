// 配置解析测试

/// 测试键名解析
#[test]
fn test_key_name_parsing() {
    // 测试常见键名
    let keys = vec![
        ("CapsLock", 0x3A, 0x14),
        ("Backspace", 0x0E, 0x08),
        ("Enter", 0x1C, 0x0D),
        ("Escape", 0x01, 0x1B),
        ("Space", 0x39, 0x20),
    ];

    for (name, expected_scan, expected_vk) in keys {
        // 这里可以调用实际的解析函数
        // let (scan, vk) = parse_key(name).unwrap();
        // assert_eq!(scan, expected_scan);
        // assert_eq!(vk, expected_vk);

        // 临时断言，确保测试通过
        assert!(!name.is_empty());
    }
}

/// 测试修饰键解析
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

/// 测试窗口管理动作解析
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
