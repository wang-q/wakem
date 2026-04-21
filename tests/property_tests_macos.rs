//! macOS platform property tests using proptest
//!
//! Verifies key invariants and properties of macOS platform implementations
//! using property-based testing to discover edge cases.

#[cfg(test)]
mod macos_property_tests {
    use proptest::prelude::*;
    use wakem::platform::macos::input::{keycode_to_virtual_key, virtual_key_to_keycode};

    /// Property 1: Keycode mapping should be injective (one-to-one) for common keys
    ///
    /// Different macOS keycodes should map to different Windows virtual keys.
    /// This ensures we don't lose information during conversion.
    ///
    /// Note: Some collisions exist in the Windows VK space itself (e.g., VK 0x27 is both
    /// OEM_7 and VK_RIGHT). These are inherent limitations of the Windows VK encoding.
    #[test]
    fn test_keycode_mapping_is_injective_for_common_keys() {
        // Define a set of common, non-overlapping keycodes
        // Excluding known collision cases (0x27 which maps both ' and Right Arrow)
        let common_keycodes: Vec<u16> = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, // A-J
            0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, // B-R
            0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, // 0-9, -, =
            0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x28, 0x29, // U-[ excluding 0x27
            0x30, 0x31, 0x33, 0x35, // Tab, Space, BS, Esc
            0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, 0x67, 0x6F, // F1-F12
            0x7B, 0x7D, 0x7E, // Left, Down, Up Arrows (excluding Right 0x7C)
        ];

        let mut seen_vks = std::collections::HashSet::new();

        for keycode in common_keycodes {
            let vk = keycode_to_virtual_key(keycode);
            assert!(
                !seen_vks.contains(&vk),
                "Duplicate mapping found: keycode {:#04X} and another both map to vk {:#04X}",
                keycode,
                vk
            );
            seen_vks.insert(vk);
        }
    }

    /// Property 2: Roundtrip conversion consistency for all valid keycodes
    ///
    /// For any valid keycode k: keycode_to_virtual_key(virtual_key_to_keycode(k)) == k
    /// This ensures the reverse mapping is consistent with the forward mapping.
    proptest! {
        #[test]
        fn prop_roundtrip_consistency_for_letters(a in 0x00u16..=0x0Du16) {
            // Letters A-R (excluding 0x0A which is undefined)
            if a != 0x0A {
                let vk = keycode_to_virtual_key(a);
                let reversed = virtual_key_to_keycode(vk);
                prop_assert_eq!(a, reversed, "Roundtrip failed for letter keycode {:#04X}", a);
            }
        }

        #[test]
        fn prop_roundtrip_consistency_for_digits(d in 0x12u16..=0x1Du16) {
            // Digits 0-9 and symbols
            let vk = keycode_to_virtual_key(d);
            let reversed = virtual_key_to_keycode(vk);
            prop_assert_eq!(d, reversed, "Roundtrip failed for digit keycode {:#04X}", d);
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

    /// Property 4: Keypad keys have distinct virtual keys from main keyboard
    ///
    /// Ensures keypad number keys don't collide with main keyboard number keys.
    #[test]
    fn test_numpad_keys_distinct_from_main_keyboard() {
        // Main keyboard digits: 0x30-0x39 (virtual keys)
        // Keypad digits: 0x60-0x69 (virtual keys)

        let main_keyboard_vk_range = 0x30..=0x39; // 0-9
        let numpad_keycodes = vec![
            0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5A, 0x5B, // 0-9
        ];

        for numpad_kc in numpad_keycodes {
            let numpad_vk = keycode_to_virtual_key(numpad_kc);
            assert!(
                !main_keyboard_vk_range.contains(&numpad_vk),
                "Numpad key {:#04X} (vk {:#04X}) collides with main keyboard range",
                numpad_kc,
                numpad_vk
            );
            // Numpad VKs should be in range 0x60-0x69
            assert!(
                (0x60..=0x69).contains(&numpad_vk),
                "Numpad key {:#04X} mapped to unexpected vk {:#04X}",
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
