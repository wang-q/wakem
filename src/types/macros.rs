//! Macro recording and playback support

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::types::{Action, InputEvent, KeyState, ModifierState, Timestamp};

/// Macro step (includes full context)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroStep {
    /// Delay (milliseconds)
    pub delay_ms: u64,
    /// Action
    pub action: Action,
    /// Modifier state during recording
    pub modifiers: ModifierState,
    /// Event timestamp
    pub timestamp: Timestamp,
}

impl MacroStep {
    /// Create new macro step
    pub fn new(
        delay_ms: u64,
        action: Action,
        modifiers: ModifierState,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            delay_ms,
            action,
            modifiers,
            timestamp,
        }
    }
}

/// Macro definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub name: String,
    /// Step list
    pub steps: Vec<MacroStep>,
    pub created_at: Option<String>,
    pub description: Option<String>,
}

impl Macro {
    /// Get step count
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Get total delay (milliseconds)
    pub fn total_delay(&self) -> u64 {
        self.steps.iter().map(|s| s.delay_ms).sum()
    }
}

/// Macro recorder
#[allow(dead_code)]
#[derive(Default)]
pub struct MacroRecorder {
    recording: RwLock<Option<MacroRecording>>,
}

struct MacroRecording {
    name: String,
    start_time: Instant,
    steps: Vec<(Duration, MacroStep)>,
    /// Current modifier state (real-time tracking)
    current_modifiers: ModifierState,
}

impl MacroRecording {
    /// Update current modifier state based on event
    fn update_modifiers(&mut self, event: &InputEvent) {
        if let InputEvent::Key(key_event) = event {
            if let Some((modifier, _pressed)) = ModifierState::from_virtual_key(
                key_event.virtual_key,
                key_event.state == KeyState::Pressed,
            ) {
                // Merge modifier state
                self.current_modifiers.merge(&modifier);
            }
        }
    }
}

impl MacroRecorder {
    pub fn new() -> Self {
        Self {
            recording: RwLock::new(None),
        }
    }

    /// Start recording macro
    pub async fn start_recording(&self, name: &str) -> anyhow::Result<()> {
        let mut recording = self.recording.write().await;
        if recording.is_some() {
            return Err(anyhow::anyhow!("Already recording macro"));
        }

        *recording = Some(MacroRecording {
            name: name.to_string(),
            start_time: Instant::now(),
            steps: Vec::new(),
            current_modifiers: ModifierState::default(),
        });

        info!("Started recording macro: {}", name);
        Ok(())
    }

    /// Stop recording and return macro
    pub async fn stop_recording(&self) -> anyhow::Result<Macro> {
        let mut recording = self.recording.write().await;
        let recording = recording
            .take()
            .ok_or_else(|| anyhow::anyhow!("Not recording"))?;

        // Convert to Macro (simplify delay information)
        let steps = simplify_delays(recording.steps);

        info!(
            "Stopped recording macro: {} with {} steps",
            recording.name,
            steps.len()
        );

        Ok(Macro {
            name: recording.name,
            steps,
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

    /// Record input event
    pub async fn record_event(&self, event: &InputEvent) {
        let mut recording = self.recording.write().await;
        if let Some(ref mut rec) = *recording {
            // Update current modifier state
            rec.update_modifiers(event);

            // Skip standalone modifier events
            if Self::is_standalone_modifier(event) {
                return;
            }

            let elapsed = rec.start_time.elapsed();
            if let Some(action) = Action::from_input_event(event) {
                let step = MacroStep::new(
                    elapsed.as_millis() as u64,
                    action,
                    rec.current_modifiers,
                    event.timestamp(),
                );
                debug!("Recorded step at {:?}: {:?}", elapsed, step);
                rec.steps.push((elapsed, step));
            }
        }
    }

    /// Check if event is a standalone modifier
    fn is_standalone_modifier(event: &InputEvent) -> bool {
        match event {
            InputEvent::Key(e) => e.is_modifier(),
            _ => false,
        }
    }

    /// Check if is recording
    pub async fn is_recording(&self) -> bool {
        self.recording.read().await.is_some()
    }

    /// Get current recording name
    pub async fn current_macro_name(&self) -> Option<String> {
        self.recording.read().await.as_ref().map(|r| r.name.clone())
    }
}

/// Simplify delays: merge consecutive actions, only keep necessary delays
fn simplify_delays(steps: Vec<(Duration, MacroStep)>) -> Vec<MacroStep> {
    if steps.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let mut last_time = Duration::from_millis(0);
    const MIN_DELAY_MS: u64 = 50; // Minimum delay 50ms

    for (time, mut step) in steps {
        let delay_ms = time.as_millis() as u64 - last_time.as_millis() as u64;

        // If delay exceeds threshold, add delay action
        if delay_ms > MIN_DELAY_MS {
            result.push(MacroStep::new(
                delay_ms,
                Action::delay(delay_ms),
                ModifierState::default(),
                0,
            ));
        }

        step.delay_ms = 0; // Actual actions have no delay
        result.push(step);
        last_time = time;
    }

    result
}

/// Macro manager (for loading and managing saved macros)
#[allow(dead_code)]
#[derive(Default)]
pub struct MacroManager {
    macros: HashMap<String, Macro>,
}

#[allow(dead_code)]
impl MacroManager {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
        }
    }

    /// Add macro
    pub fn add_macro(&mut self, macro_def: Macro) {
        self.macros.insert(macro_def.name.clone(), macro_def);
    }

    /// Get macro
    pub fn get_macro(&self, name: &str) -> Option<&Macro> {
        self.macros.get(name)
    }

    /// Remove macro
    pub fn remove_macro(&mut self, name: &str) -> Option<Macro> {
        self.macros.remove(name)
    }

    /// Get all macro names
    pub fn get_macro_names(&self) -> Vec<String> {
        self.macros.keys().cloned().collect()
    }

    /// Load macros from config (new format)
    pub fn load_from_config(&mut self, macros: &HashMap<String, Vec<MacroStep>>) {
        for (name, steps) in macros {
            let macro_def = Macro {
                name: name.clone(),
                steps: steps.clone(),
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

        // Start recording
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
            assert!(macro_def.steps.is_empty());
        });
    }

    #[test]
    fn test_simplify_delays() {
        let steps = vec![
            (
                Duration::from_millis(0),
                MacroStep::new(
                    0,
                    Action::key(KeyAction::Press {
                        scan_code: 30,
                        virtual_key: 65,
                    }),
                    ModifierState::default(),
                    0,
                ),
            ),
            (
                Duration::from_millis(10),
                MacroStep::new(
                    0,
                    Action::key(KeyAction::Release {
                        scan_code: 30,
                        virtual_key: 65,
                    }),
                    ModifierState::default(),
                    10,
                ),
            ), // 10ms, no delay added
            (
                Duration::from_millis(100),
                MacroStep::new(
                    0,
                    Action::key(KeyAction::Press {
                        scan_code: 31,
                        virtual_key: 66,
                    }),
                    ModifierState::default(),
                    100,
                ),
            ), // 90ms, add delay
            (
                Duration::from_millis(200),
                MacroStep::new(
                    0,
                    Action::key(KeyAction::Release {
                        scan_code: 31,
                        virtual_key: 66,
                    }),
                    ModifierState::default(),
                    200,
                ),
            ), // 100ms, add delay
        ];

        let simplified = simplify_delays(steps);

        // Should contain: KeyPress, KeyRelease, Delay(90), KeyPress, Delay(100), KeyRelease
        assert_eq!(simplified.len(), 6);

        // Verify delay action
        if let Action::Delay { milliseconds } = &simplified[2].action {
            assert!(
                *milliseconds >= 80 && *milliseconds <= 100,
                "Expected delay around 90ms, got {}",
                milliseconds
            );
        } else {
            panic!("Expected Delay at index 2");
        }
    }

    #[test]
    fn test_macro_step_creation() {
        let step = MacroStep::new(
            100,
            Action::key(KeyAction::Press {
                scan_code: 30,
                virtual_key: 65,
            }),
            ModifierState::default(),
            1234567890,
        );

        assert_eq!(step.delay_ms, 100);
        assert_eq!(step.timestamp, 1234567890);
        assert!(step.modifiers.is_empty());
    }

    #[test]
    fn test_is_standalone_modifier() {
        // Create modifier key event
        let shift_event = InputEvent::Key(KeyEvent::new(42, 0x10, KeyState::Pressed));
        assert!(MacroRecorder::is_standalone_modifier(&shift_event));

        // Create normal key event
        let normal_event = InputEvent::Key(KeyEvent::new(30, 65, KeyState::Pressed));
        assert!(!MacroRecorder::is_standalone_modifier(&normal_event));

        // Create mouse event
        use crate::types::{MouseButton, MouseEvent, MouseEventType};
        let mouse_event = InputEvent::Mouse(MouseEvent::new(
            MouseEventType::ButtonDown(MouseButton::Left),
            0,
            0,
        ));
        assert!(!MacroRecorder::is_standalone_modifier(&mouse_event));
    }
}
