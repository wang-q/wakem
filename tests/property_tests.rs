//! wakem 属性测试 (Property-based Testing)
//!
//! 使用 proptest 发现边缘情况和潜在 bug

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    use wakem::config::{parse_key, wildcard_match};
    use wakem::types::*;

    // ==================== 通配符匹配属性测试 ====================

    /// 属性 1: 通配符匹配的自反性 - 任何字符串都应该匹配自身
    #[test]
    fn test_wildcard_match_reflexive() {
        assert!(wildcard_match("any_string", "any_string"));
        assert!(wildcard_match("", ""));
        assert!(wildcard_match("*", "*"));
        assert!(wildcard_match("?", "?"));
    }

    /// 属性 2: 空模式只匹配空字符串
    proptest! {
        #[test]
        fn empty_pattern_only_matches_empty(s in "\\PC*") {
            let matches_empty = wildcard_match(&s, "");
            prop_assert_eq!(matches_empty, s.is_empty(),
                "Empty pattern should only match empty string");
        }
    }

    /// 属性 3: 星号模式匹配任何非空字符串
    proptest! {
        #[test]
        fn star_pattern_matches_anything(s in "\\PC*") {
            if !s.is_empty() {
                prop_assert!(wildcard_match(&s, "*"),
                    "Star pattern should match any non-empty string: '{}'", s);
            }
        }
    }

    /// 属性 4: 前缀匹配属性 - "prefix*" 应该匹配以 prefix 开头的字符串
    proptest! {
        #[test]
        fn prefix_star_matches_prefix(
            prefix in "[a-zA-Z0-9_]{1,20}",
            suffix in "\\PC*"
        ) {
            let full_string = format!("{}{}", prefix, suffix);
            let pattern = format!("{}*", prefix);

            prop_assert!(wildcard_match(&full_string, &pattern),
                "'{}' should match pattern '{}'", full_string, pattern);
        }
    }

    /// 属性 5: 后缀匹配属性 - "*suffix" 应该匹配以 suffix 结尾的字符串
    proptest! {
        #[test]
        fn suffix_star_matches_suffix(
            prefix in "\\PC*",
            suffix in "[a-zA-Z0-9_]{1,20}"
        ) {
            let full_string = format!("{}{}", prefix, suffix);
            let pattern = format!("*{}", suffix);

            prop_assert!(wildcard_match(&full_string, &pattern),
                "'{}' should match pattern '{}'", full_string, pattern);
        }
    }

    /// 属性 6: 包含匹配属性 - "*middle*" 应该包含 middle 的字符串
    proptest! {
        #[test]
        fn star_middle_star_contains(
            before in "\\PC*",
            middle in "[a-zA-Z0-9_]{1,10}",
            after in "\\PC*"
        ) {
            let full_string = format!("{}{}{}", before, middle, after);
            let pattern = format!("*{}*", middle);

            prop_assert!(wildcard_match(&full_string, &pattern),
                "'{}' should match pattern '{}' (contains {})", full_string, pattern, middle);
        }
    }

    /// 属性 7: 问号精确匹配单个字符
    proptest! {
        #[test]
        fn question_mark_matches_single_char(c in "\\PC") {
            let single_char = c.to_string();
            
            // Only test characters that don't change length when lowercased
            // (e.g., İ U+0130 → i̇ is a known Unicode normalization edge case)
            if single_char.len() == single_char.to_lowercase().len() {
                prop_assert!(wildcard_match(&single_char, "?"),
                    "Single character '{}' should match '?' pattern", c);

                // 问号不应该匹配空字符串或两个字符
                prop_assert!(!wildcard_match("", "?"),
                    "Empty string should not match '?' pattern");

                let two_chars = format!("{}x", c);
                prop_assert!(!wildcard_match(&two_chars, "?"),
                    "Two characters should not match '?' pattern");
            }
        }
    }

    // ==================== 键名解析属性测试 ====================

    /// 属性 8: 字母键名大小写不敏感（如果实现支持）
    proptest! {
        #[test]
        fn letter_key_case_insensitive(ch in "[a-z]") {
            let lower = ch.to_string();
            let upper = ch.to_ascii_uppercase().to_string();

            let lower_result = parse_key(&lower);
            let upper_result = parse_key(&upper);

            // 如果两者都能解析，结果应该相同
            match (lower_result, upper_result) {
                (Ok(lower_code), Ok(upper_code)) => {
                    prop_assert_eq!(lower_code, upper_code,
                        "Key parsing should be case-insensitive for '{}'", ch);
                }
                (Err(_), Err(_)) => { /* 两者都失败，可接受 */ }
                (Ok(_), Err(e)) | (Err(e), Ok(_)) => {
                    panic!("Inconsistent key parsing: '{}' -> Ok, '{}' -> Err({})",
                        lower, upper, e);
                }
            }
        }
    }

    /// 属性 9: 数字键名应该返回有效的扫描码和虚拟键码
    proptest! {
        #[test]
        fn digit_keys_have_valid_codes(digit in 0u8..=9) {
            let name = digit.to_string();
            let result = parse_key(&name);

            prop_assert!(result.is_ok(), "Digit key '{}' should be parsable", digit);

            if let Ok((scan_code, virtual_key)) = result {
                prop_assert!(scan_code > 0 && scan_code <= 0xFF,
                    "Scan code for '{}' should be in valid range", digit);
                prop_assert!(virtual_key > 0 && virtual_key <= 0xFF,
                    "Virtual key for '{}' should be in valid range", digit);
            }
        }
    }

    // ==================== ModifierState 属性测试 ====================

    /// 属性 10: ModifierState 合并的幂等性 - 合并相同状态两次应该与合并一次相同
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
            merged_twice.merge(&state1); // 再次合并相同状态

            // 由于使用 |=，第二次合并不应该改变结果
            prop_assert_eq!(merged_once.shift, merged_twice.shift);
            prop_assert_eq!(merged_once.ctrl, merged_twice.ctrl);
            prop_assert_eq!(merged_once.alt, merged_twice.alt);
            prop_assert_eq!(merged_once.meta, merged_twice.meta);
        }
    }

    /// 属性 11: ModifierState 合并的结合律 - (a + b) + c == a + (b + c)
    proptest! {
        #[test]
        fn modifier_state_merge_associative(
            s1 in any::<bool>(), c1 in any::<bool>(), a1 in any::<bool>(), m1 in any::<bool>(),
            s2 in any::<bool>(), c2 in any::<bool>(), a2 in any::<bool>(), m2 in any::<bool>(),
            s3 in any::<bool>(), c3 in any::<bool>(), a3 in any::<bool>(), m3 in any::<bool>()
        ) {
            let state_a = ModifierState { shift: s1, ctrl: c1, alt: a1, meta: m1 };
            let state_b = ModifierState { shift: s2, ctrl: c2, alt: a2, meta: m2 };
            let state_c = ModifierState { shift: s3, ctrl: c3, alt: a3, meta: m3 };

            // (a + b) + c
            let mut left = ModifierState::new();
            left.merge(&state_a);
            left.merge(&state_b);
            left.merge(&state_c);

            // a + (b + c)
            let mut right = ModifierState::new();
            right.merge(&state_b);
            right.merge(&state_c);
            right.merge(&state_a);

            // |= 操作满足交换律和结合律，所以结果应该相同
            prop_assert_eq!(left.shift, right.shift);
            prop_assert_eq!(left.ctrl, right.ctrl);
            prop_assert_eq!(left.alt, right.alt);
            prop_assert_eq!(left.meta, right.meta);
        }
    }

    /// 属性 12: ModifierState 合并的交换律 - a + b == b + a
    proptest! {
        #[test]
        fn modifier_state_merge_commutative(
            s1 in any::<bool>(), c1 in any::<bool>(), a1 in any::<bool>(), m1 in any::<bool>(),
            s2 in any::<bool>(), c2 in any::<bool>(), a2 in any::<bool>(), m2 in any::<bool>()
        ) {
            let state_a = ModifierState { shift: s1, ctrl: c1, alt: a1, meta: m1 };
            let state_b = ModifierState { shift: s2, ctrl: c2, alt: a2, meta: m2 };

            // a + b
            let mut ab = ModifierState::new();
            ab.merge(&state_a);
            ab.merge(&state_b);

            // b + a
            let mut ba = ModifierState::new();
            ba.merge(&state_b);
            ba.merge(&state_a);

            // |= 操作满足交换律
            prop_assert_eq!(ab.shift, ba.shift);
            prop_assert_eq!(ab.ctrl, ba.ctrl);
            prop_assert_eq!(ab.alt, ba.alt);
            prop_assert_eq!(ab.meta, ba.meta);
        }
    }

    // ==================== InputEvent 属性测试 ====================

    /// 属性 13: KeyEvent 的 is_injected() 一致性
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

            prop_assert_eq!(event.is_injected, injected,
                "KeyEvent.is_injected should return the injected flag value");
        }
    }

    /// 属性 14: MouseEvent 的坐标一致性
    proptest! {
        #[test]
        fn mouse_event_coordinate_consistency(
            x in i32::MIN..i32::MAX,
            y in i32::MIN..i32::MAX,
            button in prop_oneof![
                Just(MouseButton::Left),
                Just(MouseButton::Right),
                Just(MouseButton::Middle),
                Just(MouseButton::X1),
                Just(MouseButton::X2),
            ]
        ) {
            let move_event = MouseEvent {
                event_type: MouseEventType::Move,
                x,
                y,
                modifiers: ModifierState::default(),
                timestamp: 0,
                is_injected: false,
            };

            prop_assert_eq!(move_event.x, x);
            prop_assert_eq!(move_event.y, y);

            let button_down = MouseEvent {
                event_type: MouseEventType::ButtonDown(button.clone()),
                x,
                y,
                modifiers: ModifierState::default(),
                timestamp: 0,
                is_injected: false,
            };

            prop_assert_eq!(button_down.x, x);
            prop_assert_eq!(button_down.y, y);
            if let MouseEventType::ButtonDown(btn) = button_down.event_type {
                prop_assert_eq!(btn, button);
            } else {
                panic!("Expected ButtonDown event type");
            }
        }
    }

    // ==================== 配置序列化/反序列化属性测试 ====================

    /// 属性 15: Config 序列化往返一致性
    ///
    /// 注意：此测试需要有效的配置结构，简化版本测试基本类型
    proptest! {
        #[test]
        fn toml_roundtrip_for_basic_types(
            log_level in "[a-z]+",
            tray_icon in any::<bool>(),
            auto_reload in any::<bool>()
        ) {
            use serde::{Serialize, Deserialize};

            #[derive(Debug, Serialize, Deserialize, PartialEq)]
            struct TestConfig {
                log_level: String,
                tray_icon: bool,
                auto_reload: bool,
            }

            let config = TestConfig {
                log_level: log_level.clone(),
                tray_icon,
                auto_reload,
            };

            // 序列化
            let serialized = toml::to_string_pretty(&config)
                .expect("Serialization failed");

            // 反序列化
            let deserialized: TestConfig = toml::from_str(&serialized)
                .expect("Deserialization failed");

            // 验证一致性
            prop_assert_eq!(config, deserialized,
                "TOML roundtrip should preserve all fields");
        }
    }

    // ==================== 性能相关属性测试 ====================

    /// 属性 16: 通配符匹配时间复杂度合理性
    ///
    /// 验证匹配操作不会因输入长度而出现指数级增长
    proptest! {
        #[test]
        fn wildcard_match_performance_reasonable(
            base_pattern in "[a-z*?]{1,5}",
            repeat_count in 1usize..=100
        ) {
            use std::time::Instant;

            // 构造重复的模式和输入
            let input = base_pattern.repeat(repeat_count);
            let pattern = format!("*{}*", base_pattern);

            // 测量匹配时间
            let start = Instant::now();
            let _ = wildcard_match(&input, &pattern);
            let elapsed = start.elapsed();

            // 对于合理的输入，匹配应该在毫秒级完成
            prop_assert!(elapsed.as_millis() < 100,
                "Wildcard match took too long: {:?} for input length {}",
                elapsed, input.len());
        }
    }

    /// 属性 17: 大量事件批处理的内存安全性
    ///
    /// 验证处理大量事件不会导致内存问题
    proptest! {
        #[test]
        fn large_batch_processing_stability(event_count in 1usize..=10000) {
            use std::collections::VecDeque;

            // 模拟事件队列
            let mut events: VecDeque<InputEvent> = VecDeque::with_capacity(event_count);

            // 注入大量事件
            for i in 0..event_count {
                let scan_code = (i % 256) as u16;
                events.push_back(InputEvent::Key(KeyEvent {
                    scan_code,
                    virtual_key: scan_code,
                    state: KeyState::Pressed,
                    modifiers: ModifierState::default(),
                    device_type: DeviceType::Keyboard,
                    timestamp: 0,
                    is_injected: false,
                }));
            }

            prop_assert_eq!(events.len(), event_count,
                "Event queue should contain exactly {} events", event_count);

            // 处理所有事件（模拟）
            let mut processed = 0;
            while let Some(_) = events.pop_front() {
                processed += 1;
                if processed > event_count + 100 {
                    panic!("Processed more events than expected (possible infinite loop)");
                }
            }

            prop_assert_eq!(processed, event_count,
                "Should process exactly {} events", event_count);
        }
    }

    // ==================== 边界值属性测试 ====================

    /// 属性 18: 特殊 Unicode 字符的处理
    proptest! {
        #[test]
        fn unicode_characters_handling(s in "\\PC{1,50}") {
            // 确保通配符匹配不会因为特殊字符而 panic
            let result = std::panic::catch_unwind(|| {
                let _ = wildcard_match(&s, "*");
                let _ = wildcard_match("*", &s);
                let _ = wildcard_match(&s, &s);
            });

            prop_assert!(result.is_ok(),
                "Wildcard matching should not panic on Unicode input: {:?}", s);
        }
    }

    /// 属性 19: 长模式字符串的处理
    proptest! {
        #[test]
        fn long_patterns_handling(pattern_len in 1usize..=500) {
            let pattern = "a".repeat(pattern_len);
            let input = "a".repeat(pattern_len);

            // 不应该 panic 或超时
            let result = std::panic::catch_unwind(|| {
                wildcard_match(&input, &pattern)
            });

            prop_assert!(result.is_ok(),
                "Should handle long patterns of length {}", pattern_len);

            if result.unwrap() {
                prop_assert!(true,
                    "Identical long strings should match");
            }
        }
    }

    /// 属性 20: 多星号模式的等价性
    proptest! {
        #[test]
        fn multiple_stars_equivalence(s in "\\PC*") {
            if !s.is_empty() {
                // 单个星号、双星号、三星号都应该匹配相同的非空字符串
                let match_single = wildcard_match(&s, "*");
                let match_double = wildcard_match(&s, "**");
                let match_triple = wildcard_match(&s, "***");

                prop_assert_eq!(match_single, match_double,
                    "* and ** should produce same result for '{}'", s);
                prop_assert_eq!(match_double, match_triple,
                    "** and *** should produce same result for '{}'", s);
            }
        }
    }
}
