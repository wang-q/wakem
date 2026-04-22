use criterion::{black_box, criterion_group, criterion_main, Criterion};

use wakem::types::*;

fn bench_trigger_key_match(c: &mut Criterion) {
    let trigger = Trigger::key(0x1E, 0x41);
    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));

    c.bench_function("trigger_key_match", |b| {
        b.iter(|| black_box(trigger.matches(black_box(&event))));
    });
}

fn bench_context_match(c: &mut Criterion) {
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

    c.bench_function("context_match", |b| {
        b.iter(|| {
            black_box(cond.matches(
                black_box(&context.process_name),
                black_box(&context.window_class),
                black_box(&context.window_title),
                black_box(Some(&context.process_path)),
            ))
        });
    });
}

fn bench_layer_stack_operations(c: &mut Criterion) {
    c.bench_function("layer_stack_activate_deactivate", |b| {
        b.iter(|| {
            let mut stack = LayerStack::new();
            for i in 0..10u16 {
                let layer =
                    Layer::new(&format!("layer_{}", i), 0x3A + i, 0x14 + i);
                stack.activate_layer(layer);
                if i % 3 == 0 {
                    stack.deactivate_layer(&format!("layer_{}", i));
                }
            }
            black_box(stack)
        });
    });
}

fn bench_mapping_rule_match(c: &mut Criterion) {
    let rule = MappingRule::new(
        Trigger::key(0x1E, 0x41),
        Action::window(WindowAction::Center),
    );

    let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
    let context = ContextInfo::default();

    c.bench_function("mapping_rule_match", |b| {
        b.iter(|| black_box(rule.matches(black_box(&event), black_box(&context))));
    });
}

fn bench_action_creation(c: &mut Criterion) {
    c.bench_function("action_creation", |b| {
        b.iter(|| {
            black_box(Action::key(KeyAction::click(0x1E, 0x41)));
            black_box(Action::window(WindowAction::Center));
            black_box(Action::mouse(MouseAction::Move {
                x: 100,
                y: 100,
                relative: true,
            }));
        });
    });
}

fn bench_serialization(c: &mut Criterion) {
    let action = Action::Sequence(vec![
        Action::key(KeyAction::click(0x1E, 0x41)),
        Action::window(WindowAction::Center),
        Action::mouse(MouseAction::ButtonClick {
            button: MouseButton::Left,
        }),
    ]);

    c.bench_function("json_serialization", |b| {
        b.iter(|| black_box(serde_json::to_string(black_box(&action)).unwrap()));
    });

    let json = r#"{"Key":{"Click":{"scan_code":30,"virtual_key":65}}}"#;

    c.bench_function("json_deserialization", |b| {
        b.iter(|| {
            let _: Action = serde_json::from_str(black_box(json)).unwrap();
        });
    });
}

fn bench_window_calculations(c: &mut Criterion) {
    c.bench_function("window_center_calculation", |b| {
        b.iter(|| {
            let screen_width: i32 = 1920;
            let screen_height: i32 = 1080;
            let window_width: i32 = 800;
            let window_height: i32 = 600;

            black_box((screen_width - window_width) / 2);
            black_box((screen_height - window_height) / 2);
        });
    });
}

fn bench_real_world_scenario(c: &mut Criterion) {
    c.bench_function("real_world_layer_operations", |b| {
        b.iter(|| {
            let mut stack = LayerStack::new();

            for i in 0..10u16 {
                let layer = Layer::new(&format!("layer_{}", i % 5), 0x3A + i, 0x14 + i);
                stack.activate_layer(layer);

                let _ = stack.is_layer_active(&format!("layer_{}", i % 5));

                if i % 5 == 0 {
                    stack.clear_active_layers();
                }
            }

            black_box(stack)
        });
    });
}

criterion_group!(
    benches,
    bench_trigger_key_match,
    bench_context_match,
    bench_layer_stack_operations,
    bench_mapping_rule_match,
    bench_action_creation,
    bench_serialization,
    bench_window_calculations,
    bench_real_world_scenario
);
criterion_main!(benches);
