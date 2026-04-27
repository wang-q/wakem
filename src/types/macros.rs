//! Macro recording and playback support

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::types::{Action, InputEvent, KeyState, ModifierState, Timestamp};

/// Minimum delay threshold in milliseconds for macro recording.
///
/// Delays shorter than this are considered negligible and are not recorded
/// as separate delay actions. This helps reduce macro size and makes
/// playback more natural by filtering out tiny timing variations.
///
/// The value of 50ms was chosen because:
/// - It's below human perception threshold for UI responsiveness (100ms)
/// - It's long enough to absorb minor timing jitter during recording
/// - It's short enough to not interfere with intentional pauses
const MIN_DELAY_MS: u64 = 50;

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

/// Macro recorder
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
            self.current_modifiers.apply_from_virtual_key(
                key_event.virtual_key,
                key_event.state == KeyState::Pressed,
            );
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
}

/// Simplify delays: merge consecutive actions, only keep necessary delays
fn simplify_delays(steps: Vec<(Duration, MacroStep)>) -> Vec<MacroStep> {
    if steps.is_empty() {
        return Vec::new();
    }

    // Pre-allocate capacity: worst case is each step preceded by a delay action
    // This is a conservative estimate to avoid reallocations
    let mut result = Vec::with_capacity(steps.len() * 2);
    let mut last_time = Duration::from_millis(0);

    for (time, mut step) in steps {
        let delay_ms = time.as_millis().saturating_sub(last_time.as_millis()) as u64;

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
        assert!(
            !step.modifiers.shift
                && !step.modifiers.ctrl
                && !step.modifiers.alt
                && !step.modifiers.meta
        );
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
