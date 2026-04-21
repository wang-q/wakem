use super::{Action, InputEvent, ModifierState};
use serde::{Deserialize, Serialize};

/// Mapping rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRule {
    /// Rule name (optional)
    pub name: Option<String>,
    /// Trigger condition
    pub trigger: Trigger,
    /// Action to execute
    pub action: Action,
    /// Context condition (optional)
    pub context: Option<ContextCondition>,
    /// Whether enabled
    pub enabled: bool,
}

impl MappingRule {
    pub fn new(trigger: Trigger, action: Action) -> Self {
        Self {
            name: None,
            trigger,
            action,
            context: None,
            enabled: true,
        }
    }

    #[allow(dead_code)]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_context(mut self, context: ContextCondition) -> Self {
        self.context = Some(context);
        self
    }

    /// Check if input event matches this rule
    #[allow(dead_code)]
    pub fn matches(&self, event: &InputEvent, context: &ContextInfo) -> bool {
        if !self.enabled {
            return false;
        }

        // Check context condition
        if let Some(ref cond) = self.context {
            if !cond.matches(
                &context.process_name,
                &context.window_class,
                &context.window_title,
                Some(&context.process_path),
            ) {
                return false;
            }
        }

        // Check trigger condition
        self.trigger.matches(event)
    }
}

/// Trigger condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    /// Keyboard key
    Key {
        scan_code: Option<u16>,
        virtual_key: Option<u16>,
        modifiers: ModifierState,
    },
    /// Mouse button
    MouseButton {
        button: super::MouseButton,
        modifiers: ModifierState,
    },
    /// Hot string (text expansion)
    HotString { trigger: String },
    /// Chord trigger (multiple keys in sequence)
    Chord(Vec<Trigger>),
    /// Timer trigger
    Timer { interval_ms: u64 },
    /// Always trigger
    Always,
}

impl Trigger {
    /// Check if input event matches this trigger condition
    pub fn matches(&self, event: &InputEvent) -> bool {
        match (self, event) {
            (
                Trigger::Key {
                    scan_code,
                    virtual_key,
                    modifiers,
                },
                InputEvent::Key(e),
            ) => {
                // Check scan code
                if let Some(sc) = scan_code {
                    if *sc != e.scan_code {
                        return false;
                    }
                }
                // Check virtual key code
                if let Some(vk) = virtual_key {
                    if *vk != e.virtual_key {
                        return false;
                    }
                }
                // Check modifiers match
                if modifiers != &e.modifiers {
                    return false;
                }
                true
            }
            (Trigger::MouseButton { button, .. }, InputEvent::Mouse(e)) => {
                // Check mouse button press
                e.is_button_down(*button)
            }
            _ => false,
        }
    }

    /// Create simple key trigger
    pub fn key(scan_code: u16, virtual_key: u16) -> Self {
        Self::Key {
            scan_code: Some(scan_code),
            virtual_key: Some(virtual_key),
            modifiers: ModifierState::default(),
        }
    }

    /// Create a trigger with modifiers
    pub fn key_with_modifiers(
        scan_code: u16,
        virtual_key: u16,
        modifiers: ModifierState,
    ) -> Self {
        Self::Key {
            scan_code: Some(scan_code),
            virtual_key: Some(virtual_key),
            modifiers,
        }
    }
}

/// Context condition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextCondition {
    /// Window class name matching (supports wildcards)
    #[serde(default)]
    pub window_class: Option<String>,
    /// Process name matching (supports wildcards)
    #[serde(default)]
    pub process_name: Option<String>,
    /// Window title matching (supports wildcards)
    #[serde(default)]
    pub window_title: Option<String>,
    /// Executable path matching (supports wildcards)
    #[serde(default)]
    pub executable_path: Option<String>,
}

impl ContextCondition {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn with_window_class(mut self, class: impl Into<String>) -> Self {
        self.window_class = Some(class.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_process_name(mut self, name: impl Into<String>) -> Self {
        self.process_name = Some(name.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_window_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = Some(title.into());
        self
    }

    #[allow(dead_code)]
    pub fn with_executable_path(mut self, path: impl Into<String>) -> Self {
        self.executable_path = Some(path.into());
        self
    }

    /// Check if current context matches
    pub fn matches(
        &self,
        process_name: &str,
        window_class: &str,
        window_title: &str,
        executable_path: Option<&str>,
    ) -> bool {
        // Check process name match
        if let Some(ref pattern) = self.process_name {
            if !wildcard_match(process_name, pattern) {
                return false;
            }
        }

        // Check window class name match
        if let Some(ref pattern) = self.window_class {
            if !wildcard_match(window_class, pattern) {
                return false;
            }
        }

        // Check window title match
        if let Some(ref pattern) = self.window_title {
            if !wildcard_match(window_title, pattern) {
                return false;
            }
        }

        // Check executable path match
        if let Some(ref pattern) = self.executable_path {
            let path = executable_path.unwrap_or("");
            if !wildcard_match(path, pattern) {
                return false;
            }
        }

        true
    }
}

/// Context information (current active window, etc.)
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct ContextInfo {
    pub window_class: String,
    pub process_name: String,
    pub process_path: String,
    pub window_title: String,
    pub window_handle: isize, // HWND
}

/// Wildcard matching using the implementation from config module
/// Supports * (matches any characters) and ? (matches single character)
fn wildcard_match(text: &str, pattern: &str) -> bool {
    crate::config::wildcard_match(text, pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{KeyEvent, KeyState};

    #[test]
    fn test_trigger_matches_simple_key() {
        let trigger = Trigger::key(0x1E, 0x41); // 'A' key

        // Matching event
        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(trigger.matches(&event));

        // Non-matching scan code
        let event = InputEvent::Key(KeyEvent::new(0x1F, 0x41, KeyState::Pressed));
        assert!(!trigger.matches(&event));

        // Non-matching virtual key
        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x42, KeyState::Pressed));
        assert!(!trigger.matches(&event));
    }

    #[test]
    fn test_trigger_matches_with_modifiers() {
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        modifiers.alt = true;
        let trigger = Trigger::key_with_modifiers(0x4B, 0x25, modifiers); // Ctrl+Alt+Left

        // Matching event with correct modifiers
        let mut event = KeyEvent::new(0x4B, 0x25, KeyState::Pressed);
        event.modifiers.ctrl = true;
        event.modifiers.alt = true;
        assert!(trigger.matches(&InputEvent::Key(event)));

        // Non-matching: missing modifiers
        let event = KeyEvent::new(0x4B, 0x25, KeyState::Pressed);
        assert!(!trigger.matches(&InputEvent::Key(event)));

        // Non-matching: wrong key
        let mut event = KeyEvent::new(0x4D, 0x27, KeyState::Pressed); // Right arrow
        event.modifiers.ctrl = true;
        event.modifiers.alt = true;
        assert!(!trigger.matches(&InputEvent::Key(event)));

        // Non-matching: extra modifier
        let mut event = KeyEvent::new(0x4B, 0x25, KeyState::Pressed);
        event.modifiers.ctrl = true;
        event.modifiers.alt = true;
        event.modifiers.shift = true;
        assert!(!trigger.matches(&InputEvent::Key(event)));
    }

    #[test]
    fn test_trigger_matches_hyper_key_combo() {
        // Simulate "Ctrl+Alt+Win+Left" window shortcut
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        modifiers.alt = true;
        modifiers.meta = true;
        let trigger = Trigger::key_with_modifiers(0x4B, 0x25, modifiers);

        // Should match when all three modifiers are pressed
        let mut event = KeyEvent::new(0x4B, 0x25, KeyState::Pressed);
        event.modifiers.ctrl = true;
        event.modifiers.alt = true;
        event.modifiers.meta = true;
        assert!(trigger.matches(&InputEvent::Key(event)));

        // Should NOT match with only two modifiers
        let mut event = KeyEvent::new(0x4B, 0x25, KeyState::Pressed);
        event.modifiers.ctrl = true;
        event.modifiers.alt = true;
        assert!(!trigger.matches(&InputEvent::Key(event)));

        // Should NOT match with no modifiers (the bug we fixed)
        let event = KeyEvent::new(0x4B, 0x25, KeyState::Pressed);
        assert!(!trigger.matches(&InputEvent::Key(event)));
    }

    #[test]
    fn test_trigger_matches_only_virtual_key() {
        // Trigger with only virtual key specified
        let trigger = Trigger::Key {
            scan_code: None,
            virtual_key: Some(0x25),
            modifiers: ModifierState::default(),
        };

        // Should match any scan code as long as virtual key matches
        let event = InputEvent::Key(KeyEvent::new(0x4B, 0x25, KeyState::Pressed));
        assert!(trigger.matches(&event));
    }

    #[test]
    fn test_mapping_rule_matches() {
        let trigger = Trigger::key(0x1E, 0x41);
        let action = Action::System(crate::types::SystemAction::VolumeUp);
        let rule = MappingRule::new(trigger, action);

        let context = ContextInfo::default();
        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(rule.matches(&event, &context));

        let event = InputEvent::Key(KeyEvent::new(0x1F, 0x41, KeyState::Pressed));
        assert!(!rule.matches(&event, &context));
    }

    #[test]
    fn test_disabled_rule_never_matches() {
        let trigger = Trigger::key(0x1E, 0x41);
        let action = Action::System(crate::types::SystemAction::VolumeUp);
        let mut rule = MappingRule::new(trigger, action);
        rule.enabled = false;

        let context = ContextInfo::default();
        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(!rule.matches(&event, &context));
    }
}
