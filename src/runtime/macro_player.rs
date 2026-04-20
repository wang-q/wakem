//! 宏播放器

use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::platform::windows::OutputDevice;
use crate::types::{Action, Macro};

pub struct MacroPlayer;

impl MacroPlayer {
    /// 播放宏
    pub async fn play_macro(
        output_device: &OutputDevice,
        macro_def: &Macro,
    ) -> anyhow::Result<()> {
        info!(
            "Playing macro: {} ({} actions)",
            macro_def.name,
            macro_def.actions.len()
        );

        for (delay_ms, action) in &macro_def.actions {
            // 执行延迟
            if *delay_ms > 0 {
                debug!("Macro Delay: {}ms", delay_ms);
                sleep(Duration::from_millis(*delay_ms)).await;
            }

            Self::execute_action(output_device, action)?;
        }

        info!("Macro '{}' completed", macro_def.name);
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
        Action, KeyAction, Macro, MouseAction, MouseButton, WindowAction,
    };

    #[test]
    fn test_macro_player_creation() {
        // 这个测试只是验证编译通过
        // 实际测试需要 OutputDevice，比较复杂
    }

    #[test]
    fn test_macro_action_variants() {
        // 验证所有动作变体可以创建
        let actions: Vec<(u64, Action)> = vec![
            (
                0,
                Action::key(KeyAction::Press {
                    scan_code: 30,
                    virtual_key: 65,
                }),
            ),
            (
                0,
                Action::key(KeyAction::Release {
                    scan_code: 30,
                    virtual_key: 65,
                }),
            ),
            (
                50,
                Action::mouse(MouseAction::ButtonDown {
                    button: MouseButton::Left,
                }),
            ),
            (
                0,
                Action::mouse(MouseAction::ButtonUp {
                    button: MouseButton::Left,
                }),
            ),
            (100, Action::window(WindowAction::Center)),
            (0, Action::delay(200)),
        ];

        let macro_def = Macro {
            name: "test".to_string(),
            actions,
            created_at: None,
            description: None,
        };

        assert_eq!(macro_def.actions.len(), 6);
    }
}
