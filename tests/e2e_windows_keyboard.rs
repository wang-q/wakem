// Windows Keyboard E2E Tests
//
// These tests send real keyboard events via SendInput and install real
// keyboard hooks. They are ignored by default because they change real
// machine state (CapsLock toggle, global hook chains, etc.).
//
// Run manually with:
//   cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1

#[cfg(target_os = "windows")]
mod keyboard_e2e_tests {
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread;
    use std::time::Duration;
    use wakem::platform::windows::input::register_hyper_keys;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyState, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, VIRTUAL_KEY,
    };
    use windows::Win32::System::Console::{
        FlushConsoleInputBuffer, GetStdHandle, STD_INPUT_HANDLE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, PeekMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
        KBDLLHOOKSTRUCT, MSG, PM_REMOVE, WH_KEYBOARD_LL, WM_KEYDOWN,
    };

    unsafe fn flush_console_input() {
        if let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) {
            let _ = FlushConsoleInputBuffer(handle);
        }
    }

    unsafe fn pump_messages() {
        let mut msg = MSG::default();
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {}
    }

    unsafe fn get_capslock_toggled() -> bool {
        pump_messages();
        let state = GetKeyState(0x14_i32);
        (state & 0x0001) != 0
    }

    unsafe fn send_key(vk: u16, scan: u16, up: bool) {
        let mut input = INPUT {
            r#type: INPUT_KEYBOARD,
            ..Default::default()
        };
        input.Anonymous.ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(vk),
            wScan: scan,
            dwFlags: KEYEVENTF_SCANCODE,
            ..Default::default()
        };
        if up {
            input.Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
        }
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        thread::sleep(Duration::from_millis(30));
        pump_messages();
        flush_console_input();
    }

    unsafe fn send_extended_key(vk: u16, scan: u16, up: bool) {
        let mut input = INPUT {
            r#type: INPUT_KEYBOARD,
            ..Default::default()
        };
        input.Anonymous.ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(vk),
            wScan: scan,
            dwFlags: KEYEVENTF_SCANCODE | KEYEVENTF_EXTENDEDKEY,
            ..Default::default()
        };
        if up {
            input.Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
        }
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        thread::sleep(Duration::from_millis(30));
        pump_messages();
        flush_console_input();
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_register_hyper_keys_and_check_pressed() {
        let mut keys = std::collections::HashSet::new();
        keys.insert((0x3A_u16, 0x14_u16));
        register_hyper_keys(keys);

        unsafe {
            let caps_pressed = GetAsyncKeyState(0x14_i32) < 0;
            assert!(
                !caps_pressed,
                "CapsLock should not be pressed during automated tests"
            );
        }
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_backspace_passes_without_hyper_key() {
        static BACKSPACE_COUNT: AtomicU32 = AtomicU32::new(0);

        unsafe extern "system" fn monitor_hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if kb.vkCode == 0x08 && wparam.0 == WM_KEYDOWN as usize {
                    BACKSPACE_COUNT.fetch_add(1, Ordering::SeqCst);
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        unsafe {
            BACKSPACE_COUNT.store(0, Ordering::SeqCst);

            let hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(monitor_hook_proc), None, 0)
                    .expect("Failed to set hook");

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x08, 0x0E, false);

            let count_without_hyper = BACKSPACE_COUNT.load(Ordering::SeqCst);
            assert_eq!(
                count_without_hyper, 1,
                "Without Hyper key active, Backspace should pass through. Got {} events",
                count_without_hyper
            );

            send_key(0x08, 0x0E, true);

            let _ = UnhookWindowsHookEx(hook);
        }
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_capslock_held_suppresses_backspace() {
        let mut keys = std::collections::HashSet::new();
        keys.insert((0x3A_u16, 0x14_u16));
        register_hyper_keys(keys);

        static BACKSPACE_PASSED: AtomicU32 = AtomicU32::new(0);

        unsafe extern "system" fn monitor_hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if kb.vkCode == 0x08 && wparam.0 == WM_KEYDOWN as usize {
                    let injected = kb.flags.0 & 0x10 != 0;
                    if !injected {
                        BACKSPACE_PASSED.fetch_add(1, Ordering::SeqCst);
                    }
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        unsafe {
            let capslock_was_toggled = get_capslock_toggled();

            BACKSPACE_PASSED.store(0, Ordering::SeqCst);

            let hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(monitor_hook_proc), None, 0)
                    .expect("Failed to set monitor hook");

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x14, 0x3A, false);

            let caps_pressed = GetAsyncKeyState(0x14_i32) < 0;
            assert!(
                caps_pressed,
                "CapsLock should be detected as pressed after SendInput"
            );

            send_key(0x08, 0x0E, false);
            send_key(0x08, 0x0E, true);
            send_key(0x14, 0x3A, true);

            let backspace_passed_count = BACKSPACE_PASSED.load(Ordering::SeqCst);
            assert_eq!(
                backspace_passed_count, 0,
                "Backspace should be suppressed when CapsLock (Hyper key) is held. \
                 Got {} non-injected Backspace events passing through",
                backspace_passed_count
            );

            let _ = UnhookWindowsHookEx(hook);
            pump_messages();

            let capslock_now_toggled = get_capslock_toggled();
            if capslock_was_toggled != capslock_now_toggled {
                send_key(0x14, 0x3A, false);
                send_key(0x14, 0x3A, true);
            }

            let capslock_final = get_capslock_toggled();
            assert_eq!(
                capslock_final, capslock_was_toggled,
                "CapsLock toggle state should be restored after test"
            );
        }
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_hyper_suppression_with_real_keyboard_hook() {
        use wakem::platform::traits::InputDevice;
        use wakem::platform::windows::RawInputDevice;
        use wakem::platform::windows::input::{
            register_hyper_keys, register_hyper_suffix_keys, set_hyper_active,
        };

        static BACKSPACE_PASSED: AtomicU32 = AtomicU32::new(0);

        unsafe extern "system" fn monitor_hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if kb.vkCode == 0x08 && wparam.0 == WM_KEYDOWN as usize {
                    BACKSPACE_PASSED.fetch_add(1, Ordering::SeqCst);
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        unsafe {
            let capslock_was_toggled = get_capslock_toggled();

            let monitor_hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(monitor_hook_proc), None, 0)
                    .expect("Failed to install monitor hook");

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            let (tx, _rx) = std::sync::mpsc::channel();
            let mut raw_input = RawInputDevice::with_sender(tx)
                .expect("Failed to create RawInputDevice");
            raw_input
                .register()
                .expect("Failed to register RawInputDevice");

            let mut keys = std::collections::HashSet::new();
            keys.insert((0x3A_u16, 0x14_u16));
            register_hyper_keys(keys);

            let mut suffix_keys = std::collections::HashSet::new();
            suffix_keys.insert((0x0E_u16, 0x08_u16));
            register_hyper_suffix_keys(suffix_keys);

            BACKSPACE_PASSED.store(0, Ordering::SeqCst);

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x14, 0x3A, false);
            set_hyper_active(true);

            send_key(0x08, 0x0E, false);
            send_key(0x08, 0x0E, true);

            set_hyper_active(false);
            send_key(0x14, 0x3A, true);

            thread::sleep(Duration::from_millis(100));
            pump_messages();

            let backspace_passed = BACKSPACE_PASSED.load(Ordering::SeqCst);
            assert_eq!(
                backspace_passed, 0,
                "Backspace should be suppressed when CapsLock (Hyper key) is held. \
                 Monitor hook saw {} Backspace events pass through the wakem keyboard hook",
                backspace_passed
            );

            raw_input.stop();
            let _ = UnhookWindowsHookEx(monitor_hook);
            pump_messages();

            let capslock_now_toggled = get_capslock_toggled();
            if capslock_was_toggled != capslock_now_toggled {
                send_key(0x14, 0x3A, false);
                send_key(0x14, 0x3A, true);
            }

            let capslock_final = get_capslock_toggled();
            assert_eq!(
                capslock_final, capslock_was_toggled,
                "CapsLock toggle state should be restored after test"
            );
        }
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_modifier_combo_hyper_suppresses_backspace() {
        use wakem::platform::traits::InputDevice;
        use wakem::platform::windows::RawInputDevice;
        use wakem::platform::windows::input::register_mapped_combos;

        let mut combos = HashSet::new();
        let ctrl_win_alt_flags: u8 = 2 | 8 | 4;
        combos.insert((0x0E, 0x08, ctrl_win_alt_flags));
        register_mapped_combos(combos);

        static BACKSPACE_PASSED: AtomicU32 = AtomicU32::new(0);

        unsafe extern "system" fn monitor_hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if kb.vkCode == 0x08 && wparam.0 == WM_KEYDOWN as usize {
                    BACKSPACE_PASSED.fetch_add(1, Ordering::SeqCst);
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        unsafe {
            let monitor_hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(monitor_hook_proc), None, 0)
                    .expect("Failed to install monitor hook");

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            let (tx, _rx) = std::sync::mpsc::channel();
            let mut raw_input = RawInputDevice::with_sender(tx)
                .expect("Failed to create RawInputDevice");
            raw_input
                .register()
                .expect("Failed to register RawInputDevice");

            BACKSPACE_PASSED.store(0, Ordering::SeqCst);

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x11, 0x1D, false); // Ctrl
            send_extended_key(0x5B, 0x5B, false); // Win (Meta, extended key)
            send_key(0x12, 0x38, false); // Alt
            send_key(0x08, 0x0E, false); // Backspace
            send_key(0x08, 0x0E, true);
            send_key(0x12, 0x38, true);
            send_extended_key(0x5B, 0x5B, true);
            send_key(0x11, 0x1D, true);

            thread::sleep(Duration::from_millis(100));
            pump_messages();

            let backspace_passed = BACKSPACE_PASSED.load(Ordering::SeqCst);
            assert_eq!(
                backspace_passed, 0,
                "Backspace should be suppressed when Ctrl+Win+Alt (Hyper modifier combo) is held. \
                 Monitor hook saw {} Backspace events pass through the wakem keyboard hook",
                backspace_passed
            );

            raw_input.stop();
            let _ = UnhookWindowsHookEx(monitor_hook);
            pump_messages();
        }
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_non_hyper_alt_grave_suppressed() {
        use wakem::platform::traits::InputDevice;
        use wakem::platform::windows::RawInputDevice;
        use wakem::platform::windows::input::register_mapped_combos;

        let mut combos = HashSet::new();
        let alt_flags: u8 = 4;
        combos.insert((0x29, 0xC0, alt_flags));
        register_mapped_combos(combos);

        static GRAVE_PASSED: AtomicU32 = AtomicU32::new(0);

        unsafe extern "system" fn monitor_hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if kb.vkCode == 0xC0
                    && wparam.0 == WM_KEYDOWN as usize
                {
                    GRAVE_PASSED.fetch_add(1, Ordering::SeqCst);
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        unsafe {
            let monitor_hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(monitor_hook_proc), None, 0)
                    .expect("Failed to install monitor hook");

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            let (tx, _rx) = std::sync::mpsc::channel();
            let mut raw_input = RawInputDevice::with_sender(tx)
                .expect("Failed to create RawInputDevice");
            raw_input
                .register()
                .expect("Failed to register RawInputDevice");

            GRAVE_PASSED.store(0, Ordering::SeqCst);

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x12, 0x38, false); // Alt
            send_key(0xC0, 0x29, false); // Grave
            send_key(0xC0, 0x29, true);
            send_key(0x12, 0x38, true);

            thread::sleep(Duration::from_millis(100));
            pump_messages();

            let grave_passed = GRAVE_PASSED.load(Ordering::SeqCst);
            assert_eq!(
                grave_passed, 0,
                "Grave should be suppressed when Alt (physical combo) is held and Alt+Grave is in MAPPED_COMBOS. \
                 Monitor hook saw {} Grave events pass through",
                grave_passed
            );

            raw_input.stop();
            let _ = UnhookWindowsHookEx(monitor_hook);
            pump_messages();
        }
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_non_hyper_grave_passes_without_match() {
        use wakem::platform::traits::InputDevice;
        use wakem::platform::windows::RawInputDevice;

        static GRAVE_PASSED: AtomicU32 = AtomicU32::new(0);

        unsafe extern "system" fn monitor_hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if kb.vkCode == 0xC0
                    && wparam.0 == WM_KEYDOWN as usize
                {
                    GRAVE_PASSED.fetch_add(1, Ordering::SeqCst);
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        unsafe {
            let monitor_hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(monitor_hook_proc), None, 0)
                    .expect("Failed to install monitor hook");

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            let (tx, _rx) = std::sync::mpsc::channel();
            let mut raw_input = RawInputDevice::with_sender(tx)
                .expect("Failed to create RawInputDevice");
            raw_input
                .register()
                .expect("Failed to register RawInputDevice");

            GRAVE_PASSED.store(0, Ordering::SeqCst);

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x11, 0x1D, false); // Ctrl
            send_key(0xC0, 0x29, false); // Grave
            send_key(0xC0, 0x29, true);
            send_key(0x11, 0x1D, true);

            thread::sleep(Duration::from_millis(100));
            pump_messages();

            let grave_passed = GRAVE_PASSED.load(Ordering::SeqCst);
            assert_eq!(
                grave_passed, 1,
                "Grave should pass through when Ctrl is held but Alt+Grave (not Ctrl+Grave) is registered. \
                 Monitor hook saw {} Grave events",
                grave_passed
            );

            raw_input.stop();
            let _ = UnhookWindowsHookEx(monitor_hook);
            pump_messages();
        }
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_ctrl_x_not_hijacked_with_modifier_combo() {
        use wakem::platform::traits::InputDevice;
        use wakem::platform::windows::RawInputDevice;

        static X_PASSED: AtomicU32 = AtomicU32::new(0);

        unsafe extern "system" fn monitor_hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if kb.vkCode == 0x58 && wparam.0 == WM_KEYDOWN as usize {
                    X_PASSED.fetch_add(1, Ordering::SeqCst);
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        unsafe {
            let monitor_hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(monitor_hook_proc), None, 0)
                    .expect("Failed to install monitor hook");

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            let (tx, _rx) = std::sync::mpsc::channel();
            let mut raw_input = RawInputDevice::with_sender(tx)
                .expect("Failed to create RawInputDevice");
            raw_input
                .register()
                .expect("Failed to register RawInputDevice");

            X_PASSED.store(0, Ordering::SeqCst);

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x11, 0x1D, false); // Ctrl
            send_key(0x58, 0x2D, false); // X
            send_key(0x58, 0x2D, true);
            send_key(0x11, 0x1D, true);

            thread::sleep(Duration::from_millis(100));
            pump_messages();

            let x_passed = X_PASSED.load(Ordering::SeqCst);
            assert_eq!(
                x_passed, 1,
                "Ctrl+X should pass through when no Hyper combo is active. \
                 Monitor hook saw {} X events",
                x_passed
            );

            raw_input.stop();
            let _ = UnhookWindowsHookEx(monitor_hook);
            pump_messages();
        }
    }

    #[ignore = "Sends real keyboard events - run manually with: cargo test --test e2e_windows_keyboard -- --ignored --test-threads=1"]
    #[test]
    fn test_c_not_suppressed_with_modifier_combo_hyper() {
        use wakem::platform::traits::InputDevice;
        use wakem::platform::windows::RawInputDevice;

        static C_PASSED: AtomicU32 = AtomicU32::new(0);

        unsafe extern "system" fn monitor_hook_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
                if kb.vkCode == 0x43 && wparam.0 == WM_KEYDOWN as usize {
                    C_PASSED.fetch_add(1, Ordering::SeqCst);
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        unsafe {
            let monitor_hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(monitor_hook_proc), None, 0)
                    .expect("Failed to install monitor hook");

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            let (tx, _rx) = std::sync::mpsc::channel();
            let mut raw_input = RawInputDevice::with_sender(tx)
                .expect("Failed to create RawInputDevice");
            raw_input
                .register()
                .expect("Failed to register RawInputDevice");

            C_PASSED.store(0, Ordering::SeqCst);

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x11, 0x1D, false); // Ctrl
            send_extended_key(0x5B, 0x5B, false); // Win (Meta, extended key)
            send_key(0x12, 0x38, false); // Alt
            send_key(0x43, 0x2E, false); // C
            send_key(0x43, 0x2E, true);
            send_key(0x12, 0x38, true);
            send_extended_key(0x5B, 0x5B, true);
            send_key(0x11, 0x1D, true);

            thread::sleep(Duration::from_millis(100));
            pump_messages();

            let c_passed = C_PASSED.load(Ordering::SeqCst);
            assert_eq!(
                c_passed, 1,
                "C should NOT be suppressed with modifier-combo Hyper. \
                 Only Backspace/Delete produce weird characters. \
                 Monitor hook saw {} C events",
                c_passed
            );

            raw_input.stop();
            let _ = UnhookWindowsHookEx(monitor_hook);
            pump_messages();
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_keyboard_e2e_placeholder() {}


