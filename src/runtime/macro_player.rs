//! 宏播放器

use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::platform::windows::LegacyOutputDevice as OutputDevice;
use crate::types::{Action, KeyAction, Macro, ModifierState};

pub struct MacroPlayer;

impl MacroPlayer {
    /// 播放宏
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
            // 执行延迟
            if step.delay_ms > 0 {
                debug!("Macro Delay: {}ms", step.delay_ms);
                sleep(Duration::from_millis(step.delay_ms)).await;
            }

            // 确保修饰键状态正确
            Self::ensure_modifiers(output_device, &step.modifiers).await?;

            // 执行动作
            Self::execute_action(output_device, &step.action)?;
        }

        // 最后释放所有修饰键
        Self::release_all_modifiers(output_device).await?;

        info!("Macro '{}' completed", macro_def.name);
        Ok(())
    }

    /// 确保修饰键状态与录制时一致
    async fn ensure_modifiers(
        output: &OutputDevice,
        target: &ModifierState,
    ) -> anyhow::Result<()> {
        // 这里简化处理：假设当前没有修饰键按下
        // 实际应该跟踪当前修饰键状态，只调整差异

        if target.ctrl {
            output.send_key_action(&KeyAction::Press {
                scan_code: 0x1D,
                virtual_key: 0x11,
            })?;
        }
        if target.shift {
            output.send_key_action(&KeyAction::Press {
                scan_code: 0x2A,
                virtual_key: 0x10,
            })?;
        }
        if target.alt {
            output.send_key_action(&KeyAction::Press {
                scan_code: 0x38,
                virtual_key: 0x12,
            })?;
        }
        if target.meta {
            output.send_key_action(&KeyAction::Press {
                scan_code: 0x5B,
                virtual_key: 0x5B,
            })?;
        }

        Ok(())
    }

    /// 释放所有修饰键
    async fn release_all_modifiers(output: &OutputDevice) -> anyhow::Result<()> {
        // 释放顺序：与按下相反
        output.send_key_action(&KeyAction::Release {
            scan_code: 0x5B,
            virtual_key: 0x5B,
        })?; // Meta
        output.send_key_action(&KeyAction::Release {
            scan_code: 0x38,
            virtual_key: 0x12,
        })?; // Alt
        output.send_key_action(&KeyAction::Release {
            scan_code: 0x2A,
            virtual_key: 0x10,
        })?; // Shift
        output.send_key_action(&KeyAction::Release {
            scan_code: 0x1D,
            virtual_key: 0x11,
        })?; // Ctrl

        Ok(())
    }

    /// 执行单个动作
    fn execute_action(
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
                // 延迟在 play_macro 中处理
            }
            Action::Window(window_action) => {
                debug!("Macro WindowAction: {:?}", window_action);
                // 窗口动作需要通过 ActionMapper 执行
                // 这里暂时只记录日志
            }
            Action::Launch(launch_action) => {
                debug!("Macro LaunchAction: {:?}", launch_action);
                // 启动程序动作
            }
            Action::System(system_action) => {
                debug!("Macro SystemAction: {:?}", system_action);
                // 系统控制动作
            }
            Action::Sequence(actions) => {
                debug!("Macro Sequence: {} actions", actions.len());
                for sub_action in actions {
                    Self::execute_action(output_device, sub_action)?;
                }
            }
            Action::None => {
                // 无操作
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
        // 这个测试只是验证编译通过
        // 实际测试需要 OutputDevice，比较复杂
    }

    #[test]
    fn test_macro_step_variants() {
        // 验证所有动作变体可以创建
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
        // 测试带修饰键的宏步骤
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
