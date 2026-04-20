//! 宏播放器

use tokio::time::{sleep, Duration};
use tracing::{debug, info};

use crate::platform::windows::OutputDevice;
use crate::types::{KeyAction, Macro, MacroAction};

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

        for action in &macro_def.actions {
            Self::execute_macro_action(output_device, action).await?;
        }

        info!("Macro '{}' completed", macro_def.name);
        Ok(())
    }

    /// 执行单个宏动作
    async fn execute_macro_action(
        output_device: &OutputDevice,
        action: &MacroAction,
    ) -> anyhow::Result<()> {
        match action {
            MacroAction::KeyPress {
                scan_code,
                virtual_key,
            } => {
                debug!(
                    "Macro KeyPress: scan_code={}, virtual_key={}",
                    scan_code, virtual_key
                );
                output_device.send_key_action(&KeyAction::Press {
                    scan_code: *scan_code,
                    virtual_key: *virtual_key,
                })?;
            }
            MacroAction::KeyRelease {
                scan_code,
                virtual_key,
            } => {
                debug!(
                    "Macro KeyRelease: scan_code={}, virtual_key={}",
                    scan_code, virtual_key
                );
                output_device.send_key_action(&KeyAction::Release {
                    scan_code: *scan_code,
                    virtual_key: *virtual_key,
                })?;
            }
            MacroAction::MousePress {
                button: _,
                x: _,
                y: _,
            } => {
                // 鼠标动作通过 send_mouse_action 处理
                debug!("Macro MousePress (not implemented)");
            }
            MacroAction::MouseRelease {
                button: _,
                x: _,
                y: _,
            } => {
                // 鼠标动作通过 send_mouse_action 处理
                debug!("Macro MouseRelease (not implemented)");
            }
            MacroAction::MouseMove { x: _, y: _ } => {
                // 鼠标移动需要额外实现
                debug!("Macro MouseMove (not implemented)");
            }
            MacroAction::MouseWheel { delta, horizontal } => {
                debug!(
                    "Macro MouseWheel: delta={}, horizontal={}",
                    delta, horizontal
                );
                use crate::types::MouseAction;
                if *horizontal {
                    output_device
                        .send_mouse_action(&MouseAction::HWheel { delta: *delta })?;
                } else {
                    output_device
                        .send_mouse_action(&MouseAction::Wheel { delta: *delta })?;
                }
            }
            MacroAction::Delay { milliseconds } => {
                debug!("Macro Delay: {}ms", milliseconds);
                sleep(Duration::from_millis(*milliseconds)).await;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Macro, MacroAction};

    #[test]
    fn test_macro_player_creation() {
        // 这个测试只是验证编译通过
        // 实际测试需要 OutputDevice，比较复杂
    }

    #[test]
    fn test_macro_action_variants() {
        // 验证所有宏动作变体可以创建
        use crate::types::MouseButton;
        let actions = vec![
            MacroAction::KeyPress {
                scan_code: 30,
                virtual_key: 65,
            },
            MacroAction::KeyRelease {
                scan_code: 30,
                virtual_key: 65,
            },
            MacroAction::MousePress {
                button: MouseButton::Left,
                x: 100,
                y: 200,
            },
            MacroAction::MouseRelease {
                button: MouseButton::Left,
                x: 100,
                y: 200,
            },
            MacroAction::MouseMove { x: 300, y: 400 },
            MacroAction::MouseWheel {
                delta: 120,
                horizontal: false,
            },
            MacroAction::Delay { milliseconds: 100 },
        ];

        let macro_def = Macro {
            name: "test".to_string(),
            actions,
            created_at: None,
            description: None,
        };

        assert_eq!(macro_def.actions.len(), 7);
    }
}
