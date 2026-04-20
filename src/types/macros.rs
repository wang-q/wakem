//! 宏录制和回放支持

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::types::{InputEvent, KeyState, MouseButton, MouseEventType};

/// 宏定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub name: String,
    pub actions: Vec<MacroAction>,
    pub created_at: Option<String>,
    pub description: Option<String>,
}

/// 宏动作（简化版，只记录关键信息）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MacroAction {
    KeyPress { scan_code: u16, virtual_key: u16 },
    KeyRelease { scan_code: u16, virtual_key: u16 },
    MousePress { button: MouseButton, x: i32, y: i32 },
    MouseRelease { button: MouseButton, x: i32, y: i32 },
    MouseMove { x: i32, y: i32 },
    MouseWheel { delta: i32, horizontal: bool },
    Delay { milliseconds: u64 },
}

/// 宏录制器
pub struct MacroRecorder {
    recording: RwLock<Option<MacroRecording>>,
}

struct MacroRecording {
    name: String,
    start_time: Instant,
    actions: Vec<(Duration, MacroAction)>, // (相对时间, 动作)
}

impl MacroRecorder {
    pub fn new() -> Self {
        Self {
            recording: RwLock::new(None),
        }
    }

    /// 开始录制宏
    pub async fn start_recording(&self, name: &str) -> anyhow::Result<()> {
        let mut recording = self.recording.write().await;
        if recording.is_some() {
            return Err(anyhow::anyhow!("Already recording macro"));
        }

        *recording = Some(MacroRecording {
            name: name.to_string(),
            start_time: Instant::now(),
            actions: Vec::new(),
        });

        info!("Started recording macro: {}", name);
        Ok(())
    }

    /// 停止录制并返回宏
    pub async fn stop_recording(&self) -> anyhow::Result<Macro> {
        let mut recording = self.recording.write().await;
        let recording = recording
            .take()
            .ok_or_else(|| anyhow::anyhow!("Not recording"))?;

        // 转换为 Macro（简化延迟信息）
        let actions = simplify_delays(recording.actions);

        info!(
            "Stopped recording macro: {} with {} actions",
            recording.name,
            actions.len()
        );

        Ok(Macro {
            name: recording.name,
            actions,
            created_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .to_string(),
            ),
            description: None,
        })
    }

    /// 记录输入事件
    pub async fn record_event(&self, event: &InputEvent) {
        let mut recording = self.recording.write().await;
        if let Some(ref mut rec) = *recording {
            let elapsed = rec.start_time.elapsed();
            if let Some(action) = convert_event_to_macro_action(event) {
                debug!("Recorded action at {:?}: {:?}", elapsed, action);
                rec.actions.push((elapsed, action));
            }
        }
    }

    /// 检查是否正在录制
    pub async fn is_recording(&self) -> bool {
        self.recording.read().await.is_some()
    }

    /// 获取当前录制名称
    pub async fn current_macro_name(&self) -> Option<String> {
        self.recording.read().await.as_ref().map(|r| r.name.clone())
    }
}

/// 将输入事件转换为宏动作
fn convert_event_to_macro_action(event: &InputEvent) -> Option<MacroAction> {
    match event {
        InputEvent::Key(key_event) => {
            if key_event.state == KeyState::Pressed {
                Some(MacroAction::KeyPress {
                    scan_code: key_event.scan_code,
                    virtual_key: key_event.virtual_key,
                })
            } else {
                Some(MacroAction::KeyRelease {
                    scan_code: key_event.scan_code,
                    virtual_key: key_event.virtual_key,
                })
            }
        }
        InputEvent::Mouse(mouse_event) => match mouse_event.event_type {
            MouseEventType::ButtonDown(button) => Some(MacroAction::MousePress {
                button,
                x: mouse_event.x,
                y: mouse_event.y,
            }),
            MouseEventType::ButtonUp(button) => Some(MacroAction::MouseRelease {
                button,
                x: mouse_event.x,
                y: mouse_event.y,
            }),
            MouseEventType::Move => Some(MacroAction::MouseMove {
                x: mouse_event.x,
                y: mouse_event.y,
            }),
            MouseEventType::Wheel(delta) => Some(MacroAction::MouseWheel {
                delta,
                horizontal: false,
            }),
            MouseEventType::HWheel(delta) => Some(MacroAction::MouseWheel {
                delta,
                horizontal: true,
            }),
        },
    }
}

/// 简化延迟：将连续的动作合并，只保留必要的延迟
fn simplify_delays(actions: Vec<(Duration, MacroAction)>) -> Vec<MacroAction> {
    if actions.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut last_time = Duration::from_millis(0);
    const MIN_DELAY_MS: u64 = 50; // 最小延迟 50ms

    for (time, action) in actions {
        let delay_ms = time.as_millis() as u64 - last_time.as_millis() as u64;

        // 如果延迟超过阈值，添加延迟动作
        if delay_ms > MIN_DELAY_MS {
            result.push(MacroAction::Delay {
                milliseconds: delay_ms,
            });
        }

        result.push(action);
        last_time = time;
    }

    result
}

/// 宏管理器（用于加载和管理已保存的宏）
pub struct MacroManager {
    macros: HashMap<String, Macro>,
}

impl MacroManager {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
        }
    }

    /// 添加宏
    pub fn add_macro(&mut self, macro_def: Macro) {
        self.macros.insert(macro_def.name.clone(), macro_def);
    }

    /// 获取宏
    pub fn get_macro(&self, name: &str) -> Option<&Macro> {
        self.macros.get(name)
    }

    /// 删除宏
    pub fn remove_macro(&mut self, name: &str) -> Option<Macro> {
        self.macros.remove(name)
    }

    /// 获取所有宏名称
    pub fn get_macro_names(&self) -> Vec<String> {
        self.macros.keys().cloned().collect()
    }

    /// 从配置加载宏
    pub fn load_from_config(&mut self, macros: &HashMap<String, Vec<MacroAction>>) {
        for (name, actions) in macros {
            let macro_def = Macro {
                name: name.clone(),
                actions: actions.clone(),
                created_at: None,
                description: None,
            };
            self.add_macro(macro_def);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_recorder_start_stop() {
        let recorder = MacroRecorder::new();

        // 开始录制
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            assert!(!recorder.is_recording().await);

            recorder.start_recording("test-macro").await.unwrap();
            assert!(recorder.is_recording().await);
            assert_eq!(
                recorder.current_macro_name().await,
                Some("test-macro".to_string())
            );

            let macro_def = recorder.stop_recording().await.unwrap();
            assert!(!recorder.is_recording().await);
            assert_eq!(macro_def.name, "test-macro");
        });
    }

    #[test]
    fn test_simplify_delays() {
        let actions = vec![
            (
                Duration::from_millis(0),
                MacroAction::KeyPress {
                    scan_code: 30,
                    virtual_key: 65,
                },
            ),
            (
                Duration::from_millis(10),
                MacroAction::KeyRelease {
                    scan_code: 30,
                    virtual_key: 65,
                },
            ), // 10ms，不添加延迟
            (
                Duration::from_millis(100),
                MacroAction::KeyPress {
                    scan_code: 31,
                    virtual_key: 66,
                },
            ), // 90ms，添加延迟
            (
                Duration::from_millis(200),
                MacroAction::KeyRelease {
                    scan_code: 31,
                    virtual_key: 66,
                },
            ), // 100ms，添加延迟
        ];

        let simplified = simplify_delays(actions);

        // 应该包含: KeyPress, KeyRelease, Delay(90), KeyPress, Delay(100), KeyRelease
        assert_eq!(simplified.len(), 6);

        // 验证延迟动作
        if let MacroAction::Delay { milliseconds } = simplified[2] {
            assert!(
                milliseconds >= 80 && milliseconds <= 100,
                "Expected delay around 90ms, got {}",
                milliseconds
            );
        } else {
            panic!("Expected Delay at index 2");
        }
    }
}
