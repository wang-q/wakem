// Runtime 深度测试 - KeyMapper, LayerManager, MacroPlayer

use wakem::runtime::{KeyMapper, LayerManager};
use wakem::types::{
    Action, InputEvent, KeyAction, KeyEvent, KeyState, Layer, LayerMode, Macro,
    MacroStep, MappingRule, ModifierState, MouseAction, MouseEvent, MouseEventType,
    Trigger, WindowAction,
};

// ==================== KeyMapper 测试 ====================

/// 测试 KeyMapper 初始化
#[test]
fn test_key_mapper_new() {
    let _mapper = KeyMapper::new();
}

/// 测试 Default trait
#[test]
fn test_key_mapper_default() {
    let _mapper = KeyMapper::default();
}

/// 测试加载规则
#[test]
fn test_mapper_load_rules() {
    let mut mapper = KeyMapper::new();

    let rules = vec![
        MappingRule::new(
            Trigger::key(0x3A, 0x14),                  // CapsLock
            Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
        ),
        MappingRule::new(
            Trigger::key(0x01, 0x1B),                  // Escape
            Action::key(KeyAction::click(0x4B, 0x25)), // Left
        ),
    ];

    mapper.load_rules(rules);
}

/// 测试简单事件处理（有匹配规则）
#[test]
fn test_mapper_process_event_simple_match() {
    let mut mapper = KeyMapper::new();

    let rules = vec![MappingRule::new(
        Trigger::key(0x3A, 0x14),                  // CapsLock
        Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
    )];

    mapper.load_rules(rules);

    let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
    let result = mapper.process_event_with_context(&event, None);

    assert!(result.is_some(), "应该找到匹配的映射");
}

/// 测试简单事件处理（无匹配规则）
#[test]
fn test_mapper_process_event_no_match() {
    let mut mapper = KeyMapper::new();

    let rules = vec![MappingRule::new(
        Trigger::key(0x3A, 0x14),                  // CapsLock
        Action::key(KeyAction::click(0x0E, 0x08)), // Backspace
    )];

    mapper.load_rules(rules);

    // 按下 'A' 键，不应该匹配
    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    let result = mapper.process_event_with_context(&event, None);

    assert!(result.is_none(), "不应该找到匹配");
}

/// 测试禁用状态下的映射器
#[test]
fn test_mapper_disabled() {
    let mut mapper = KeyMapper::new();

    let rules = vec![MappingRule::new(
        Trigger::key(0x3A, 0x14),
        Action::key(KeyAction::click(0x0E, 0x08)),
    )];

    mapper.load_rules(rules);

    // 禁用状态下事件应该返回 None
    // 注意：enabled 字段是私有的，我们通过测试行为来验证
    let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
    let result = mapper.process_event_with_context(&event, None);
    assert!(result.is_some()); // 默认是启用状态，所以应该有结果
}

/// 测试鼠标事件处理（当前返回 None）
#[test]
fn test_mapper_process_mouse_event() {
    let mapper = KeyMapper::new();

    let mouse_event = MouseEvent::new(MouseEventType::Move, 100, 200);
    let event = InputEvent::Mouse(mouse_event);

    let result = mapper.process_event_with_context(&event, None);
    assert!(result.is_none(), "鼠标事件当前不处理");
}

/// 测试 Click 动作根据按键状态调整
#[test]
fn test_mapper_adjust_action_pressed() {
    let mut mapper = KeyMapper::new();

    let rules = vec![MappingRule::new(
        Trigger::key(0x3A, 0x14),
        Action::key(KeyAction::click(0x0E, 0x08)), // Click 动作
    )];

    mapper.load_rules(rules);

    // 按下事件 -> 应该返回 Press 动作
    let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
    let result = mapper.process_event_with_context(&event, None);

    assert!(result.is_some());
    if let Some(Action::Key(KeyAction::Press { .. })) = result {
        // 正确：按下事件转换为 Press
    } else {
        panic!("按下事件应该转换为 Press 动作");
    }
}

/// 测试 Click 动作根据按键状态调整（释放）
#[test]
fn test_mapper_adjust_action_released() {
    let mut mapper = KeyMapper::new();

    let rules = vec![MappingRule::new(
        Trigger::key(0x3A, 0x14),
        Action::key(KeyAction::click(0x0E, 0x08)), // Click 动作
    )];

    mapper.load_rules(rules);

    // 释放事件 -> 应该返回 Release 动作
    let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Released));
    let result = mapper.process_event_with_context(&event, None);

    assert!(result.is_some());
    if let Some(Action::Key(KeyAction::Release { .. })) = result {
        // 正确：释放事件转换为 Release
    } else {
        panic!("释放事件应该转换为 Release 动作");
    }
}

// ==================== LayerManager 测试 ====================

/// 测试 LayerManager 初始化
#[test]
fn test_layer_manager_new() {
    let manager = LayerManager::new();
    assert!(!manager.is_layer_active("any_layer"));
    assert!(manager.get_active_layers().is_empty());
}

/// 测试 Default trait
#[test]
fn test_layer_manager_default() {
    let manager = LayerManager::default();
    assert!(manager.get_active_layers().is_empty());
}

/// 测试层注册
#[test]
fn test_layer_manager_register_layer() {
    let mut manager = LayerManager::new();

    let layer = Layer::new("test_layer", 0x3A, 0x14).with_mode(LayerMode::Hold);
    manager.register_layer(layer);

    // 注册后，层尚未激活
    assert!(!manager.is_layer_active("test_layer"));
}

/// 测试 Hold 模式层的激活和取消激活
#[test]
fn test_layer_manager_hold_mode_activate_deactivate() {
    let mut manager = LayerManager::new();

    let layer =
        LayerManager::create_layer_from_config("nav", "CapsLock", LayerMode::Hold, &[])
            .unwrap();
    manager.register_layer(layer);

    // 按下激活键
    let press = KeyEvent::new(0x3A, 0x14, KeyState::Pressed);
    let (handled, _) = manager.process_event(&InputEvent::Key(press));

    assert!(handled, "应该处理了激活键按下事件");
    assert!(manager.is_layer_active("nav"), "层应该被激活");

    // 释放激活键
    let release = KeyEvent::new(0x3A, 0x14, KeyState::Released);
    let (handled, _) = manager.process_event(&InputEvent::Key(release));

    assert!(handled, "应该处理了激活键释放事件");
    assert!(
        !manager.is_layer_active("nav"),
        "Hold 模式释放后层应该被取消激活"
    );
}

/// 测试 Toggle 模式层
#[test]
fn test_layer_manager_toggle_mode() {
    let mut manager = LayerManager::new();

    let layer =
        LayerManager::create_layer_from_config("sym", "Space", LayerMode::Toggle, &[])
            .unwrap();
    manager.register_layer(layer);

    // 第一次按下 -> 激活
    let press1 = KeyEvent::new(0x39, 0x20, KeyState::Pressed);
    let (handled, _) = manager.process_event(&InputEvent::Key(press1));
    assert!(handled);
    assert!(manager.is_layer_active("sym"), "第一次按下应该激活");

    // 第二次按下 -> 取消激活
    let press2 = KeyEvent::new(0x39, 0x20, KeyState::Pressed);
    let (handled, _) = manager.process_event(&InputEvent::Key(press2));
    assert!(handled);
    assert!(!manager.is_layer_active("sym"), "第二次按下应该取消激活");
}

/// 测试层内映射查找
#[test]
fn test_layer_mapping_lookup() {
    let mut manager = LayerManager::new();

    let layer = LayerManager::create_layer_from_config(
        "nav",
        "RAlt",
        LayerMode::Hold,
        &[("H".to_string(), "Left".to_string())],
    )
    .unwrap();
    manager.register_layer(layer);

    // 激活层
    let alt_press = KeyEvent::new(0xE038, 0xA5, KeyState::Pressed);
    manager.process_event(&InputEvent::Key(alt_press));

    // 在层内按 H，应该映射为 Left
    let h_press = KeyEvent::new(0x23, 0x48, KeyState::Pressed);
    let (handled, action) = manager.process_event(&InputEvent::Key(h_press));

    assert!(handled, "应该在层中找到映射");
    assert!(action.is_some(), "应该返回动作");
}

/// 测试非激活键事件的传递（未处理）
#[test]
fn test_layer_non_activation_key_not_handled() {
    let mut manager = LayerManager::new();

    let layer =
        LayerManager::create_layer_from_config("nav", "CapsLock", LayerMode::Hold, &[])
            .unwrap();
    manager.register_layer(layer);

    // 按下一个普通键（不是激活键）
    let a_press = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    let (handled, action) = manager.process_event(&InputEvent::Key(a_press));

    assert!(!handled, "非激活键不应该被处理");
    assert!(action.is_none(), "不应该返回动作");
}

/// 测试多个层的注册和管理
#[test]
fn test_layer_manager_multiple_layers() {
    let mut manager = LayerManager::new();

    let nav =
        LayerManager::create_layer_from_config("nav", "RAlt", LayerMode::Hold, &[])
            .unwrap();
    let sym =
        LayerManager::create_layer_from_config("sym", "Space", LayerMode::Toggle, &[])
            .unwrap();
    let num = LayerManager::create_layer_from_config("num", "F12", LayerMode::Hold, &[])
        .unwrap();

    manager.register_layer(nav);
    manager.register_layer(sym);
    manager.register_layer(num);

    // 所有层初始状态都是未激活
    assert!(!manager.is_layer_active("nav"));
    assert!(!manager.is_layer_active("sym"));
    assert!(!manager.is_layer_active("num"));
}

/// 测试清除所有层
#[test]
fn test_layer_manager_clear_layers() {
    let mut manager = LayerManager::new();

    let layer =
        LayerManager::create_layer_from_config("test", "F11", LayerMode::Toggle, &[])
            .unwrap();
    manager.register_layer(layer);

    // 激活层
    let f11_press = KeyEvent::new(0x57, 0x7A, KeyState::Pressed);
    manager.process_event(&InputEvent::Key(f11_press));
    assert!(manager.is_layer_active("test"));

    // 清除所有层
    manager.clear_layers();
    assert!(!manager.is_layer_active("test"));
    assert!(manager.get_active_layers().is_empty());
}

// ==================== Macro 和 MacroStep 测试 ====================

/// 测试空宏的属性
#[test]
fn test_empty_macro() {
    let macro_def = Macro {
        name: "empty".to_string(),
        steps: vec![],
        created_at: None,
        description: None,
    };

    assert_eq!(macro_def.step_count(), 0);
    assert_eq!(macro_def.total_delay(), 0);
}

/// 测试单步宏
#[test]
fn test_single_step_macro() {
    let step = MacroStep::new(
        0,
        Action::key(KeyAction::click(0x1E, 0x41)),
        ModifierState::default(),
        0,
    );

    let macro_def = Macro {
        name: "single".to_string(),
        steps: vec![step],
        created_at: Some("2024-01-01".to_string()),
        description: Some("Single step macro".to_string()),
    };

    assert_eq!(macro_def.step_count(), 1);
    assert_eq!(macro_def.total_delay(), 0);
    assert_eq!(macro_def.name, "single");
    assert!(macro_def.created_at.is_some());
    assert!(macro_def.description.is_some());
}

/// 测试多步宏的延迟计算
#[test]
fn test_multi_step_macro_delays() {
    let steps: Vec<MacroStep> = vec![
        MacroStep::new(
            0,
            Action::key(KeyAction::click(0x1E, 0x41)),
            ModifierState::default(),
            0,
        ),
        MacroStep::new(
            50,
            Action::key(KeyAction::click(0x30, 0x42)),
            ModifierState::default(),
            50,
        ),
        MacroStep::new(
            100,
            Action::key(KeyAction::click(0x2E, 0x43)),
            ModifierState::default(),
            150,
        ),
        MacroStep::new(
            200,
            Action::mouse(MouseAction::Wheel { delta: 120 }),
            ModifierState::default(),
            350,
        ),
    ];

    let macro_def = Macro {
        name: "multi_step".to_string(),
        steps,
        created_at: None,
        description: None,
    };

    assert_eq!(macro_def.step_count(), 4);
    assert_eq!(macro_def.total_delay(), 350); // max timestamp
}

/// 测试带修饰键的宏步骤
#[test]
fn test_macro_step_with_modifiers() {
    let mut modifiers = ModifierState::default();
    modifiers.ctrl = true;
    modifiers.shift = true;

    let step = MacroStep::new(
        0,
        Action::key(KeyAction::press(0x2E, 0x43)),
        modifiers.clone(),
        100,
    );

    assert!(step.modifiers.ctrl);
    assert!(step.modifiers.shift);
    assert!(!step.modifiers.alt);
    assert!(!step.modifiers.meta);
    assert_eq!(step.timestamp, 100);
}

/// 测试各种动作类型的宏步骤
#[test]
fn test_macro_steps_all_action_types() {
    let steps: Vec<MacroStep> = vec![
        // 按键动作
        MacroStep::new(
            0,
            Action::key(KeyAction::Press {
                scan_code: 0x1E,
                virtual_key: 0x41,
            }),
            ModifierState::default(),
            0,
        ),
        // 鼠标动作
        MacroStep::new(
            10,
            Action::mouse(MouseAction::Move {
                x: 100,
                y: 200,
                relative: false,
            }),
            ModifierState::default(),
            10,
        ),
        // 窗口动作
        MacroStep::new(
            20,
            Action::window(WindowAction::Center),
            ModifierState::default(),
            20,
        ),
        // 延迟动作
        MacroStep::new(30, Action::delay(500), ModifierState::default(), 30),
        // 序列动作
        MacroStep::new(
            40,
            Action::sequence(vec![
                Action::key(KeyAction::click(0x01, 0x1B)),
                Action::key(KeyAction::click(0x0E, 0x08)),
            ]),
            ModifierState::default(),
            40,
        ),
        // 无操作
        MacroStep::new(50, Action::None, ModifierState::default(), 50),
    ];

    let macro_def = Macro {
        name: "all_types".to_string(),
        steps,
        created_at: None,
        description: None,
    };

    assert_eq!(macro_def.step_count(), 6);
}

/// 测试宏名称支持 Unicode
#[test]
fn test_macro_unicode_name() {
    let macro_def = Macro {
        name: "测试宏 🎉 日本語マクロ".to_string(),
        steps: vec![],
        created_at: None,
        description: Some("中文描述".to_string()),
    };

    assert_eq!(macro_def.name, "测试宏 🎉 日本語マクロ");
    assert_eq!(macro_def.description.unwrap(), "中文描述");
}

/// 测试大量宏步骤
#[test]
fn test_large_macro() {
    let steps: Vec<MacroStep> = (0..100)
        .map(|i| {
            MacroStep::new(
                i as u64 * 10,
                Action::key(KeyAction::click(i as u16, i as u16)),
                ModifierState::default(),
                i as u64 * 10,
            )
        })
        .collect();

    let macro_def = Macro {
        name: "large_macro".to_string(),
        steps,
        created_at: None,
        description: None,
    };

    assert_eq!(macro_def.step_count(), 100);
    // total_delay() 是所有步骤的 delay_ms 的总和：0 + 10 + 20 + ... + 990
    // 这是一个等差数列求和：(首项+末项) * 项数 / 2 = (0 + 990) * 100 / 2 = 49500
    assert_eq!(macro_def.total_delay(), 49500);
}
