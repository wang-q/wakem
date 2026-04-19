// 性能基准测试
// 使用 Rust 内置的测试框架进行简单性能测试
// 注意: 这不是正式的 benchmark，只是基本的性能验证

use wakem_common::types::*;

/// 测试触发器匹配性能 - 简单按键
#[test]
fn benchmark_trigger_key_match() {
    let trigger = Trigger::key(0x1E, 0x41);
    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    
    // 运行多次匹配
    let iterations = 10000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = trigger.matches(&event);
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    
    println!("Trigger key match: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} ns per match", avg_ns);
    
    // 应该非常快（< 1000 ns）
    assert!(avg_ns < 1000.0, "Trigger matching too slow: {:.2} ns", avg_ns);
}

/// 测试上下文匹配性能
#[test]
fn benchmark_context_match() {
    let cond = ContextCondition::new()
        .with_process_name("notepad.exe")
        .with_window_class("Notepad");
    
    let context = ContextInfo {
        window_class: "Notepad".to_string(),
        process_name: "notepad.exe".to_string(),
        process_path: "C:\\Windows\\notepad.exe".to_string(),
        window_title: "Untitled - Notepad".to_string(),
        window_handle: 0x123456,
    };
    
    let iterations = 10000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = cond.matches(&context);
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    
    println!("Context match: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} ns per match", avg_ns);
    
    assert!(avg_ns < 5000.0, "Context matching too slow: {:.2} ns", avg_ns);
}

/// 测试层栈操作性能
#[test]
fn benchmark_layer_stack_operations() {
    let mut stack = LayerStack::new();
    
    let iterations = 10000;
    let start = std::time::Instant::now();
    
    for i in 0..iterations {
        let layer = Layer::new(&format!("layer_{}", i % 10), 0x3A + (i % 10) as u16, 0x14 + (i % 10) as u16);
        stack.activate_layer(layer);
        
        if i % 3 == 0 {
            stack.deactivate_layer(&format!("layer_{}", i % 10));
        }
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    
    println!("Layer stack operations: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} ns per operation", avg_ns);
    
    assert!(avg_ns < 2000.0, "Layer stack operations too slow: {:.2} ns", avg_ns);
}

/// 测试映射规则匹配性能
#[test]
fn benchmark_mapping_rule_match() {
    let rule = MappingRule::new(
        Trigger::key(0x1E, 0x41),
        Action::window(WindowAction::Center),
    );
    
    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    
    let context = ContextInfo::default();
    
    let iterations = 10000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = rule.matches(&event, &context);
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    
    println!("Mapping rule match: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} ns per match", avg_ns);
    
    assert!(avg_ns < 1500.0, "Rule matching too slow: {:.2} ns", avg_ns);
}

/// 测试 Action 创建性能
#[test]
fn benchmark_action_creation() {
    let iterations = 100000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = Action::key(KeyAction::click(0x1E, 0x41));
        let _ = Action::window(WindowAction::Center);
        let _ = Action::mouse(MouseAction::Move { x: 100, y: 100, relative: true });
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    
    println!("Action creation: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} ns per creation", avg_ns);
    
    assert!(avg_ns < 500.0, "Action creation too slow: {:.2} ns", avg_ns);
}

/// 测试序列化性能
#[test]
fn benchmark_serialization() {
    let action = Action::Sequence(vec![
        Action::key(KeyAction::click(0x1E, 0x41)),
        Action::window(WindowAction::Center),
        Action::mouse(MouseAction::ButtonClick { button: MouseButton::Left }),
    ]);
    
    let iterations = 10000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _ = serde_json::to_string(&action).unwrap();
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    
    println!("JSON serialization: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} ns per serialization", avg_ns);
    
    // 序列化通常较慢，放宽要求
    assert!(avg_ns < 50000.0, "Serialization too slow: {:.2} ns", avg_ns);
}

/// 测试反序列化性能
#[test]
fn benchmark_deserialization() {
    let json = r#"{"Key":{"Click":{"scan_code":30,"virtual_key":65}}}"#;
    
    let iterations = 10000;
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let _: Action = serde_json::from_str(json).unwrap();
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    
    println!("JSON deserialization: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} ns per deserialization", avg_ns);
    
    assert!(avg_ns < 10000.0, "Deserialization too slow: {:.2} ns", avg_ns);
}

/// 测试窗口计算性能
#[test]
fn benchmark_window_calculations() {
    let iterations = 100000;
    let start = std::time::Instant::now();
    
    for i in 0..iterations {
        let screen_width = 1920;
        let screen_height = 1080;
        let window_width = 800 + (i % 400) as i32;
        let window_height = 600 + (i % 300) as i32;
        
        // 居中计算
        let _x = (screen_width - window_width) / 2;
        let _y = (screen_height - window_height) / 2;
        
        // 半屏计算
        let _half_width = screen_width / 2;
        let _half_height = screen_height / 2;
    }
    
    let elapsed = start.elapsed();
    let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;
    
    println!("Window calculations: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} ns per calculation", avg_ns);
    
    assert!(avg_ns < 100.0, "Window calculations too slow: {:.2} ns", avg_ns);
}

/// 测试内存分配 - 大量层创建
#[test]
fn benchmark_layer_creation_memory() {
    let iterations = 1000;
    let start = std::time::Instant::now();
    
    let mut layers = Vec::with_capacity(iterations);
    
    for i in 0..iterations {
        let mut layer = Layer::new(&format!("layer_{}", i), 0x3A, 0x14);
        layer.add_mapping(
            Trigger::key(0x1E, 0x41),
            Action::key(KeyAction::click(0x1E, 0x41)),
        );
        layer.add_mapping(
            Trigger::key(0x30, 0x42),
            Action::window(WindowAction::Center),
        );
        layers.push(layer);
    }
    
    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() as f64 / iterations as f64;
    
    println!("Layer creation: {} layers in {:?}", iterations, elapsed);
    println!("Average: {:.2} μs per layer", avg_us);
    println!("Total layers created: {}", layers.len());
    
    assert!(avg_us < 100.0, "Layer creation too slow: {:.2} μs", avg_us);
}

/// 综合性能测试 - 模拟实际使用场景
#[test]
fn benchmark_real_world_scenario() {
    // 创建一个模拟的配置
    let mut layers: Vec<Layer> = Vec::new();
    for i in 0..5 {
        let mut layer = Layer::new(&format!("layer_{}", i), 0x3A + i as u16, 0x14 + i as u16);
        for j in 0..10 {
            layer.add_mapping(
                Trigger::key(0x1E + j as u16, 0x41 + j as u16),
                Action::window(WindowAction::Center),
            );
        }
        layers.push(layer);
    }
    
    let mut stack = LayerStack::new();
    
    // 模拟用户输入
    let iterations = 1000;
    let start = std::time::Instant::now();
    
    for i in 0..iterations {
        // 激活层
        let layer = Layer::new(&format!("layer_{}", i % 5), 0x3A + (i % 5) as u16, 0x14 + (i % 5) as u16);
        stack.activate_layer(layer);
        
        // 检查层是否激活
        let _ = stack.is_layer_active(&format!("layer_{}", i % 5));
        
        // 每100次迭代清除一次
        if i % 100 == 0 {
            stack.clear_active_layers();
        }
    }
    
    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() as f64 / iterations as f64;
    
    println!("Real world scenario: {} iterations in {:?}", iterations, elapsed);
    println!("Average: {:.2} μs per iteration", avg_us);
    
    assert!(avg_us < 50.0, "Real world scenario too slow: {:.2} μs", avg_us);
}
