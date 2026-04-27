use super::{Action, InputEvent, ModifierState};
use serde::{Deserialize, Serialize};

/// Mapping rule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

/// Trigger condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Trigger {
    /// Keyboard key
    Key {
        scan_code: Option<u16>,
        virtual_key: Option<u16>,
        modifiers: ModifierState,
    },
    /// Mouse button trigger (only matches ButtonDown events)
    MouseButton {
        button: super::MouseButton,
        modifiers: ModifierState,
    },
    /// Hot string (text expansion) - RESERVED FOR FUTURE USE
    ///
    /// # Warning
    /// This trigger type is not yet implemented. Using it will have no effect.
    /// It is reserved for future text expansion functionality.
    #[doc(alias = "text_expansion")]
    #[doc(hidden)]
    HotString { trigger: String },
    /// Chord trigger (multiple keys in sequence) - RESERVED FOR FUTURE USE
    ///
    /// # Warning
    /// This trigger type is not yet implemented. Using it will have no effect.
    /// It is reserved for future chord/sequence matching functionality.
    #[doc(alias = "sequence")]
    #[doc(hidden)]
    Chord(Vec<Trigger>),
    /// Timer trigger - RESERVED FOR FUTURE USE
    ///
    /// # Warning
    /// This trigger type is not yet implemented. Using it will have no effect.
    /// It is reserved for future timer-based automation functionality.
    #[doc(alias = "interval")]
    #[doc(hidden)]
    Timer { interval_ms: u64 },
    /// Always trigger
    Always,
}

impl Trigger {
    /// Check if input event matches this trigger condition
    ///
    /// # Supported Trigger Types
    ///
    /// - `Key`: Matches keyboard events with optional scan code, virtual key, and modifier checks
    /// - `MouseButton`: Matches mouse button down events with optional modifier checks
    /// - `HotString`: Not yet implemented (always returns false)
    /// - `Chord`: Not yet implemented (always returns false)
    /// - `Timer`: Not yet implemented (always returns false)
    /// - `Always`: Always matches
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
            (Trigger::Always, _) => true,
            // Key trigger doesn't match mouse events
            (Trigger::Key { .. }, InputEvent::Mouse(_)) => false,
            // MouseButton trigger doesn't match key events
            (Trigger::MouseButton { .. }, InputEvent::Key(_)) => false,
            // These trigger types require stateful matching infrastructure
            (Trigger::HotString { .. }, _) => false,
            (Trigger::Chord(_), _) => false,
            (Trigger::Timer { .. }, _) => false,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
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

/// Wildcard matching (supports * and ?)
///
/// Performance optimizations:
/// - Fast path for exact matches and simple patterns (no allocation)
/// - Uses dynamic programming (DP) for complex patterns
/// - Time complexity: O(m*n) worst case, O(1) best case
pub fn wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Fast path: no wildcards - case-insensitive comparison without allocation
    if !pattern.contains('*') && !pattern.contains('?') {
        return text.eq_ignore_ascii_case(pattern);
    }

    // Fast path: simple suffix match (e.g., "*.exe")
    if pattern.starts_with('*') && !pattern[1..].contains('*') && !pattern.contains('?')
    {
        let suffix = &pattern[1..];
        return text.len() >= suffix.len()
            && text[text.len() - suffix.len()..].eq_ignore_ascii_case(suffix);
    }

    // Fast path: simple prefix match (e.g., "chrome*")
    if pattern.ends_with('*')
        && !pattern[..pattern.len() - 1].contains('*')
        && !pattern.contains('?')
    {
        let prefix = &pattern[..pattern.len() - 1];
        return text.len() >= prefix.len()
            && text[..prefix.len()].eq_ignore_ascii_case(prefix);
    }

    // Complex patterns: use DP with case-insensitive comparison
    // Avoids heap allocation by comparing characters directly
    wildcard_match_dp(text, pattern)
}

/// Dynamic programming implementation of wildcard matching
/// Uses rolling array optimization (2 rows instead of full matrix)
/// Performs case-insensitive comparison without heap allocation for ASCII.
fn wildcard_match_dp(text: &str, pattern: &str) -> bool {
    let text_chars: Vec<char> = text.chars().collect();
    let pattern_chars: Vec<char> = pattern.chars().collect();

    let m = text_chars.len();
    let n = pattern_chars.len();

    if n == 0 {
        return m == 0;
    }

    /// Maximum input size for wildcard matching to prevent DoS via excessive memory allocation.
    /// Patterns or texts larger than this limit will not match.
    /// Set to 4096 to accommodate long window titles while still preventing abuse.
    const WILDCARD_MAX_INPUT_SIZE: usize = 4096;
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
                pattern_char => {
                    let text_char = text_chars[i - 1];
                    // Case-insensitive comparison for ASCII, exact match for non-ASCII
                    let matches = if pattern_char.is_ascii() && text_char.is_ascii() {
                        text_char == pattern_char
                            || text_char.eq_ignore_ascii_case(&pattern_char)
                    } else {
                        // For non-ASCII, use lowercase comparison which may allocate
                        text_char.to_lowercase().eq(pattern_char.to_lowercase())
                    };
                    curr[j] = prev[j - 1] && matches;
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
                assert!(
                    !modifiers.shift
                        && !modifiers.ctrl
                        && !modifiers.alt
                        && !modifiers.meta
                );
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
        let mut event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
        event.modifiers.ctrl = true;
        let event = InputEvent::Key(event);
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
        let mut mouse_event = crate::types::MouseEvent::new(
            crate::types::MouseEventType::ButtonDown(crate::types::MouseButton::Left),
            0,
            0,
        );
        mouse_event.modifiers.ctrl = true;
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
        let mut mouse_event3 = crate::types::MouseEvent::new(
            crate::types::MouseEventType::ButtonDown(crate::types::MouseButton::Right),
            0,
            0,
        );
        mouse_event3.modifiers.ctrl = true;
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
    fn test_mapping_rule_creation_alt() {
        let trigger = Trigger::key(0x1E, 0x41); // 'A' key
        let action = Action::Window(crate::types::WindowAction::Center);

        let rule = MappingRule::new(trigger, action);

        assert!(rule.enabled);
        assert!(rule.name.is_none());
        assert!(rule.context.is_none());
    }

    #[test]
    fn test_mapping_rule_disabled() {
        let trigger = Trigger::key(0x1E, 0x41);
        let action = Action::Window(crate::types::WindowAction::Center);

        let mut rule = MappingRule::new(trigger, action);
        rule.enabled = false;

        // Verify the rule is disabled
        assert!(!rule.enabled);
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
                assert!(
                    !modifiers.shift
                        && !modifiers.ctrl
                        && !modifiers.alt
                        && !modifiers.meta
                );
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
}
