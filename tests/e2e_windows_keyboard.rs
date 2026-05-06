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
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::thread;
    use std::time::Duration;
    use wakem::platform::windows::input::register_hyper_keys;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, GetKeyState, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT,
        KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, PeekMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
        KBDLLHOOKSTRUCT, MSG, PM_REMOVE, WH_KEYBOARD_LL, WM_KEYDOWN,
    };

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

            BACKSPACE_PASSED.store(0, Ordering::SeqCst);

            pump_messages();
            thread::sleep(Duration::from_millis(50));

            send_key(0x14, 0x3A, false);
            send_key(0x08, 0x0E, false);
            send_key(0x08, 0x0E, true);
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
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_keyboard_e2e_placeholder() {}
