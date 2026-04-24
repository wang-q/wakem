// 属性测试 - 配置相关属性测试

use proptest::prelude::*;
use wakem::config::{parse_key, wildcard_match};
use wakem::types::*;

/// 通配符匹配自反性
#[test]
fn test_wildcard_match_reflexive() {
    assert!(wildcard_match("any_string", "any_string"));
    assert!(wildcard_match("", ""));
    assert!(wildcard_match("*", "*"));
    assert!(wildcard_match("?", "?"));
}

/// 空模式只匹配空字符串
proptest! {
    #[test]
    fn empty_pattern_only_matches_empty(s in "\\PC*") {
        let matches_empty = wildcard_match(&s, "");
        prop_assert_eq!(matches_empty, s.is_empty());
    }
}

/// 星号模式匹配任何非空字符串
proptest! {
    #[test]
    fn star_pattern_matches_anything(s in "\\PC*") {
        if !s.is_empty() {
            prop_assert!(wildcard_match(&s, "*"));
        }
    }
}

/// 前缀匹配属性
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

/// 后缀匹配属性
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

/// 问号精确匹配单个字符
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

/// 字母键名大小写不敏感
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

/// 数字键名返回有效代码
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

/// ModifierState 合并幂等性
proptest! {
    #[test]
    fn modifier_state_merge_idempotent(
        shift1 in any::<bool>(),
        ctrl1 in any::<bool>(),
        alt1 in any::<bool>(),
        meta1 in any::<bool>()
    ) {
        let state1 = ModifierState {
            shift: shift1,
            ctrl: ctrl1,
            alt: alt1,
            meta: meta1,
        };

        let mut merged_once = ModifierState::new();
        merged_once.merge(&state1);

        let mut merged_twice = ModifierState::new();
        merged_twice.merge(&state1);
        merged_twice.merge(&state1);

        prop_assert_eq!(merged_once.shift, merged_twice.shift);
        prop_assert_eq!(merged_once.ctrl, merged_twice.ctrl);
        prop_assert_eq!(merged_once.alt, merged_twice.alt);
        prop_assert_eq!(merged_once.meta, merged_twice.meta);
    }
}

/// ModifierState 合并交换律
proptest! {
    #[test]
    fn modifier_state_merge_commutative(
        s1 in any::<bool>(), c1 in any::<bool>(), a1 in any::<bool>(), m1 in any::<bool>(),
        s2 in any::<bool>(), c2 in any::<bool>(), a2 in any::<bool>(), m2 in any::<bool>()
    ) {
        let state_a = ModifierState { shift: s1, ctrl: c1, alt: a1, meta: m1 };
        let state_b = ModifierState { shift: s2, ctrl: c2, alt: a2, meta: m2 };

        let mut ab = ModifierState::new();
        ab.merge(&state_a);
        ab.merge(&state_b);

        let mut ba = ModifierState::new();
        ba.merge(&state_b);
        ba.merge(&state_a);

        prop_assert_eq!(ab.shift, ba.shift);
        prop_assert_eq!(ab.ctrl, ba.ctrl);
        prop_assert_eq!(ab.alt, ba.alt);
        prop_assert_eq!(ab.meta, ba.meta);
    }
}

/// KeyEvent 注入标记一致性
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

/// 多星号模式等价性
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

/// Unicode 字符处理
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
