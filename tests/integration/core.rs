//! wakem 集成测试
//!
//! 测试核心功能的端到端行为，确保各模块正确协作

#[cfg(test)]
mod integration_tests {
    use std::path::PathBuf;
    use wakem::config::{parse_key, parse_window_action, wildcard_match, Config};
    use wakem::types::*;

    // ==================== 配置管理集成测试 ====================

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

        // 验证配置加载正确
        assert_eq!(config.log_level, "debug");
        assert!(!config.tray_icon);
        assert!(config.auto_reload);
        assert_eq!(config.keyboard.remap.len(), 2);
        assert_eq!(config.keyboard.layers.len(), 1);
        assert_eq!(config.window.shortcuts.len(), 2);
        assert_eq!(config.launch.len(), 1);
        assert!(config.network.enabled);
        assert_eq!(config.network.instance_id, 1);

        // 序列化回字符串
        let serialized =
            toml::to_string_pretty(&config).expect("Failed to serialize config");

        // 重新解析
        let config2: Config =
            toml::from_str(&serialized).expect("Failed to re-parse config");

        // 验证往返一致性
        assert_eq!(config.log_level, config2.log_level);
        assert_eq!(config.tray_icon, config2.tray_icon);
        assert_eq!(config.auto_reload, config2.auto_reload);
        assert_eq!(config.keyboard.remap.len(), config2.keyboard.remap.len());
        assert_eq!(config.network.instance_id, config2.network.instance_id);
    }

    #[test]
    fn test_config_validation_comprehensive() {
        // 测试有效配置
        let valid_config = r#"
[keyboard.remap]
A = "B"
"#;
        let config: Config = toml::from_str(valid_config).unwrap();
        assert!(config.validate().is_ok());

        // 测试无效日志级别
        let invalid_log_level = r#"
log_level = "invalid"
"#;
        let config: Config = toml::from_str(invalid_log_level).unwrap();
        assert!(config.validate().is_err());

        // 测试无效端口（虽然这里不会触发，因为使用默认值）
        let valid_port_config = r#"
[network]
instance_id = 0
"#;
        let config: Config = toml::from_str(valid_port_config).unwrap();
        assert!(config.validate().is_ok());

        // 测试超出范围的实例 ID
        let invalid_instance = r#"
[network]
instance_id = 256
"#;
        let config: Config = toml::from_str(invalid_instance).unwrap();
        assert!(config.validate().is_err());

        // 测试空层激活键（需要完整的层配置结构）
        let empty_activation_key = r#"
[keyboard.layers.test]
activation_key = ""
mappings = {}
"#;
        let config: Config = toml::from_str(empty_activation_key).unwrap();
        assert!(config.validate().is_err());

        // 测试宏绑定引用不存在的宏
        let invalid_macro_binding = r#"
[macro_bindings]
F5 = "nonexistent_macro"
"#;
        let config: Config = toml::from_str(invalid_macro_binding).unwrap();
        assert!(config.validate().is_err());
    }

    // ==================== 键名映射集成测试 ====================

    #[test]
    fn test_parse_key_consistency() {
        // 测试所有字母键
        for ch in 'a'..='z' {
            let name = ch.to_string();
            let (scan_code, virtual_key) = parse_key(&name)
                .unwrap_or_else(|_| panic!("Failed to parse key: {}", name));

            // 验证扫描码和虚拟键码在合理范围内
            assert!(
                scan_code > 0 && scan_code <= 0xFF,
                "Invalid scan code for {}: {:04X}",
                name,
                scan_code
            );
            assert!(
                virtual_key > 0 && virtual_key <= 0xFF,
                "Invalid virtual key for {}: {:04X}",
                name,
                virtual_key
            );
        }

        // 测试所有数字键
        for ch in '0'..='9' {
            let name = ch.to_string();
            let result = parse_key(&name);
            assert!(result.is_ok(), "Failed to parse digit key: {}", name);
        }

        // 测试特殊键别名一致性
        let capslock_aliases = ["capslock", "caps"];
        let first_result = parse_key(capslock_aliases[0]).unwrap();
        for alias in &capslock_aliases[1..] {
            assert_eq!(
                parse_key(alias).unwrap(),
                first_result,
                "Alias {} should produce same result as capslock",
                alias
            );
        }

        // 测试 Enter 别名
        assert_eq!(parse_key("enter").unwrap(), parse_key("return").unwrap());

        // 测试 Escape 别名
        assert_eq!(parse_key("escape").unwrap(), parse_key("esc").unwrap());
    }

    #[test]
    fn test_parse_key_all_defined_keys() {
        // 确保所有文档中提到的键都能解析
        let defined_keys = [
            "capslock",
            "backspace",
            "enter",
            "escape",
            "space",
            "tab",
            "left",
            "up",
            "right",
            "down",
            "home",
            "end",
            "pageup",
            "pagedown",
            "delete",
            "insert",
            "lshift",
            "rshift",
            "lctrl",
            "rctrl",
            "lalt",
            "ralt",
            "lwin",
            "rwin",
            "a",
            "b",
            "c",
            "z",
            "0",
            "1",
            "9",
            "f1",
            "f2",
            "f12",
        ];

        for key in &defined_keys {
            assert!(
                parse_key(key).is_ok(),
                "Defined key '{}' should be parsable",
                key
            );
        }
    }

    // ==================== 窗口动作集成测试 ====================

    #[test]
    fn test_parse_window_action_comprehensive() {
        // 测试简单动作
        assert!(matches!(
            parse_window_action("Center").unwrap(),
            WindowAction::Center
        ));
        assert!(matches!(
            parse_window_action("Minimize").unwrap(),
            WindowAction::Minimize
        ));
        assert!(matches!(
            parse_window_action("Maximize").unwrap(),
            WindowAction::Maximize
        ));

        // 测试带参数的动作
        assert!(matches!(
            parse_window_action("MoveToEdge(Left)").unwrap(),
            WindowAction::MoveToEdge(Edge::Left)
        ));
        assert!(matches!(
            parse_window_action("HalfScreen(Right)").unwrap(),
            WindowAction::HalfScreen(Edge::Right)
        ));

        // 测试 FixedRatio 动作
        if let WindowAction::FixedRatio { ratio, scale_index } =
            parse_window_action("FixedRatio(1.333, 0)").unwrap()
        {
            assert!((ratio - 1.333).abs() < 0.001);
            assert_eq!(scale_index, 0);
        } else {
            panic!("Expected FixedRatio action");
        }

        // 测试 Move 和 Resize 动作
        if let WindowAction::Move { x, y } =
            parse_window_action("Move(100, 200)").unwrap()
        {
            assert_eq!(x, 100);
            assert_eq!(y, 200);
        } else {
            panic!("Expected Move action");
        }

        if let WindowAction::Resize { width, height } =
            parse_window_action("Resize(1920, 1080)").unwrap()
        {
            assert_eq!(width, 1920);
            assert_eq!(height, 1080);
        } else {
            panic!("Expected Resize action");
        }

        // 测试 ShowNotification 动作
        if let WindowAction::ShowNotification { title, message } =
            parse_window_action("ShowNotification(Test, Hello World!)").unwrap()
        {
            assert_eq!(title, "Test");
            assert_eq!(message, "Hello World!");
        } else {
            panic!("Expected ShowNotification action");
        }
    }

    #[test]
    fn test_parse_window_action_invalid_cases() {
        // 测试完全无效的动作名
        assert!(parse_window_action("InvalidAction").is_err());

        // 测试无效参数
        assert!(parse_window_action("MoveToEdge(InvalidEdge)").is_err());
        assert!(parse_window_action("FixedRatio(not_a_number, 0)").is_err());

        // 注意：某些格式错误可能被容错处理（取决于实现）
        // 以下断言可能需要根据实际实现调整
    }

    // ==================== 通配符匹配集成测试 ====================

    #[test]
    fn test_wildcard_match_real_world_patterns() {
        // 文件扩展名匹配
        assert!(wildcard_match("document.pdf", "*.pdf"));
        assert!(wildcard_match("image.png", "*.png"));
        assert!(wildcard_match("archive.tar.gz", "*.tar.gz"));
        assert!(!wildcard_match("document.txt", "*.pdf"));

        // 进程名匹配（Windows 风格）
        assert!(wildcard_match("chrome.exe", "chrome.exe"));
        assert!(wildcard_match("chrome.exe", "*.exe"));
        assert!(wildcard_match("notepad++.exe", "notepad*.exe"));
        assert!(!wildcard_match("firefox.exe", "chrome*"));

        // 窗口标题模式匹配
        assert!(wildcard_match("Google Chrome - Google Search", "*Chrome*"));
        assert!(wildcard_match("Visual Studio Code", "*Code*"));
        assert!(wildcard_match("Untitled - Notepad", "Untitled*"));

        // 路径风格匹配
        assert!(wildcard_match(
            "C:\\Users\\test\\Documents\\file.txt",
            "C:\\Users\\*\\*.txt"
        ));
        assert!(wildcard_match(
            "/home/user/documents/report.pdf",
            "/home/user/*/*.pdf"
        ));

        // 版本号模式
        assert!(wildcard_match("v1.2.3", "v*.*.*"));
        assert!(wildcard_match("version-2.0.1-beta", "version-*"));
    }

    #[test]
    fn test_wildcard_match_edge_cases() {
        // 空模式和空字符串
        assert!(wildcard_match("", ""));
        assert!(!wildcard_match("a", ""));

        // 只有星号的模式
        assert!(wildcard_match("anything", "*"));
        assert!(wildcard_match("", "*"));
        assert!(wildcard_match("multiple words here", "*"));

        // 连续星号
        assert!(wildcard_match("test", "**"));
        assert!(wildcard_match("test", "***"));

        // 问号精确匹配单个字符
        assert!(wildcard_match("a", "?"));
        assert!(!wildcard_match("", "?"));
        assert!(!wildcard_match("ab", "?"));

        // 特殊字符转义（当前实现不支持转义，但应记录行为）
        assert!(wildcard_match("[test]", "[test]")); // 字面量匹配
        assert!(wildcard_match("file.name", "file.name")); // 点号不是特殊字符
    }

    #[test]
    fn test_wildcard_match_performance_safety() {
        // 测试大输入不会导致问题
        let long_text = "a".repeat(1000);
        let long_pattern = "*".repeat(100);

        // 应该能正常处理（可能返回 false 或 true，但不应该 panic）
        let _ = wildcard_match(&long_text, &long_pattern);

        // 空模式不应崩溃
        assert!(!wildcard_match("anything", ""));
    }

    // ==================== ModifierState 集成测试 ====================

    #[test]
    fn test_modifier_state_from_virtual_key() {
        // 测试修饰键识别
        let shift_result = ModifierState::from_virtual_key(0x10, true); // VK_SHIFT
        assert!(shift_result.is_some());
        let (state, pressed) = shift_result.unwrap();
        assert!(state.shift);
        assert!(pressed);

        let ctrl_result = ModifierState::from_virtual_key(0x11, true); // VK_CONTROL
        assert!(ctrl_result.is_some());
        let (state, _) = ctrl_result.unwrap();
        assert!(state.ctrl);

        let alt_result = ModifierState::from_virtual_key(0x12, true); // VK_MENU (Alt)
        assert!(alt_result.is_some());
        let (state, _) = alt_result.unwrap();
        assert!(state.alt);

        // 测试非修饰键返回 None
        assert!(ModifierState::from_virtual_key(0x41, true).is_none()); // 'A' 键
        assert!(ModifierState::from_virtual_key(0x0D, false).is_none()); // Return 键
    }

    #[test]
    fn test_modifier_state_merge() {
        let mut state = ModifierState::new();

        // 合并 Shift
        state.merge(&ModifierState {
            shift: true,
            ..Default::default()
        });
        assert!(state.shift);
        assert!(!state.ctrl);
        assert!(!state.alt);
        assert!(!state.meta);

        // 合并 Ctrl + Alt（Shift 应该保持）
        state.merge(&ModifierState {
            ctrl: true,
            alt: true,
            ..Default::default()
        });
        assert!(state.shift); // 保持不变
        assert!(state.ctrl);
        assert!(state.alt);
        assert!(!state.meta);
    }

    // ==================== Action 类型集成测试 ====================

    #[test]
    fn test_action_from_input_event() {
        use wakem::types::{KeyEvent, MouseButton, MouseEvent, MouseEventType};

        // 键盘事件
        let key_press = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        if let Action::Key(key_action) = Action::from_input_event(&key_press).unwrap() {
            assert!(matches!(key_action, KeyAction::Press { .. }));
        } else {
            panic!("Expected Key action");
        }

        let key_release = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Released));
        if let Action::Key(key_action) = Action::from_input_event(&key_release).unwrap()
        {
            assert!(matches!(key_action, KeyAction::Release { .. }));
        } else {
            panic!("Expected Key action");
        }

        // 鼠标事件
        let mouse_move = MouseEvent::new(MouseEventType::Move, 100, 200);
        if let Action::Mouse(mouse_action) =
            Action::from_input_event(&InputEvent::Mouse(mouse_move)).unwrap()
        {
            assert!(matches!(
                mouse_action,
                MouseAction::Move { x: 100, y: 200, .. }
            ));
        } else {
            panic!("Expected Mouse action");
        }

        let mouse_button_down =
            MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 50, 50);
        if let Action::Mouse(mouse_action) =
            Action::from_input_event(&InputEvent::Mouse(mouse_button_down)).unwrap()
        {
            assert!(matches!(
                mouse_action,
                MouseAction::ButtonDown {
                    button: MouseButton::Left,
                    ..
                }
            ));
        } else {
            panic!("Expected Mouse action");
        }

        // 滚轮事件
        let wheel = MouseEvent::new(MouseEventType::Wheel(120), 0, 0);
        if let Action::Mouse(mouse_action) =
            Action::from_input_event(&InputEvent::Mouse(wheel)).unwrap()
        {
            assert!(matches!(mouse_action, MouseAction::Wheel { delta: 120 }));
        } else {
            panic!("Expected Wheel action");
        }
    }

    // ==================== 配置路径缓存集成测试 ====================

    #[test]
    fn test_config_path_cache_basic() {
        use wakem::config::{clear_config_path_cache, resolve_config_file_path};

        // 清除缓存以确保干净状态
        clear_config_path_cache();

        // 多次调用同一实例 ID 应该返回相同结果
        let path1 = resolve_config_file_path(None, 0);
        let path2 = resolve_config_file_path(None, 0);
        let path3 = resolve_config_file_path(None, 0);

        // 所有结果应该一致（要么都是 None，要么都指向同一路径）
        match (&path1, &path2, &path3) {
            (None, None, None) => {} // 合法：没有找到配置文件
            (Some(p1), Some(p2), Some(p3)) => {
                assert_eq!(p1, p2);
                assert_eq!(p2, p3);
            }
            _ => {
                panic!("Inconsistent results from cached path resolution");
            }
        }
    }

    #[test]
    fn test_config_path_cache_different_instances() {
        use wakem::config::{clear_config_path_cache, resolve_config_file_path};

        clear_config_path_cache();

        // 不同实例 ID 应该有不同的缓存条目
        let path0 = resolve_config_file_path(None, 0);
        let path1 = resolve_config_file_path(None, 1);
        let path2 = resolve_config_file_path(None, 2);

        // 这些路径应该不同（如果存在的话），或者都是 None
        // 重要的是它们不应该混淆
        if let (Some(p0), Some(p1)) = (&path0, &path1) {
            assert_ne!(p0, p1, "Different instances should have different paths");
        }
    }

    // ==================== 综合场景测试 ====================

    #[test]
    fn test_complete_mapping_workflow() {
        // 模拟完整的键位映射工作流程
        let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"
A = "B"

[keyboard.layers.navigate]
activation_key = "Space"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"
"#;

        let config: Config = toml::from_str(config_str).expect("Failed to parse config");

        // 1. 验证配置
        assert!(config.validate().is_ok(), "Config validation failed");

        // 2. 获取所有规则
        let rules = config.get_all_rules();
        assert!(!rules.is_empty(), "Should have mapping rules");

        // 3. 验证基本映射规则
        assert!(rules.iter().any(|r| {
            matches!(&r.trigger, Trigger::Key { scan_code, .. } if *scan_code == Some(0x3A))
        }), "Should have CapsLock -> Backspace rule");

        // 4. 验证层映射规则
        let layer_rules: Vec<_> = rules.iter()
            .filter(|r| {
                matches!(&r.trigger, Trigger::Key { scan_code, .. } if *scan_code == Some(0x39))
            })
            .collect();

        assert!(
            !layer_rules.is_empty(),
            "Should have layer activation rule for Space"
        );
    }

    #[test]
    fn test_window_preset_matching_workflow() {
        use wakem::config::WindowPreset;

        // 创建窗口预设
        let preset = WindowPreset {
            name: "browser".to_string(),
            process_name: Some("chrome.exe".to_string()),
            executable_path: None,
            title_pattern: Some("*Google*".to_string()),
            x: 100,
            y: 100,
            width: 1920,
            height: 1080,
        };

        // 测试各种匹配场景
        assert!(
            preset.matches("chrome.exe", None, "Google Chrome - Google Search"),
            "Should match Chrome with Google in title"
        );

        assert!(
            !preset.matches("firefox.exe", None, "Google Search - Firefox"),
            "Should not match Firefox even with Google in title"
        );

        assert!(
            !preset.matches(
                "chrome.exe",
                None,
                "Stack Overflow - Where Developers Learn"
            ),
            "Should not match Chrome without Google in title"
        );

        // 测试通配符进程名
        let wildcard_process_preset = WindowPreset {
            name: "any_editor".to_string(),
            process_name: Some("*code*.exe".to_string()),
            executable_path: None,
            title_pattern: None,
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        };

        assert!(
            wildcard_process_preset.matches("vscode.exe", None, "Visual Studio Code"),
            "Wildcard process name should match VS Code"
        );

        assert!(
            wildcard_process_preset.matches(
                "code-insiders.exe",
                None,
                "VS Code Insiders"
            ),
            "Wildcard process name should match Code Insiders"
        );
    }
}
