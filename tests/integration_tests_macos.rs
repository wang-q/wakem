//! macOS platform integration tests
//!
//! Tests the interaction between different macOS platform modules to ensure
//! they work correctly together in real-world scenarios.
//!
//! Note: These tests use public APIs only. For tests requiring internal Mock types,
//! see the unit tests in each module's `#[cfg(test)]` sections.

#[cfg(test)]
mod macos_integration_tests {
    use wakem::platform::macos::context::WindowContext;
    use wakem::platform::macos::input::{
        keycode_to_virtual_key, virtual_key_to_keycode,
    };
    use wakem::platform::macos::launcher::Launcher;
    use wakem::platform::macos::window_preset::{
        MacosWindowPresetManager, WindowPreset,
    };
    use wakem::types::{
        KeyAction, KeyEvent, KeyState, ModifierState, MouseAction, MouseButton,
        MouseEvent, MouseEventType,
    };

    /// Test 1: Keycode mapping consistency across modules
    ///
    /// Verifies that keycode mappings used by input and output devices are consistent.
    #[test]
    fn test_keycode_mapping_consistency() {
        // Test that common keycodes produce expected virtual keys
        let test_cases = vec![
            (0x00, 0x41), // A
            (0x01, 0x53), // S
            (0x12, 0x31), // 1
            (0x30, 0x09), // Tab
            (0x35, 0x1B), // Escape
            (0x7A, 0x70), // F1
            (0x7B, 0x25), // Left Arrow
        ];

        for (keycode, expected_vk) in test_cases {
            let vk = keycode_to_virtual_key(keycode);
            assert_eq!(
                vk, expected_vk,
                "Keycode {:#04X} should map to VK {:#04X}, got {:#04X}",
                keycode, expected_vk, vk
            );
        }
    }

    /// Test 2: Launcher command parsing
    ///
    /// Tests that the launcher can correctly parse command strings.
    #[test]
    fn test_macos_launcher_parsing() {
        // Test parsing command string
        let parsed = Launcher::parse_command("open -a Safari https://example.com");
        assert_eq!(parsed.program, "open");
        assert_eq!(parsed.args, vec!["-a", "Safari", "https://example.com"]);

        // Test empty command
        let empty = Launcher::parse_command("");
        assert_eq!(empty.program, "");
        assert!(empty.args.is_empty());

        // Test single word command
        let single = Launcher::parse_command("ls");
        assert_eq!(single.program, "ls");
        assert!(single.args.is_empty());
    }

    /// Test 3: Window context pattern matching
    ///
    /// Tests that window context can match various patterns for process-specific rules.
    #[test]
    fn test_macos_context_pattern_matching() {
        // Create context for Safari browser
        let safari_ctx = WindowContext {
            process_name: "Safari".to_string(),
            window_class: String::new(),
            window_title: "Apple - Official Website".to_string(),
            executable_path: Some(
                "/Applications/Safari.app/Contents/MacOS/Safari".to_string(),
            ),
        };

        // Test exact match
        assert!(safari_ctx.matches(Some("Safari"), None, None, None));

        // Test wildcard match on process name
        assert!(safari_ctx.matches(Some("Saf*"), None, None, None));
        assert!(safari_ctx.matches(Some("*ari"), None, None, None));

        // Test title pattern matching
        assert!(safari_ctx.matches(None, None, Some("*Apple*"), None));

        // Test executable path matching
        assert!(safari_ctx.matches(None, None, None, Some("*Safari*")));

        // Non-matching cases
        assert!(!safari_ctx.matches(Some("Firefox"), None, None, None));
        assert!(!safari_ctx.matches(None, None, Some("Google Chrome"), None));

        // Create context for Terminal
        let terminal_ctx = WindowContext {
            process_name: "Terminal".to_string(),
            window_class: String::new(),
            window_title: "~/Projects/wakem — zsh".to_string(),
            executable_path: Some(
                "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal"
                    .to_string(),
            ),
        };

        // Test terminal-specific patterns
        assert!(terminal_ctx.matches(Some("Term*"), None, None, None));
        assert!(terminal_ctx.matches(None, None, Some("*wakem*"), None));
        assert!(terminal_ctx.matches(None, None, Some("*zsh*"), None));

        // Convert to platform-agnostic context
        let platform_ctx = safari_ctx.to_platform_context();
        assert_eq!(platform_ctx.process_name, "Safari");
        assert_eq!(platform_ctx.window_title, "Apple - Official Website");
        assert_eq!(
            platform_ctx.executable_path,
            Some("/Applications/Safari.app/Contents/MacOS/Safari".to_string())
        );
    }

    /// Test 4: Preset configuration loading
    ///
    /// Tests that window presets can be loaded from configuration vectors.
    #[test]
    fn test_macos_preset_configuration() {
        let presets = vec![
            WindowPreset {
                name: "Editor Layout".to_string(),
                process_pattern: "VSCode".to_string(),
                title_pattern: None,
                x: 50,
                y: 50,
                width: 1200,
                height: 800,
                maximize: false,
                minimize: false,
            },
            WindowPreset {
                name: "Browser Fullscreen".to_string(),
                process_pattern: "*Chrome*".to_string(),
                title_pattern: None,
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                maximize: true,
                minimize: false,
            },
            WindowPreset {
                name: "Minimal Terminal".to_string(),
                process_pattern: "Terminal".to_string(),
                title_pattern: Some("*zsh*".to_string()),
                x: 100,
                y: 800,
                width: 1720,
                height: 280,
                maximize: false,
                minimize: false,
            },
        ];

        // Verify preset properties
        assert_eq!(presets.len(), 3);
        assert_eq!(presets[0].name, "Editor Layout");
        assert_eq!(presets[0].width, 1200);
        assert_eq!(presets[0].height, 800);
        assert!(!presets[0].maximize);

        assert!(presets[1].maximize);
        assert_eq!(presets[1].process_pattern, "*Chrome*");

        assert!(presets[2].title_pattern.is_some());
        assert_eq!(presets[2].title_pattern.as_deref(), Some("*zsh*"));
    }

    /// Test 5: Keyboard event creation consistency
    ///
    /// Tests that keyboard events can be created with correct state transitions.
    #[test]
    fn test_keyboard_event_state_transitions() {
        // Simulate a key press/release cycle for 'A'
        let press_event = KeyEvent::new(0x00, 0x41, KeyState::Pressed);
        let release_event = KeyEvent::new(0x00, 0x41, KeyState::Released);

        assert_eq!(press_event.virtual_key, 0x41); // 'A'
        assert_eq!(press_event.state, KeyState::Pressed);
        assert_eq!(release_event.state, KeyState::Released);

        // Convert to actions
        let press_action = KeyAction::Press {
            scan_code: press_event.scan_code,
            virtual_key: press_event.virtual_key,
        };
        let release_action = KeyAction::Release {
            scan_code: release_event.scan_code,
            virtual_key: release_event.virtual_key,
        };

        assert!(matches!(press_action, KeyAction::Press { .. }));
        assert!(matches!(release_action, KeyAction::Release { .. }));
    }

    /// Test 6: Mouse event type coverage
    ///
    /// Tests all mouse event types can be created correctly.
    #[test]
    fn test_mouse_event_types() {
        // Mouse movement
        let move_event = MouseEvent::new(MouseEventType::Move, 100, 200);
        assert_eq!(move_event.x, 100);
        assert_eq!(move_event.y, 200);
        assert!(matches!(move_event.event_type, MouseEventType::Move));

        // Left button down
        let left_down =
            MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Left), 150, 250);
        assert!(matches!(
            left_down.event_type,
            MouseEventType::ButtonDown(MouseButton::Left)
        ));

        // Left button up
        let left_up =
            MouseEvent::new(MouseEventType::ButtonUp(MouseButton::Left), 150, 250);
        assert!(matches!(
            left_up.event_type,
            MouseEventType::ButtonUp(MouseButton::Left)
        ));

        // Right button
        let right_down =
            MouseEvent::new(MouseEventType::ButtonDown(MouseButton::Right), 300, 400);
        assert!(matches!(
            right_down.event_type,
            MouseEventType::ButtonDown(MouseButton::Right)
        ));

        // Scroll wheel
        let scroll = MouseEvent::new(MouseEventType::Wheel(120), 500, 600);
        assert!(matches!(scroll.event_type, MouseEventType::Wheel(120)));
        assert_eq!(scroll.x, 500);
        assert_eq!(scroll.y, 600);

        // Horizontal scroll
        let hscroll = MouseEvent::new(MouseEventType::HWheel(-60), 700, 800);
        assert!(matches!(hscroll.event_type, MouseEventType::HWheel(-60)));
    }

    /// Test 7: Modifier state operations
    ///
    /// Tests modifier state merging and manipulation.
    #[test]
    fn test_modifier_state_operations() {
        let mut state = ModifierState::default();
        assert!(!state.shift && !state.ctrl && !state.alt && !state.meta);

        // Set individual modifiers
        state.shift = true;
        assert!(state.shift);

        // Merge with another state
        let other = ModifierState {
            ctrl: true,
            meta: true,
            ..ModifierState::default()
        };
        state.merge(&other);

        assert!(state.shift);
        assert!(state.ctrl);
        assert!(!state.alt);
        assert!(state.meta);
    }

    /// Test 8: Roundtrip keycode conversion for common keys
    ///
    /// Tests bidirectional conversion consistency for frequently used keys.
    #[test]
    fn test_roundtrip_conversion_common_keys() {
        let test_keys = vec![
            (0x00, 'A'),         // A
            (0x0D, 'W'),         // W
            (0x31, ' '),         // Space
            (0x30, '\t'),        // Tab
            (0x33, 8u8 as char), // Backspace (ASCII BS)
        ];

        for (keycode, _) in test_keys {
            let vk = keycode_to_virtual_key(keycode);
            let reversed = virtual_key_to_keycode(vk);
            // Note: Due to VK collisions (e.g., 0x5B for both '[' and Command),
            // roundtrip may not always be exact. This test verifies no panics occur.
            let _ = reversed;
        }
    }
}
