//! Macro player

use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::platform::traits::OutputDeviceTrait;
use crate::types::{Action, KeyAction, Macro, ModifierState};

#[cfg(all(target_os = "windows", not(test)))]
use crate::platform::windows::LegacyOutputDevice as OutputDevice;

#[cfg(all(target_os = "windows", test))]
use crate::platform::windows::MockOutputDevice as OutputDevice;

#[cfg(all(target_os = "macos", not(test)))]
use crate::platform::macos::MacosOutputDevice as OutputDevice;

#[cfg(all(target_os = "macos", test))]
use crate::platform::macos::output_device::MockMacosOutputDevice as OutputDevice;

pub struct MacroPlayer;

impl MacroPlayer {
    /// Play macro
    pub async fn play_macro(
        output_device: &OutputDevice,
        macro_def: &Macro,
    ) -> anyhow::Result<()> {
        info!(
            "Playing macro: {} ({} steps)",
            macro_def.name,
            macro_def.steps.len()
        );

        for step in &macro_def.steps {
            // Execute delay
            if step.delay_ms > 0 {
                debug!("Macro Delay: {}ms", step.delay_ms);
                sleep(Duration::from_millis(step.delay_ms)).await;
            }

            // Ensure modifier state is correct
            Self::ensure_modifiers(output_device, &step.modifiers).await?;

            // Execute action
            Self::execute_action(output_device, &step.action).await?;
        }

        // Finally release all modifiers
        Self::release_all_modifiers(output_device).await?;

        info!("Macro '{}' completed", macro_def.name);
        Ok(())
    }

    /// Ensure modifier state matches recording (uses spawn_blocking to avoid blocking Tokio runtime)
    async fn ensure_modifiers(
        output: &OutputDevice,
        target: &ModifierState,
    ) -> anyhow::Result<()> {
        let output_copy = output.clone();
        let target_copy = *target;

        tokio::task::spawn_blocking(move || {
            if target_copy.ctrl {
                output_copy.send_key_action(&KeyAction::Press {
                    scan_code: 0x1D,
                    virtual_key: 0x11,
                })?;
            }
            if target_copy.shift {
                output_copy.send_key_action(&KeyAction::Press {
                    scan_code: 0x2A,
                    virtual_key: 0x10,
                })?;
            }
            if target_copy.alt {
                output_copy.send_key_action(&KeyAction::Press {
                    scan_code: 0x38,
                    virtual_key: 0x12,
                })?;
            }
            if target_copy.meta {
                output_copy.send_key_action(&KeyAction::Press {
                    scan_code: 0x5B,
                    virtual_key: 0x5B,
                })?;
            }

            Ok::<(), anyhow::Error>(())
        })
        .await?
    }

    /// Release all modifiers (uses spawn_blocking to avoid blocking Tokio runtime)
    async fn release_all_modifiers(output: &OutputDevice) -> anyhow::Result<()> {
        let output_copy = output.clone();

        tokio::task::spawn_blocking(move || {
            // Release order: opposite of press
            output_copy.send_key_action(&KeyAction::Release {
                scan_code: 0x5B,
                virtual_key: 0x5B,
            })?; // Meta
            output_copy.send_key_action(&KeyAction::Release {
                scan_code: 0x38,
                virtual_key: 0x12,
            })?; // Alt
            output_copy.send_key_action(&KeyAction::Release {
                scan_code: 0x2A,
                virtual_key: 0x10,
            })?; // Shift
            output_copy.send_key_action(&KeyAction::Release {
                scan_code: 0x1D,
                virtual_key: 0x11,
            })?; // Ctrl

            Ok::<(), anyhow::Error>(())
        })
        .await?
    }

    /// Execute single action
    async fn execute_action(
        output_device: &OutputDevice,
        action: &Action,
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
                debug!("Macro Delay: {}ms", milliseconds);
                // Delay is handled in play_macro
            }
            Action::Window(window_action) => {
                debug!("Macro WindowAction: {:?}", window_action);
                // Window actions need to be executed via ActionMapper
                // Just log here for now
            }
            Action::Launch(launch_action) => {
                debug!("Macro LaunchAction: {:?}", launch_action);
                // Launch program action
            }
            Action::System(system_action) => {
                debug!("Macro SystemAction: {:?}", system_action);
                // System control action
            }
            Action::Sequence(actions) => {
                debug!("Macro Sequence: {} actions", actions.len());
                for sub_action in actions {
                    Box::pin(Self::execute_action(output_device, sub_action)).await?;
                }
            }
            Action::None => {
                // No operation
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Action, KeyAction, Macro, MacroStep, MouseAction, MouseButton, WindowAction,
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
        assert_eq!(macro_def.step_count(), 6);
        assert_eq!(macro_def.total_delay(), 150); // 0+0+50+0+100+0
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
}
