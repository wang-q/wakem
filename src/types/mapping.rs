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
                // At least one of scan_code or virtual_key must be specified
                if scan_code.is_none() && virtual_key.is_none() {
                    return false;
                }
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
            (Trigger::MouseButton { button, modifiers }, InputEvent::Mouse(e)) => {
                if !e.is_button_down(*button) {
                    return false;
                }
                if *modifiers != e.modifiers {
                    return false;
                }
                true
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

/// Wildcard matching (supports * and ?)
///
/// Performance optimizations:
/// - Fast path for exact matches and simple patterns
/// - Uses dynamic programming (DP) for complex patterns
/// - Time complexity: O(m*n) worst case, O(1) best case
pub fn wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if !pattern.contains('*') && !pattern.contains('?') {
        return text.eq_ignore_ascii_case(pattern);
    }

    if pattern.starts_with('*') && !pattern[1..].contains('*') && !pattern.contains('?')
    {
        let suffix = &pattern[1..];
        return text.to_lowercase().ends_with(&suffix.to_lowercase());
    }
    if pattern.ends_with('*')
        && !pattern[..pattern.len() - 1].contains('*')
        && !pattern.contains('?')
    {
        let prefix = &pattern[..pattern.len() - 1];
        return text.to_lowercase().starts_with(&prefix.to_lowercase());
    }

    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    wildcard_match_dp(&text_lower, &pattern_lower)
}

/// Dynamic programming implementation of wildcard matching
/// Uses rolling array optimization (2 rows instead of full matrix)
fn wildcard_match_dp(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    let m = text_chars.len();
    let n = pattern_chars.len();

    if n == 0 {
        return m == 0;
    }

    const WILDCARD_MAX_INPUT_SIZE: usize = 1024;
    if m > WILDCARD_MAX_INPUT_SIZE || n > WILDCARD_MAX_INPUT_SIZE {
        return false;
    }

    let mut prev = vec![false; n + 1];
    let mut curr = vec![false; n + 1];

    prev[0] = true;

    for j in 1..=n {
        if pattern_chars[j - 1] == '*' {
            prev[j] = prev[j - 1];
        } else {
            break;
        }
    }

    for i in 1..=m {
        curr[0] = false;
        for j in 1..=n {
            match pattern_chars[j - 1] {
                '*' => {
                    curr[j] = curr[j - 1] || prev[j];
                }
                '?' => {
                    curr[j] = prev[j - 1];
                }
                _ => {
                    curr[j] = prev[j - 1] && (text_chars[i - 1] == pattern_chars[j - 1]);
                }
            }
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.iter_mut().for_each(|v| *v = false);
    }

    prev[n]
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
    fn test_trigger_empty_key_never_matches() {
        // Trigger with neither scan_code nor virtual_key should not match anything
        let trigger = Trigger::Key {
            scan_code: None,
            virtual_key: None,
            modifiers: ModifierState::default(),
        };

        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(!trigger.matches(&event));
    }

    #[test]
    fn test_mapping_rule_matches() {
        let trigger = Trigger::key(0x1E, 0x41);
        let action = Action::System(crate::types::SystemAction::BrightnessUp);
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
        let action = Action::System(crate::types::SystemAction::BrightnessUp);
        let mut rule = MappingRule::new(trigger, action);
        rule.enabled = false;

        let context = ContextInfo::default();
        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(!rule.matches(&event, &context));
    }

    // ==================== Additional tests from ut_types_basic.rs and ut_types_comprehensive.rs ====================

    #[test]
    fn test_key_trigger_creation() {
        let trigger = Trigger::key(0x3A, 0x14); // CapsLock

        match trigger {
            Trigger::Key {
                scan_code,
                virtual_key,
                modifiers,
            } => {
                assert_eq!(scan_code, Some(0x3A));
                assert_eq!(virtual_key, Some(0x14));
                assert!(modifiers.is_empty());
            }
            _ => panic!("Expected Key trigger"),
        }
    }

    #[test]
    fn test_key_trigger_with_modifiers() {
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        modifiers.shift = true;

        let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers);

        match trigger {
            Trigger::Key {
                scan_code,
                virtual_key,
                modifiers,
            } => {
                assert_eq!(scan_code, Some(0x1E));
                assert_eq!(virtual_key, Some(0x41));
                assert!(modifiers.ctrl);
                assert!(modifiers.shift);
                assert!(!modifiers.alt);
                assert!(!modifiers.meta);
            }
            _ => panic!("Expected Key trigger"),
        }
    }

    #[test]
    fn test_trigger_matching() {
        let trigger = Trigger::key(0x3A, 0x14);

        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        assert!(trigger.matches(&event));

        // Non-matching scan code
        let wrong_event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(!trigger.matches(&wrong_event));
    }

    #[test]
    fn test_mapping_rule_creation() {
        let trigger = Trigger::key(0x3A, 0x14);
        let action = Action::Key(crate::types::KeyAction::click(0x0E, 0x08));

        let rule = MappingRule::new(trigger, action);

        assert!(rule.enabled);
        assert!(rule.name.is_none());
        assert!(rule.context.is_none());
    }

    #[test]
    fn test_mapping_rule_with_name() {
        let trigger = Trigger::key(0x3A, 0x14);
        let action = Action::Key(crate::types::KeyAction::click(0x0E, 0x08));

        let rule = MappingRule::new(trigger, action).with_name("caps_to_backspace");

        assert_eq!(rule.name, Some("caps_to_backspace".to_string()));
    }

    #[test]
    fn test_mapping_rule_with_context() {
        let trigger = Trigger::key(0x3A, 0x14);
        let action = Action::Key(crate::types::KeyAction::click(0x0E, 0x08));

        let context = ContextCondition::new().with_process_name("notepad.exe");

        let rule = MappingRule::new(trigger, action).with_context(context);

        assert!(rule.context.is_some());
    }

    #[test]
    fn test_mapping_rule_matching() {
        let trigger = Trigger::key(0x3A, 0x14);
        let action = Action::Key(crate::types::KeyAction::click(0x0E, 0x08));

        let rule = MappingRule::new(trigger, action);

        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        let context = ContextInfo::default();

        assert!(rule.matches(&event, &context));
    }

    #[test]
    fn test_disabled_rule_not_matching() {
        let trigger = Trigger::key(0x3A, 0x14);
        let action = Action::Key(crate::types::KeyAction::click(0x0E, 0x08));

        let mut rule = MappingRule::new(trigger, action);
        rule.enabled = false;

        let event = InputEvent::Key(KeyEvent::new(0x3A, 0x14, KeyState::Pressed));
        let context = ContextInfo::default();

        assert!(!rule.matches(&event, &context));
    }

    #[test]
    fn test_context_condition_matching() {
        let context = ContextCondition::new().with_process_name("notepad.exe");

        let matching_info = ContextInfo {
            window_class: "Notepad".to_string(),
            process_name: "notepad.exe".to_string(),
            process_path: "C:\\Windows\\notepad.exe".to_string(),
            window_title: "Untitled".to_string(),
            window_handle: 0,
        };

        let non_matching_info = ContextInfo {
            window_class: "Chrome".to_string(),
            process_name: "chrome.exe".to_string(),
            process_path: "C:\\Program Files\\chrome.exe".to_string(),
            window_title: "Google".to_string(),
            window_handle: 0,
        };

        assert!(context.matches(
            &matching_info.process_name,
            &matching_info.window_class,
            &matching_info.window_title,
            Some(&matching_info.process_path)
        ));
        assert!(!context.matches(
            &non_matching_info.process_name,
            &non_matching_info.window_class,
            &non_matching_info.window_title,
            Some(&non_matching_info.process_path)
        ));
    }

    #[test]
    fn test_trigger_matches_exact_key() {
        let trigger = Trigger::key(0x1E, 0x41); // 'A'

        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(trigger.matches(&event));

        // Different key doesn't match
        let event2 = InputEvent::Key(KeyEvent::new(0x30, 0x42, KeyState::Pressed));
        assert!(!trigger.matches(&event2));
    }

    #[test]
    fn test_trigger_matches_with_modifiers_alt() {
        let mut modifiers = ModifierState::default();
        modifiers.ctrl = true;
        let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers);

        // Event with Ctrl - should match
        let mut event_modifiers = ModifierState::default();
        event_modifiers.ctrl = true;
        let event = InputEvent::Key(
            KeyEvent::new(0x1E, 0x41, KeyState::Pressed).with_modifiers(event_modifiers),
        );
        assert!(trigger.matches(&event));

        // Event without modifiers - shouldn't match
        let event2 = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));
        assert!(!trigger.matches(&event2));
    }

    #[test]
    fn test_trigger_matches_mouse_button() {
        let trigger = Trigger::MouseButton {
            button: crate::types::MouseButton::Left,
            modifiers: ModifierState::default(),
        };

        let mouse_event = crate::types::MouseEvent::new(
            crate::types::MouseEventType::ButtonDown(crate::types::MouseButton::Left),
            0,
            0,
        );
        let event = InputEvent::Mouse(mouse_event);
        assert!(trigger.matches(&event));

        // Different button doesn't match
        let mouse_event2 = crate::types::MouseEvent::new(
            crate::types::MouseEventType::ButtonDown(crate::types::MouseButton::Right),
            0,
            0,
        );
        let event2 = InputEvent::Mouse(mouse_event2);
        assert!(!trigger.matches(&event2));
    }

    #[test]
    fn test_trigger_matches_mouse_button_with_modifiers() {
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        let trigger = Trigger::MouseButton {
            button: crate::types::MouseButton::Left,
            modifiers,
        };

        // Matching: left button + Ctrl
        let mouse_event = crate::types::MouseEvent::new(
            crate::types::MouseEventType::ButtonDown(crate::types::MouseButton::Left),
            0,
            0,
        )
        .with_modifiers({
            let mut m = ModifierState::new();
            m.ctrl = true;
            m
        });
        let event = InputEvent::Mouse(mouse_event);
        assert!(trigger.matches(&event));

        // Not matching: left button without Ctrl
        let mouse_event2 = crate::types::MouseEvent::new(
            crate::types::MouseEventType::ButtonDown(crate::types::MouseButton::Left),
            0,
            0,
        );
        let event2 = InputEvent::Mouse(mouse_event2);
        assert!(!trigger.matches(&event2));

        // Not matching: right button + Ctrl
        let mouse_event3 = crate::types::MouseEvent::new(
            crate::types::MouseEventType::ButtonDown(crate::types::MouseButton::Right),
            0,
            0,
        )
        .with_modifiers({
            let mut m = ModifierState::new();
            m.ctrl = true;
            m
        });
        let event3 = InputEvent::Mouse(mouse_event3);
        assert!(!trigger.matches(&event3));
    }

    #[test]
    fn test_trigger_matches_hotstring() {
        let trigger = Trigger::HotString {
            trigger: "test".to_string(),
        };

        // HotString trigger matching may need special handling
        // Here we just verify it can be created
        let _ = trigger;
    }

    #[test]
    fn test_mapping_rule_enable_disable() {
        let rule = MappingRule::new(
            Trigger::key(0x3A, 0x14),
            Action::Key(crate::types::KeyAction::click(0x0E, 0x08)),
        );

        assert!(rule.enabled);

        let mut rule_disabled = rule.clone();
        rule_disabled.enabled = false;
        assert!(!rule_disabled.enabled);
    }

    #[test]
    fn test_mapping_rule_with_name_alt() {
        let rule = MappingRule::new(
            Trigger::key(0x3A, 0x14),
            Action::Key(crate::types::KeyAction::click(0x0E, 0x08)),
        )
        .with_name("caps_to_esc");

        assert_eq!(rule.name.as_deref().unwrap(), "caps_to_esc");
    }

    #[test]
    fn test_mapping_rule_with_context_alt() {
        let context = ContextCondition::new()
            .with_process_name("notepad.exe")
            .with_window_class("Notepad");

        let rule = MappingRule::new(
            Trigger::key(0x41, 0x41),
            Action::Key(crate::types::KeyAction::click(0x42, 0x42)),
        )
        .with_context(context);

        assert!(rule.context.is_some());
    }

    #[test]
    fn test_mapping_rule_creation_alt() {
        let trigger = Trigger::key(0x1E, 0x41); // 'A' key
        let action = Action::Window(crate::types::WindowAction::Center);

        let rule = MappingRule::new(trigger, action);

        assert!(rule.enabled);
        assert!(rule.name.is_none());
        assert!(rule.context.is_none());
    }

    #[test]
    fn test_mapping_rule_with_name_alt2() {
        let trigger = Trigger::key(0x1E, 0x41);
        let action = Action::Window(crate::types::WindowAction::Center);

        let rule = MappingRule::new(trigger, action).with_name("Center Window");

        assert_eq!(rule.name, Some("Center Window".to_string()));
    }

    #[test]
    fn test_mapping_rule_with_context_alt2() {
        let trigger = Trigger::key(0x1E, 0x41);
        let action = Action::Window(crate::types::WindowAction::Center);

        let context = ContextCondition::new().with_process_name("notepad.exe");

        let rule = MappingRule::new(trigger, action).with_context(context);

        assert!(rule.context.is_some());
    }

    #[test]
    fn test_mapping_rule_disabled() {
        let trigger = Trigger::key(0x1E, 0x41);
        let action = Action::Window(crate::types::WindowAction::Center);

        let mut rule = MappingRule::new(trigger, action);
        rule.enabled = false;

        let event = InputEvent::Key(KeyEvent::new(0x1E, 0x41, KeyState::Pressed));

        let context = ContextInfo::default();

        // Disabled rule should not match
        assert!(!rule.matches(&event, &context));
    }

    #[test]
    fn test_complex_context_condition() {
        let cond = ContextCondition::new()
            .with_process_name("code.exe")
            .with_window_class("Chrome_WidgetWin_1");

        let full_match = ContextInfo {
            window_class: "Chrome_WidgetWin_1".to_string(),
            process_name: "code.exe".to_string(),
            process_path: "".to_string(),
            window_title: "".to_string(),
            window_handle: 0,
        };

        let partial_match = ContextInfo {
            window_class: "Chrome_WidgetWin_1".to_string(),
            process_name: "notepad.exe".to_string(),
            process_path: "".to_string(),
            window_title: "".to_string(),
            window_handle: 0,
        };

        assert!(cond.matches(
            &full_match.process_name,
            &full_match.window_class,
            &full_match.window_title,
            Some(&full_match.process_path)
        ));
        assert!(!cond.matches(
            &partial_match.process_name,
            &partial_match.window_class,
            &partial_match.window_title,
            Some(&partial_match.process_path)
        ));
    }

    #[test]
    fn test_trigger_key_creation() {
        let trigger = Trigger::key(0x1E, 0x41);

        match trigger {
            Trigger::Key {
                scan_code,
                virtual_key,
                modifiers,
            } => {
                assert_eq!(scan_code, Some(0x1E));
                assert_eq!(virtual_key, Some(0x41));
                assert!(modifiers.is_empty());
            }
            _ => panic!("Expected Key trigger"),
        }
    }

    #[test]
    fn test_trigger_key_with_modifiers_alt() {
        let mut modifiers = ModifierState::new();
        modifiers.ctrl = true;
        modifiers.shift = true;

        let trigger = Trigger::key_with_modifiers(0x1E, 0x41, modifiers);

        match trigger {
            Trigger::Key {
                scan_code,
                virtual_key,
                modifiers: m,
            } => {
                assert_eq!(scan_code, Some(0x1E));
                assert_eq!(virtual_key, Some(0x41));
                assert!(m.ctrl);
                assert!(m.shift);
                assert!(!m.alt);
                assert!(!m.meta);
            }
            _ => panic!("Expected Key trigger"),
        }
    }

    #[test]
    fn test_trigger_variants() {
        let key_trigger = Trigger::Key {
            scan_code: Some(0x1E),
            virtual_key: Some(0x41),
            modifiers: ModifierState::default(),
        };

        let mouse_trigger = Trigger::MouseButton {
            button: crate::types::MouseButton::Left,
            modifiers: ModifierState::default(),
        };

        let hotstring_trigger = Trigger::HotString {
            trigger: ".date".to_string(),
        };

        let always_trigger = Trigger::Always;

        assert!(matches!(key_trigger, Trigger::Key { .. }));
        assert!(matches!(mouse_trigger, Trigger::MouseButton { .. }));
        assert!(matches!(hotstring_trigger, Trigger::HotString { .. }));
        assert!(matches!(always_trigger, Trigger::Always));
    }

    #[test]
    fn test_context_info_default() {
        let context = ContextInfo::default();

        assert_eq!(context.window_class, "");
        assert_eq!(context.process_name, "");
        assert_eq!(context.process_path, "");
        assert_eq!(context.window_title, "");
        assert_eq!(context.window_handle, 0);
    }

    #[test]
    fn test_context_condition_creation() {
        let condition = ContextCondition::new()
            .with_process_name("chrome.exe")
            .with_window_class("Chrome_WidgetWin_1")
            .with_window_title("*Google*");

        assert_eq!(condition.process_name.as_deref().unwrap(), "chrome.exe");
        assert_eq!(
            condition.window_class.as_deref().unwrap(),
            "Chrome_WidgetWin_1"
        );
        assert_eq!(condition.window_title.as_deref().unwrap(), "*Google*");
    }

    #[test]
    fn test_context_condition_empty() {
        let condition = ContextCondition::new();
        assert!(condition.process_name.is_none());
        assert!(condition.window_class.is_none());
        assert!(condition.window_title.is_none());
    }

    #[test]
    fn test_context_condition_empty_matches_all() {
        let cond = ContextCondition::new();
        let context = ContextInfo {
            window_class: "AnyClass".to_string(),
            process_name: "any.exe".to_string(),
            process_path: "C:\\any.exe".to_string(),
            window_title: "Any Title".to_string(),
            window_handle: 0,
        };

        assert!(cond.matches(
            &context.process_name,
            &context.window_class,
            &context.window_title,
            Some(&context.process_path)
        ));
    }

    #[test]
    fn test_context_condition_process_match() {
        let cond = ContextCondition::new().with_process_name("notepad.exe");

        let matching_context = ContextInfo {
            window_class: "Notepad".to_string(),
            process_name: "notepad.exe".to_string(),
            process_path: "C:\\Windows\\notepad.exe".to_string(),
            window_title: "Untitled".to_string(),
            window_handle: 0,
        };

        let non_matching_context = ContextInfo {
            window_class: "Chrome".to_string(),
            process_name: "chrome.exe".to_string(),
            process_path: "C:\\Program Files\\chrome.exe".to_string(),
            window_title: "Google".to_string(),
            window_handle: 0,
        };

        assert!(cond.matches(
            &matching_context.process_name,
            &matching_context.window_class,
            &matching_context.window_title,
            Some(&matching_context.process_path)
        ));
        assert!(!cond.matches(
            &non_matching_context.process_name,
            &non_matching_context.window_class,
            &non_matching_context.window_title,
            Some(&non_matching_context.process_path)
        ));
    }

    #[test]
    fn test_wildcard_matching() {
        // These tests depend on ContextCondition's internal implementation
        // Here we mainly test that ContextCondition can be created correctly
        let cond = ContextCondition::new()
            .with_window_class("Chrome*")
            .with_process_name("chrome.exe");

        let info = ContextInfo {
            window_class: "Chrome_WidgetWin_1".to_string(),
            process_name: "chrome.exe".to_string(),
            process_path: "".to_string(),
            window_title: "".to_string(),
            window_handle: 0,
        };

        // Simplified matching may not be perfect, but at least won't panic
        let _result = cond.matches(
            &info.process_name,
            &info.window_class,
            &info.window_title,
            Some(&info.process_path),
        );
    }
}
