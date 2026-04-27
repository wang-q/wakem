//! Macro player

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::platform::traits::OutputDeviceTrait;
use crate::types::key_codes::{
    SCAN_CODE_ALT, SCAN_CODE_CTRL, SCAN_CODE_META, SCAN_CODE_SHIFT,
};
use crate::types::{Action, KeyAction, Macro, ModifierState};

pub struct MacroPlayer;

impl MacroPlayer {
    pub async fn play_macro(
        output_device: &(dyn OutputDeviceTrait + Send + Sync),
        macro_def: &Macro,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> anyhow::Result<()> {
        info!(
            "Playing macro: {} ({} steps)",
            macro_def.name,
            macro_def.steps.len()
        );

        let mut current_modifiers = ModifierState::default();

        for step in &macro_def.steps {
            if let Some(ref flag) = cancel_flag {
                if flag.load(Ordering::Relaxed) {
                    debug!("Macro '{}' cancelled", macro_def.name);
                    Self::release_held_modifiers(output_device, &current_modifiers)
                        .await?;
                    return Ok(());
                }
            }

            // Execute inter-step delay (from simplify_delays, represents time gap between actions)
            if step.delay_ms > 0 {
                debug!("Macro inter-step delay: {}ms", step.delay_ms);
                let delay_ms = step.delay_ms;
                let cancelled = if let Some(ref flag) = cancel_flag {
                    tokio::select! {
                        _ = sleep(Duration::from_millis(delay_ms)) => false,
                        _ = tokio::task::yield_now() => flag.load(Ordering::Relaxed),
                    }
                } else {
                    sleep(Duration::from_millis(delay_ms)).await;
                    false
                };
                if cancelled {
                    debug!("Macro '{}' cancelled during delay", macro_def.name);
                    Self::release_held_modifiers(output_device, &current_modifiers)
                        .await?;
                    return Ok(());
                }
            }

            // Ensure modifier state is correct (only press/release differences)
            Self::ensure_modifiers(
                output_device,
                &mut current_modifiers,
                &step.modifiers,
            )
            .await?;

            // Execute action
            Self::execute_action(output_device, &step.action, &cancel_flag).await?;
        }

        // Release only modifiers that were pressed by the macro player
        Self::release_held_modifiers(output_device, &current_modifiers).await?;

        info!("Macro '{}' completed", macro_def.name);
        Ok(())
    }

    async fn ensure_modifiers(
        output: &(dyn OutputDeviceTrait + Send + Sync),
        current: &mut ModifierState,
        target: &ModifierState,
    ) -> anyhow::Result<()> {
        // Press modifiers that are in target but not in current
        if target.ctrl && !current.ctrl {
            output.send_key_action(&KeyAction::Press {
                scan_code: SCAN_CODE_CTRL,
                virtual_key: 0x11,
            })?;
            current.ctrl = true;
        }
        if target.shift && !current.shift {
            output.send_key_action(&KeyAction::Press {
                scan_code: SCAN_CODE_SHIFT,
                virtual_key: 0x10,
            })?;
            current.shift = true;
        }
        if target.alt && !current.alt {
            output.send_key_action(&KeyAction::Press {
                scan_code: SCAN_CODE_ALT,
                virtual_key: 0x12,
            })?;
            current.alt = true;
        }
        if target.meta && !current.meta {
            output.send_key_action(&KeyAction::Press {
                scan_code: SCAN_CODE_META,
                virtual_key: 0x5B,
            })?;
            current.meta = true;
        }

        // Release modifiers that are in current but not in target
        if current.meta && !target.meta {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_META,
                virtual_key: 0x5B,
            })?;
            current.meta = false;
        }
        if current.alt && !target.alt {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_ALT,
                virtual_key: 0x12,
            })?;
            current.alt = false;
        }
        if current.shift && !target.shift {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_SHIFT,
                virtual_key: 0x10,
            })?;
            current.shift = false;
        }
        if current.ctrl && !target.ctrl {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_CTRL,
                virtual_key: 0x11,
            })?;
            current.ctrl = false;
        }

        Ok(())
    }

    async fn release_held_modifiers(
        output: &(dyn OutputDeviceTrait + Send + Sync),
        current: &ModifierState,
    ) -> anyhow::Result<()> {
        if current.meta {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_META,
                virtual_key: 0x5B,
            })?;
        }
        if current.alt {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_ALT,
                virtual_key: 0x12,
            })?;
        }
        if current.shift {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_SHIFT,
                virtual_key: 0x10,
            })?;
        }
        if current.ctrl {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_CTRL,
                virtual_key: 0x11,
            })?;
        }

        Ok(())
    }

    async fn execute_action(
        output_device: &(dyn OutputDeviceTrait + Send + Sync),
        action: &Action,
        cancel_flag: &Option<Arc<AtomicBool>>,
    ) -> anyhow::Result<()> {
        match action {
            Action::Key(key_action) => {
                debug!("Macro KeyAction: {:?}", key_action);
                output_device.send_key_action(key_action)?;
            }
            Action::Mouse(mouse_action) => {
                debug!("Macro MouseAction: {:?}", mouse_action);
                output_device.send_mouse_action(mouse_action)?;
            }
            Action::Delay { milliseconds } => {
                debug!("Macro Delay action: {}ms", milliseconds);
                if let Some(ref flag) = cancel_flag {
                    tokio::select! {
                        _ = sleep(Duration::from_millis(*milliseconds)) => {},
                        _ = tokio::task::yield_now() => {
                            if flag.load(Ordering::Relaxed) {
                                debug!("Macro cancelled during Delay action");
                            }
                        }
                    }
                } else {
                    sleep(Duration::from_millis(*milliseconds)).await;
                }
            }
            Action::Window(window_action) => {
                debug!("Macro WindowAction: {:?}", window_action);
            }
            Action::Launch(launch_action) => {
                debug!("Macro LaunchAction: {:?}", launch_action);
            }
            Action::Sequence(actions) => {
                debug!("Macro Sequence: {} actions", actions.len());
                for sub_action in actions {
                    if let Some(ref flag) = cancel_flag {
                        if flag.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                    Box::pin(Self::execute_action(
                        output_device,
                        sub_action,
                        cancel_flag,
                    ))
                    .await?;
                }
            }
            Action::None => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{
        Action, ContextCondition, InputEvent, KeyAction, KeyEvent, KeyState,
        LaunchAction, LayerMode, Macro, MacroStep, MappingRule, ModifierState,
        MouseAction, MouseButton, Trigger, WindowAction,
    };

    #[test]
    fn test_macro_player_creation() {
        // This test just verifies compilation passes
        // Actual testing requires OutputDevice, which is complex
    }

    #[test]
    fn test_macro_step_variants() {
        // Verify all action variants can be created
        let steps: Vec<MacroStep> = vec![
            MacroStep::new(
                0,
                Action::key(KeyAction::Press {
                    scan_code: 30,
                    virtual_key: 65,
                }),
                ModifierState::default(),
                0,
            ),
            MacroStep::new(
                0,
                Action::key(KeyAction::Release {
                    scan_code: 30,
                    virtual_key: 65,
                }),
                ModifierState::default(),
                10,
            ),
            MacroStep::new(
                50,
                Action::mouse(MouseAction::ButtonDown {
                    button: MouseButton::Left,
                }),
                ModifierState::default(),
                50,
            ),
            MacroStep::new(
                0,
                Action::mouse(MouseAction::ButtonUp {
                    button: MouseButton::Left,
                }),
                ModifierState::default(),
                60,
            ),
            MacroStep::new(
                100,
                Action::window(WindowAction::Center),
                ModifierState::default(),
                160,
            ),
            MacroStep::new(0, Action::delay(200), ModifierState::default(), 360),
        ];

        let macro_def = Macro {
            name: "test".to_string(),
            steps,
            created_at: None,
            description: None,
        };

        assert_eq!(macro_def.steps.len(), 6);
    }

    #[test]
    fn test_macro_with_modifiers() {
        // Test macro steps with modifiers
        let mut modifiers = ModifierState::default();
        modifiers.ctrl = true;
        modifiers.shift = true;

        let step = MacroStep::new(
            0,
            Action::key(KeyAction::Press {
                scan_code: 46, // C key
                virtual_key: 67,
            }),
            modifiers,
            1000,
        );

        assert!(step.modifiers.ctrl);
        assert!(step.modifiers.shift);
        assert!(!step.modifiers.alt);
        assert!(!step.modifiers.meta);
        assert_eq!(step.timestamp, 1000);
    }

    // ==================== Additional tests from ut_runtime_mapper_full.rs ====================

    #[test]
    fn test_empty_macro() {
        let macro_def = Macro {
            name: "empty".to_string(),
            steps: vec![],
            created_at: None,
            description: None,
        };

        assert_eq!(macro_def.steps.len(), 0);
    }

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

        assert_eq!(macro_def.steps.len(), 1);
        assert_eq!(macro_def.name, "single");
        assert!(macro_def.created_at.is_some());
        assert!(macro_def.description.is_some());
    }

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

        assert_eq!(macro_def.steps.len(), 4);
    }

    #[test]
    fn test_macro_step_with_modifiers_alt() {
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

    #[test]
    fn test_macro_steps_all_action_types() {
        let steps: Vec<MacroStep> = vec![
            // Key action
            MacroStep::new(
                0,
                Action::key(KeyAction::Press {
                    scan_code: 0x1E,
                    virtual_key: 0x41,
                }),
                ModifierState::default(),
                0,
            ),
            // Mouse action
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
            // Window action
            MacroStep::new(
                20,
                Action::window(WindowAction::Center),
                ModifierState::default(),
                20,
            ),
            // Delay action
            MacroStep::new(30, Action::delay(500), ModifierState::default(), 30),
            // Sequence action
            MacroStep::new(
                40,
                Action::sequence(vec![
                    Action::key(KeyAction::click(0x01, 0x1B)),
                    Action::key(KeyAction::click(0x0E, 0x08)),
                ]),
                ModifierState::default(),
                40,
            ),
            // No-op
            MacroStep::new(50, Action::None, ModifierState::default(), 50),
        ];

        let macro_def = Macro {
            name: "all_types".to_string(),
            steps,
            created_at: None,
            description: None,
        };

        assert_eq!(macro_def.steps.len(), 6);
    }

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

        assert_eq!(macro_def.steps.len(), 100);
    }

    // ==================== Additional tests from ut_runtime_mapper.rs ====================

    #[test]
    fn test_action_variants() {
        let key_action = Action::key(KeyAction::click(0x1E, 0x41));
        assert!(matches!(key_action, Action::Key(_)));

        let mouse_action = Action::mouse(MouseAction::Move {
            x: 100,
            y: 100,
            relative: false,
        });
        assert!(matches!(mouse_action, Action::Mouse(_)));

        let window_action = Action::window(WindowAction::Maximize);
        assert!(matches!(window_action, Action::Window(_)));

        let launch_action = Action::launch("notepad.exe");
        assert!(matches!(launch_action, Action::Launch(_)));

        let delay_action = Action::delay(100);
        assert!(matches!(delay_action, Action::Delay { .. }));
    }

    #[test]
    fn test_key_action_variants() {
        let press = KeyAction::Press {
            scan_code: 0x1E,
            virtual_key: 0x41,
        };
        assert!(matches!(press, KeyAction::Press { .. }));

        let release = KeyAction::Release {
            scan_code: 0x1E,
            virtual_key: 0x41,
        };
        assert!(matches!(release, KeyAction::Release { .. }));

        let click = KeyAction::click(0x1E, 0x41);
        assert!(matches!(click, KeyAction::Click { .. }));

        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        let combo = KeyAction::combo(modifiers, 0x1E, 0x41);
        assert!(matches!(combo, KeyAction::Combo { .. }));

        let type_text = KeyAction::TypeText("hello".to_string());
        assert!(matches!(type_text, KeyAction::TypeText(_)));

        let none = KeyAction::None;
        assert!(matches!(none, KeyAction::None));
    }

    #[test]
    fn test_key_event_creation_alt() {
        let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);

        assert_eq!(event.scan_code, 0x1E);
        assert_eq!(event.virtual_key, 0x41);
        assert!(matches!(event.state, KeyState::Pressed));
    }

    #[test]
    fn test_key_event_with_modifiers_alt() {
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        modifiers.shift = true;
        let event =
            KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(modifiers);

        assert!(event.modifiers.ctrl);
        assert!(event.modifiers.shift);
        assert!(!event.modifiers.alt);
    }

    #[test]
    fn test_key_event_injected_alt() {
        let event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected();

        assert!(event.is_injected);
    }

    #[test]
    fn test_modifier_state_alt() {
        let mut state = ModifierState::new();
        state.ctrl = true;
        assert!(state.ctrl);
        assert!(!state.shift);

        let mut state = ModifierState::new();
        state.shift = true;
        assert!(!state.ctrl);
        assert!(state.shift);

        let mut state = ModifierState::new();
        state.alt = true;
        assert!(state.alt);

        let mut state = ModifierState::new();
        state.meta = true;
        assert!(state.meta);
    }

    #[test]
    fn test_trigger_matching_alt() {
        let trigger = Trigger::key(0x1E, 0x41);

        let matching_event =
            InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(trigger.matches(&matching_event));

        let non_matching_event =
            InputEvent::Key(KeyEvent::new(0x1F, 0x42, KeyState::Pressed));
        assert!(!trigger.matches(&non_matching_event));
    }

    #[test]
    fn test_window_action_variants_alt() {
        let center = WindowAction::Center;
        assert!(matches!(center, WindowAction::Center));

        let maximize = WindowAction::Maximize;
        assert!(matches!(maximize, WindowAction::Maximize));

        let minimize = WindowAction::Minimize;
        assert!(matches!(minimize, WindowAction::Minimize));

        let close = WindowAction::Close;
        assert!(matches!(close, WindowAction::Close));

        let resize = WindowAction::Resize {
            width: 800,
            height: 600,
        };
        assert!(matches!(resize, WindowAction::Resize { .. }));
    }

    #[test]
    fn test_mouse_action_variants_alt() {
        let move_action = MouseAction::Move {
            x: 100,
            y: 200,
            relative: false,
        };
        assert!(matches!(move_action, MouseAction::Move { .. }));

        let relative_move = MouseAction::Move {
            x: 10,
            y: -10,
            relative: true,
        };
        assert!(matches!(relative_move, MouseAction::Move { .. }));
    }

    #[test]
    fn test_launch_action_alt() {
        let action = LaunchAction {
            program: "notepad.exe".to_string(),
            args: vec![],
            working_dir: None,
            env_vars: vec![],
        };
        assert_eq!(action.program, "notepad.exe");
        assert!(action.args.is_empty());

        let action = LaunchAction {
            program: "code".to_string(),
            args: vec![".".to_string()],
            working_dir: None,
            env_vars: vec![],
        };
        assert_eq!(action.program, "code");
        assert_eq!(action.args, vec!["."]);
    }

    #[test]
    fn test_action_sequence_alt() {
        let actions = vec![
            Action::key(KeyAction::click(0x1E, 0x41)),
            Action::key(KeyAction::click(0x1F, 0x42)),
            Action::key(KeyAction::click(0x20, 0x43)),
        ];

        assert_eq!(actions.len(), 3);
    }

    #[test]
    fn test_event_sequence_alt() {
        let events = vec![
            InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed)),
            InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Released)),
            InputEvent::Key(KeyEvent::new(0x1F, 0x42, KeyState::Pressed)),
            InputEvent::Key(KeyEvent::new(0x1F, 0x42, KeyState::Released)),
        ];

        assert_eq!(events.len(), 4);
    }

    #[test]
    fn test_mapping_rule_enabled() {
        let rule = MappingRule::new(
            Trigger::key(0x1E, 0x41),
            Action::key(KeyAction::click(0x1F, 0x42)),
        );

        assert!(rule.enabled);
    }

    #[test]
    fn test_layer_modes_alt2() {
        let toggle_mode = LayerMode::Toggle;
        assert!(matches!(toggle_mode, LayerMode::Toggle));

        let hold_mode = LayerMode::Hold;
        assert!(matches!(hold_mode, LayerMode::Hold));
    }

    #[test]
    fn test_trigger_variants_alt() {
        let key_trigger = Trigger::key(0x1E, 0x41);
        assert!(matches!(key_trigger, Trigger::Key { .. }));

        let mouse_trigger = Trigger::MouseButton {
            button: MouseButton::Left,
            modifiers: ModifierState::new(),
        };
        assert!(matches!(mouse_trigger, Trigger::MouseButton { .. }));

        let hotstring_trigger = Trigger::HotString {
            trigger: "test".to_string(),
        };
        assert!(matches!(hotstring_trigger, Trigger::HotString { .. }));
    }

    #[test]
    fn test_key_state_variants() {
        let pressed = KeyState::Pressed;
        assert!(matches!(pressed, KeyState::Pressed));

        let released = KeyState::Released;
        assert!(matches!(released, KeyState::Released));
    }

    #[test]
    fn test_mouse_button_variants() {
        assert!(matches!(MouseButton::Left, MouseButton::Left));
        assert!(matches!(MouseButton::Right, MouseButton::Right));
        assert!(matches!(MouseButton::Middle, MouseButton::Middle));
        assert!(matches!(MouseButton::X1, MouseButton::X1));
        assert!(matches!(MouseButton::X2, MouseButton::X2));
    }
}
