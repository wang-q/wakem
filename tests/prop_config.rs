// Property tests - configuration-related property tests

use proptest::prelude::*;
use wakem::config::{parse_key, wildcard_match};
use wakem::types::*;

/// Wildcard match reflexive
#[test]
fn test_wildcard_match_reflexive() {
    assert!(wildcard_match("any_string", "any_string"));
    assert!(wildcard_match("", ""));
    assert!(wildcard_match("*", "*"));
    assert!(wildcard_match("?", "?"));
}

// Empty pattern only matches empty string
proptest! {
    #[test]
    fn empty_pattern_only_matches_empty(s in "\\PC*") {
        let matches_empty = wildcard_match(&s, "");
        prop_assert_eq!(matches_empty, s.is_empty());
    }
}

// Star pattern matches any non-empty string
proptest! {
    #[test]
    fn star_pattern_matches_anything(s in "\\PC*") {
        if !s.is_empty() {
            prop_assert!(wildcard_match(&s, "*"));
        }
    }
}

// Prefix matching property
proptest! {
    #[test]
    fn prefix_star_matches_prefix(
        prefix in "[a-zA-Z0-9_]{1,20}",
        suffix in "\\PC*"
    ) {
        let full_string = format!("{}{}", prefix, suffix);
        let pattern = format!("{}*", prefix);
        prop_assert!(wildcard_match(&full_string, &pattern));
    }
}

// Suffix matching property
proptest! {
    #[test]
    fn suffix_star_matches_suffix(
        prefix in "\\PC*",
        suffix in "[a-zA-Z0-9_]{1,20}"
    ) {
        let full_string = format!("{}{}", prefix, suffix);
        let pattern = format!("*{}", suffix);
        prop_assert!(wildcard_match(&full_string, &pattern));
    }
}

// Question mark matches single character exactly
proptest! {
    #[test]
    fn question_mark_matches_single_char(c in "\\PC") {
        let single_char = c.to_string();
        if single_char.len() == single_char.to_lowercase().len() {
            prop_assert!(wildcard_match(&single_char, "?"));
            prop_assert!(!wildcard_match("", "?"));
            let two_chars = format!("{}x", c);
            prop_assert!(!wildcard_match(&two_chars, "?"));
        }
    }
}

// Letter key names are case-insensitive
proptest! {
    #[test]
    fn letter_key_case_insensitive(ch in "[a-z]") {
        let lower = ch.to_string();
        let upper = ch.to_ascii_uppercase().to_string();

        let lower_result = parse_key(&lower);
        let upper_result = parse_key(&upper);

        match (lower_result, upper_result) {
            (Ok(lower_code), Ok(upper_code)) => {
                prop_assert_eq!(lower_code, upper_code);
            }
            (Err(_), Err(_)) => {}
            _ => {
                panic!("Inconsistent key parsing");
            }
        }
    }
}

// Digit key names return valid codes
proptest! {
    #[test]
    fn digit_keys_have_valid_codes(digit in 0u8..=9) {
        let name = digit.to_string();
        let result = parse_key(&name);
        prop_assert!(result.is_ok());

        if let Ok((scan_code, virtual_key)) = result {
            prop_assert!(scan_code > 0 && scan_code <= 0xFF);
            prop_assert!(virtual_key > 0 && virtual_key <= 0xFF);
        }
    }
}

// KeyEvent injected flag consistency
proptest! {
    #[test]
    fn key_event_injected_consistency(
        scan_code in 0u16..=0xFFFF,
        virtual_key in 0u16..=0xFFFF,
        injected in any::<bool>()
    ) {
        let event = KeyEvent {
            scan_code,
            virtual_key,
            state: KeyState::Pressed,
            modifiers: ModifierState::default(),
            device_type: DeviceType::Keyboard,
            timestamp: 0,
            is_injected: injected,
        };

        prop_assert_eq!(event.is_injected, injected);
    }
}

// Multiple star pattern equivalence
proptest! {
    #[test]
    fn multiple_stars_equivalence(s in "\\PC*") {
        if !s.is_empty() {
            let match_single = wildcard_match(&s, "*");
            let match_double = wildcard_match(&s, "**");
            let match_triple = wildcard_match(&s, "***");

            prop_assert_eq!(match_single, match_double);
            prop_assert_eq!(match_double, match_triple);
        }
    }
}

// Unicode character handling
proptest! {
    #[test]
    fn unicode_characters_handling(s in "\\PC{1,50}") {
        let result = std::panic::catch_unwind(|| {
            let _ = wildcard_match(&s, "*");
            let _ = wildcard_match("*", &s);
            let _ = wildcard_match(&s, &s);
        });
        prop_assert!(result.is_ok());
    }
}
