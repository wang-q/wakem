//! Macro player

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, info, warn};

use crate::platform::traits::{
    LauncherTrait, OutputDeviceTrait, WindowManagerExt, WindowManagerTrait,
};
use crate::types::key_codes::{
    SCAN_CODE_ALT, SCAN_CODE_CTRL, SCAN_CODE_META, SCAN_CODE_SHIFT, VK_ALT, VK_CONTROL,
    VK_LMETA, VK_SHIFT,
};
use crate::types::{Action, KeyAction, Macro, ModifierState};

/// Cancel check interval in milliseconds for responsive macro cancellation
const CANCEL_CHECK_INTERVAL_MS: u64 = 10;

pub struct MacroPlayer;

/// Context for executing macro actions that require window manager or launcher
pub struct MacroContext<'a> {
    pub window_manager: Option<&'a (dyn WindowManagerTrait + Send + Sync)>,
    pub launcher: Option<&'a (dyn LauncherTrait + Send + Sync)>,
}

impl MacroPlayer {
    pub async fn play_macro(
        output_device: &(dyn OutputDeviceTrait + Send + Sync),
        macro_def: &Macro,
        cancel_flag: Option<Arc<AtomicBool>>,
        context: Option<MacroContext<'_>>,
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
                    Self::sleep_with_cancellation(delay_ms, flag).await
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
            Self::execute_action(output_device, &step.action, &cancel_flag, context.as_ref())
                .await?;
        }

        // Release only modifiers that were pressed by the macro player
        Self::release_held_modifiers(output_device, &current_modifiers).await?;

        info!("Macro '{}' completed", macro_def.name);
        Ok(())
    }

    /// Sleep with periodic cancellation checks for responsive macro cancellation
    async fn sleep_with_cancellation(delay_ms: u64, cancel_flag: &AtomicBool) -> bool {
        let start = Instant::now();
        let total_delay = Duration::from_millis(delay_ms);
        let check_interval = Duration::from_millis(CANCEL_CHECK_INTERVAL_MS);

        loop {
            if cancel_flag.load(Ordering::Relaxed) {
                return true;
            }

            let elapsed = start.elapsed();
            if elapsed >= total_delay {
                return false;
            }

            let remaining = total_delay - elapsed;
            let sleep_duration = std::cmp::min(remaining, check_interval);
            sleep(sleep_duration).await;
        }
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
                virtual_key: VK_CONTROL,
            })?;
            current.ctrl = true;
        }
        if target.shift && !current.shift {
            output.send_key_action(&KeyAction::Press {
                scan_code: SCAN_CODE_SHIFT,
                virtual_key: VK_SHIFT,
            })?;
            current.shift = true;
        }
        if target.alt && !current.alt {
            output.send_key_action(&KeyAction::Press {
                scan_code: SCAN_CODE_ALT,
                virtual_key: VK_ALT,
            })?;
            current.alt = true;
        }
        if target.meta && !current.meta {
            output.send_key_action(&KeyAction::Press {
                scan_code: SCAN_CODE_META,
                virtual_key: VK_LMETA,
            })?;
            current.meta = true;
        }

        // Release modifiers that are in current but not in target
        if current.meta && !target.meta {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_META,
                virtual_key: VK_LMETA,
            })?;
            current.meta = false;
        }
        if current.alt && !target.alt {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_ALT,
                virtual_key: VK_ALT,
            })?;
            current.alt = false;
        }
        if current.shift && !target.shift {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_SHIFT,
                virtual_key: VK_SHIFT,
            })?;
            current.shift = false;
        }
        if current.ctrl && !target.ctrl {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_CTRL,
                virtual_key: VK_CONTROL,
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
                virtual_key: VK_LMETA,
            })?;
        }
        if current.alt {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_ALT,
                virtual_key: VK_ALT,
            })?;
        }
        if current.shift {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_SHIFT,
                virtual_key: VK_SHIFT,
            })?;
        }
        if current.ctrl {
            output.send_key_action(&KeyAction::Release {
                scan_code: SCAN_CODE_CTRL,
                virtual_key: VK_CONTROL,
            })?;
        }

        Ok(())
    }

    async fn execute_action(
        output_device: &(dyn OutputDeviceTrait + Send + Sync),
        action: &Action,
        cancel_flag: &Option<Arc<AtomicBool>>,
        context: Option<&MacroContext<'_>>,
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
                    if Self::sleep_with_cancellation(*milliseconds, flag).await {
                        debug!("Macro cancelled during Delay action");
                    }
                } else {
                    sleep(Duration::from_millis(*milliseconds)).await;
                }
            }
            Action::Window(window_action) => {
                debug!("Macro WindowAction: {:?}", window_action);
                if let Some(ctx) = context {
                    if let Some(wm) = ctx.window_manager {
                        if let Some(window_id) = wm.get_foreground_window() {
                            use crate::types::WindowAction;
                            match window_action {
                                WindowAction::Center => {
                                    if let Err(e) = wm.move_to_center(window_id) {
                                        warn!("Failed to center window: {}", e);
                                    }
                                }
                                WindowAction::Maximize => {
                                    if let Err(e) = wm.maximize_window(window_id) {
                                        warn!("Failed to maximize window: {}", e);
                                    }
                                }
                                WindowAction::Minimize => {
                                    if let Err(e) = wm.minimize_window(window_id) {
                                        warn!("Failed to minimize window: {}", e);
                                    }
                                }
                                WindowAction::Restore => {
                                    if let Err(e) = wm.restore_window(window_id) {
                                        warn!("Failed to restore window: {}", e);
                                    }
                                }
                                WindowAction::Close => {
                                    if let Err(e) = wm.close_window(window_id) {
                                        warn!("Failed to close window: {}", e);
                                    }
                                }
                                WindowAction::ToggleTopmost => {
                                    if let Err(e) = wm.toggle_topmost(window_id) {
                                        warn!("Failed to toggle topmost: {}", e);
                                    }
                                }
                                _ => {
                                    debug!("Window action {:?} not supported in macros", window_action);
                                }
                            }
                        } else {
                            warn!("No foreground window for window action");
                        }
                    } else {
                        warn!("Window manager not available for window action");
                    }
                } else {
                    warn!("Macro context not available for window action");
                }
            }
            Action::Launch(launch_action) => {
                debug!("Macro LaunchAction: {:?}", launch_action);
                if let Some(ctx) = context {
                    if let Some(launcher) = ctx.launcher {
                        if let Err(e) = launcher.launch(launch_action) {
                            warn!("Failed to launch program: {}", e);
                        }
                    } else {
                        warn!("Launcher not available for launch action");
                    }
                } else {
                    warn!("Macro context not available for launch action");
                }
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
                        context,
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
        Action, InputEvent, KeyAction, KeyEvent, KeyState, LaunchAction, LayerMode,
        Macro, MacroStep, MappingRule, ModifierState, MouseAction, MouseButton,
        Trigger, WindowAction,
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

}
