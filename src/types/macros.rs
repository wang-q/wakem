//! 宏录制和回放支持

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::types::{Action, InputEvent};

/// 宏定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub name: String,
    /// 动作列表，每个动作包含延迟（毫秒）和动作本身
    pub actions: Vec<(u64, Action)>,
    pub created_at: Option<String>,
    pub description: Option<String>,
}

/// 宏录制器
pub struct MacroRecorder {
    recording: RwLock<Option<MacroRecording>>,
}

struct MacroRecording {
    name: String,
    start_time: Instant,
    actions: Vec<(Duration, Action)>, // (相对时间, 动作)
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
            if let Some(action) = Action::from_input_event(event) {
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

/// 简化延迟：将连续的动作合并，只保留必要的延迟
fn simplify_delays(actions: Vec<(Duration, Action)>) -> Vec<(u64, Action)> {
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
            result.push((delay_ms, Action::delay(delay_ms)));
        }

        result.push((0, action));
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
    pub fn load_from_config(&mut self, macros: &HashMap<String, Vec<(u64, Action)>>) {
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
    use crate::types::{KeyAction, KeyEvent, KeyState};

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
                Action::key(KeyAction::Press {
                    scan_code: 30,
                    virtual_key: 65,
                }),
            ),
            (
                Duration::from_millis(10),
                Action::key(KeyAction::Release {
                    scan_code: 30,
                    virtual_key: 65,
                }),
            ), // 10ms，不添加延迟
            (
                Duration::from_millis(100),
                Action::key(KeyAction::Press {
                    scan_code: 31,
                    virtual_key: 66,
                }),
            ), // 90ms，添加延迟
            (
                Duration::from_millis(200),
                Action::key(KeyAction::Release {
                    scan_code: 31,
                    virtual_key: 66,
                }),
            ), // 100ms，添加延迟
        ];

        let simplified = simplify_delays(actions);

        // 应该包含: KeyPress(0), KeyRelease(0), Delay(90), KeyPress(0), Delay(100), KeyRelease(0)
        assert_eq!(simplified.len(), 6);

        // 验证延迟动作
        if let Action::Delay { milliseconds } = simplified[2].1 {
            assert!(
                milliseconds >= 80 && milliseconds <= 100,
                "Expected delay around 90ms, got {}",
                milliseconds
            );
        } else {
            panic!("Expected Delay at index 2");
        }
    }

    #[test]
    fn test_action_from_input_event() {
        let key_event = KeyEvent::new(30, 65, KeyState::Pressed);
        let input_event = InputEvent::Key(key_event);

        let action = Action::from_input_event(&input_event);
        assert!(action.is_some());

        if let Some(Action::Key(KeyAction::Press {
            scan_code,
            virtual_key,
        })) = action
        {
            assert_eq!(scan_code, 30);
            assert_eq!(virtual_key, 65);
        } else {
            panic!("Expected Key Press action");
        }
    }
}
