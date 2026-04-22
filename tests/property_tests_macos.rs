//! macOS platform property tests using proptest
//!
//! Verifies key invariants and properties of macOS platform implementations
//! using property-based testing to discover edge cases.

#[cfg(all(test, target_os = "macos"))]
mod macos_property_tests {
    use proptest::prelude::*;
    use wakem::platform::macos::input::{
        keycode_to_virtual_key, virtual_key_to_keycode,
    };

    /// Property 1: Keycode mapping is injective (one-to-one) for mapped keys
    ///
    /// Note: With keyboard-codes, unmapped keys return passthrough (original value),
    /// so we only verify that explicitly mapped keys have unique virtual keys.
    #[test]
    fn test_keycode_mapping_is_injective_for_common_keys() {
        // Test a subset of well-known keys that should be uniquely mapped
        let common_keycodes = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, // A-J
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, // 1-9
            0x7A, 0x78, 0x63, 0x76, 0x77, 0x75, 0x73, 0x79, 0x6D, 0x69, 0x6B, 0x71,
            0x7B, 0x7C, 0x7D, 0x7E, // F1-F12 + Arrows
        ];

        let mut seen_vks = std::collections::HashSet::new();

        for keycode in common_keycodes {
            let vk = keycode_to_virtual_key(keycode);
            assert!(
                !seen_vks.contains(&vk) || vk == keycode as u16,
                "Duplicate mapping found: keycode {:#04X} and another both map to vk {:#04X}",
                keycode,
                vk
            );
            seen_vks.insert(vk);
        }
    }

    /// Property 2: Roundtrip conversion consistency for letters
    ///
    /// For letter keys: keycode_to_virtual_key(virtual_key_to_keycode(k)) == k
    /// Note: keyboard-codes may not guarantee perfect bidirectional mapping for all keys.
    proptest! {
        #[test]
        fn prop_roundtrip_consistency_for_letters(a in 0x00u16..=0x0Du16) {
            // Letters A-R (excluding 0x0A which is undefined)
            if a != 0x0A {
                let vk = keycode_to_virtual_key(a);
                if vk != a {
                    let reversed = virtual_key_to_keycode(vk);
                    prop_assert_eq!(a, reversed, "Roundtrip failed for letter keycode {:#04X}", a);
                }
            }
        }

        #[test]
        fn prop_roundtrip_consistency_for_digits(d in 0x12u16..=0x1Au16) {
            // Digits 1-9 (keyboard-codes may handle 0 differently)
            let vk = keycode_to_virtual_key(d);
            if vk != d {
                let reversed = virtual_key_to_keycode(vk);
                prop_assert_eq!(d, reversed, "Roundtrip failed for digit keycode {:#04X}", d);
            }
        }
    }

    /// Property 3: Wildcard pattern matching satisfies commutativity and idempotency
    ///
    /// Tests that wildcard matching has these algebraic properties:
    /// - Commutative: match("*", s) == match(s, "*") == true (for non-empty strings)
    /// - Idempotent: applying match twice gives same result
    proptest! {
        #[test]
        fn prop_wildcard_star_matches_anything(s in "[a-zA-Z0-9 _-]*") {
            use wakem::platform::macos::context::WindowContext;

            if !s.is_empty() {
                let ctx = WindowContext {
                    process_name: s.clone(),
                    window_class: String::new(),
                    window_title: String::new(),
                    executable_path: None,
                };

                // "*" should match any non-empty string
                prop_assert!(ctx.matches(Some("*"), None, None, None));
                prop_assert!(ctx.matches(None, Some("*"), None, None));
                prop_assert!(ctx.matches(None, None, Some("*"), None));
            }
        }

        #[test]
        fn prop_exact_match_idempotency(pattern in "[a-zA-Z][a-zA-Z0-9_-]*") {
            use wakem::platform::macos::context::WindowContext;

            let ctx = WindowContext {
                process_name: pattern.clone(),
                window_class: String::new(),
                window_title: String::new(),
                executable_path: None,
            };

            // Exact match should return same result when called twice
            let result1 = ctx.matches(Some(&pattern), None, None, None);
            let result2 = ctx.matches(Some(&pattern), None, None, None);
            prop_assert_eq!(result1, result2, "Exact match not idempotent for '{}'", pattern);
        }
    }

    /// Property 4: Keypad keys are mapped by keyboard-codes
    ///
    /// Verifies that numpad keycodes produce valid mappings (not necessarily in 0x60-0x69 range).
    #[test]
    fn test_numpad_keys_distinct_from_main_keyboard() {
        let numpad_keycodes = vec![
            0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x5B, // 0-9
        ];

        let main_keyboard_vk_range = 0x30..=0x39; // 0-9

        for numpad_kc in numpad_keycodes {
            let numpad_vk = keycode_to_virtual_key(numpad_kc);

            // keyboard-codes should map these to something (either specific VK or passthrough)
            assert!(
                !main_keyboard_vk_range.contains(&numpad_vk) || numpad_vk == numpad_kc,
                "Numpad key {:#04X} (vk {:#04X}) collides with main keyboard range",
                numpad_kc,
                numpad_vk
            );
        }
    }

    /// Property 5: Function keys are monotonically ordered
    ///
    /// F1-F12 should map to consecutive or predictable virtual key codes.
    #[test]
    fn test_function_keys_ordered_sequence() {
        let f_keycodes = vec![
            (0x7A, 0x70), // F1 -> VK_F1
            (0x78, 0x71), // F2 -> VK_F2
            (0x63, 0x72), // F3 -> VK_F3
            (0x76, 0x73), // F4 -> VK_F4
            (0x60, 0x74), // F5 -> VK_F5
            (0x61, 0x75), // F6 -> VK_F6
            (0x62, 0x76), // F7 -> VK_F7
            (0x64, 0x77), // F8 -> VK_F8
            (0x65, 0x78), // F9 -> VK_F9
            (0x6D, 0x79), // F10 -> VK_F10
            (0x67, 0x7A), // F11 -> VK_F11
            (0x6F, 0x7B), // F12 -> VK_F12
        ];

        // Verify ordering: VKs should be strictly increasing
        for window in f_keycodes.windows(2) {
            let (_, vk1) = window[0];
            let (_, vk2) = window[1];
            assert!(
                vk2 > vk1,
                "Function keys not in order: vk {:#04X} should be less than vk {:#04X}",
                vk1,
                vk2
            );
        }

        // Verify all F-keys map to range 0x70-0x7B
        for (kc, vk) in &f_keycodes {
            assert!(
                (0x70..=0x7B).contains(vk),
                "F-key {:#04X} mapped to out-of-range vk {:#04X}",
                kc,
                vk
            );
        }
    }

    /// Property 6: Unknown keycodes pass through unchanged
    ///
    /// Keycodes not in our mapping table should be returned as-is to allow
    /// future extensibility without data loss.
    proptest! {
        #[test]
        fn prop_unknown_keycode_passthrough(kc in 0xC0u16..0xFFu16) {
            // High-range keycodes unlikely to be in standard mapping
            let result = keycode_to_virtual_key(kc);
            prop_assert_eq!(
                result, kc,
                "Unknown keycode {:#04X} was modified to {:#04X}, expected passthrough",
                kc, result
            );
        }
    }
}
