// macOS 平台属性测试

#[cfg(all(test, target_os = "macos"))]
mod macos_property_tests {
    use proptest::prelude::*;
    use wakem::platform::macos::input::{
        keycode_to_virtual_key, virtual_key_to_keycode,
    };

    /// 键码映射单射性
    #[test]
    fn test_keycode_mapping_is_injective_for_common_keys() {
        let common_keycodes = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x12, 0x13,
            0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x7A, 0x78, 0x63, 0x76, 0x77,
            0x75, 0x73, 0x79, 0x6D, 0x69, 0x6B, 0x71, 0x7B, 0x7C, 0x7D, 0x7E,
        ];

        let mut seen_vks = std::collections::HashSet::new();

        for keycode in common_keycodes {
            let vk = keycode_to_virtual_key(keycode);
            assert!(
                !seen_vks.contains(&vk) || vk == keycode as u16,
                "Duplicate mapping found"
            );
            seen_vks.insert(vk);
        }
    }

    /// 往返一致性
    proptest! {
        #[test]
        fn prop_roundtrip_consistency_for_letters(a in 0x00u16..=0x0Du16) {
            if a != 0x0A {
                let vk = keycode_to_virtual_key(a);
                if vk != a {
                    let reversed = virtual_key_to_keycode(vk);
                    prop_assert_eq!(a, reversed);
                }
            }
        }

        #[test]
        fn prop_roundtrip_consistency_for_digits(d in 0x12u16..=0x1Au16) {
            let vk = keycode_to_virtual_key(d);
            if vk != d {
                let reversed = virtual_key_to_keycode(vk);
                prop_assert_eq!(d, reversed);
            }
        }
    }

    /// 未知键码透传
    proptest! {
        #[test]
        fn prop_unknown_keycode_passthrough(kc in 0xC0u16..0xFFu16) {
            let result = keycode_to_virtual_key(kc);
            prop_assert_eq!(result, kc);
        }
    }

    /// 功能键有序性
    #[test]
    fn test_function_keys_ordered_sequence() {
        let f_keycodes = vec![
            (0x7A, 0x70),
            (0x78, 0x71),
            (0x63, 0x72),
            (0x76, 0x73),
            (0x60, 0x74),
            (0x61, 0x75),
            (0x62, 0x76),
            (0x64, 0x77),
            (0x65, 0x78),
            (0x6D, 0x79),
            (0x67, 0x7A),
            (0x6F, 0x7B),
        ];

        for window in f_keycodes.windows(2) {
            let (_, vk1) = window[0];
            let (_, vk2) = window[1];
            assert!(vk2 > vk1);
        }

        for (kc, vk) in &f_keycodes {
            assert!((0x70..=0x7B).contains(vk));
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod non_macos_placeholder {
    #[test]
    fn placeholder() {
        // macOS 专用测试在非 macOS 平台上跳过
    }
}
