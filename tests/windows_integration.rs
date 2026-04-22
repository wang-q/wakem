// Real Windows integration tests for window switching (Alt+` functionality)
//
// These tests launch REAL windows on your desktop and verify window switching behavior.
// They are marked #[ignore] by default — run with:
//   cargo test --test windows_integration -- --ignored
//
// Prerequisites:
//   - Windows OS (auto-skipped on other platforms)
//   - notepad.exe must be available (built-in on all Windows)

#[cfg(target_os = "windows")]
mod integration_tests {
    use std::process::Command;
    use std::time::Duration;

    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, IsWindow,
    };

    use wakem::platform::PlatformWindowManager;

    fn launch_notepad() -> u32 {
        let child = Command::new("notepad.exe")
            .spawn()
            .expect("Failed to launch notepad.exe");
        child.id()
    }

    fn wait_for<F>(condition: F, timeout: Duration) -> bool
    where
        F: Fn() -> bool,
    {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }

    unsafe fn get_window_title(hwnd: HWND) -> String {
        let mut buf = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..len as usize])
    }

    fn cleanup_notepads() {
        let _ = Command::new("taskkill")
            .args(["/IM", "notepad.exe", "/F"])
            .output();
    }

    // ==================== Core Tests ====================

    #[test]
    #[ignore]
    fn test_switch_between_two_notepad_windows() {
        cleanup_notepads();
        std::thread::sleep(Duration::from_millis(500));

        let pid1 = launch_notepad();
        let pid2 = launch_notepad();

        assert_ne!(pid1, pid2, "Two notepads should have different PIDs");

        let wm = PlatformWindowManager::new();

        let appeared = wait_for(
            || match wm.get_app_visible_windows("notepad.exe") {
                windows if windows.len() >= 2 => true,
                _ => false,
            },
            Duration::from_secs(5),
        );
        assert!(appeared, "Timed out waiting for 2 notepad windows to appear");

        let windows_before = wm.get_app_visible_windows("notepad.exe");
        assert!(
            windows_before.len() >= 2,
            "Expected >= 2 notepad windows, got {}",
            windows_before.len()
        );

        let hwnd_a = windows_before[0];
        let hwnd_b = windows_before[1];

        unsafe {
            println!(
                "[TEST] Window A ({:?}): '{}'",
                hwnd_a,
                get_window_title(hwnd_a)
            );
            println!(
                "[TEST] Window B ({:?}): '{}'",
                hwnd_b,
                get_window_title(hwnd_b)
            );
        }

        wm.switch_to_next_window_of_same_process()
            .expect("First switch should succeed");

        std::thread::sleep(Duration::from_millis(300));

        unsafe {
            let fg = GetForegroundWindow();
            println!(
                "[TEST] After 1st switch, foreground: {:?} '{}'",
                fg,
                get_window_title(fg)
            );
        }

        wm.switch_to_next_window_of_same_process()
            .expect("Second switch should succeed");

        std::thread::sleep(Duration::from_millis(300));

        unsafe {
            let fg = GetForegroundWindow();
            println!(
                "[TEST] After 2nd switch, foreground: {:?} '{}'",
                fg,
                get_window_title(fg)
            );
        }

        cleanup_notepads();
    }

    #[test]
    #[ignore]
    fn test_get_app_visible_windows_finds_notepad() {
        let _pid = launch_notepad();
        std::thread::sleep(Duration::from_millis(1000));

        let wm = PlatformWindowManager::new();
        let windows = wm.get_app_visible_windows("notepad.exe");

        println!("[TEST] Found {} notepad window(s)", windows.len());
        assert!(!windows.is_empty(), "Should find at least 1 notepad window");

        for (i, &hwnd) in windows.iter().enumerate() {
            unsafe {
                println!(
                    "[TEST]   Window {}: {:?} '{}'",
                    i,
                    hwnd,
                    get_window_title(hwnd)
                );
            }
        }

        cleanup_notepads();
    }

    #[test]
    #[ignore]
    fn test_explorer_multi_process_window_enumeration() {
        let wm = PlatformWindowManager::new();
        let explorer_windows = wm.get_app_visible_windows("explorer.exe");

        println!(
            "[TEST] Found {} explorer window(s)",
            explorer_windows.len()
        );

        for (i, &hwnd) in explorer_windows.iter().enumerate() {
            unsafe {
                println!(
                    "[TEST]   Explorer window {}: {:?} '{}'",
                    i,
                    hwnd,
                    get_window_title(hwnd)
                );
            }
        }

        assert!(
            !unsafe { has_program_manager(&explorer_windows) },
            "Should NOT include Program Manager (desktop shell)"
        );
    }

    unsafe fn has_program_manager(windows: &[HWND]) -> bool {
        windows
            .iter()
            .any(|&hwnd| get_window_title(hwnd) == "Program Manager")
    }

    #[test]
    #[ignore]
    fn test_single_window_does_not_panic() {
        let _pid = launch_notepad();
        std::thread::sleep(Duration::from_millis(1000));

        let wm = PlatformWindowManager::new();

        let result = wm.switch_to_next_window_of_same_process();
        assert!(
            result.is_ok(),
            "Single window switch should be Ok (graceful no-op), got err: {:?}",
            result.err()
        );

        println!("[TEST] Single window switch returned Ok as expected");
        cleanup_notepads();
    }

    #[test]
    #[ignore]
    fn test_switch_cycles_through_three_windows() {
        cleanup_notepads();
        std::thread::sleep(Duration::from_millis(500));

        let _pid1 = launch_notepad();
        let _pid2 = launch_notepad();
        let _pid3 = launch_notepad();

        let wm = PlatformWindowManager::new();

        let ready = wait_for(
            || wm.get_app_visible_windows("notepad.exe").len() >= 3,
            Duration::from_secs(5),
        );
        assert!(ready, "Timed out waiting for 3 notepad windows");

        for cycle in 0..=3 {
            wm.switch_to_next_window_of_same_process()
                .unwrap_or_else(|e| panic!("Switch {} failed: {}", cycle, e));
            std::thread::sleep(Duration::from_millis(300));

            unsafe {
                let fg = GetForegroundWindow();
                println!(
                    "[TEST] Cycle {} foreground: {:?} '{}'",
                    cycle,
                    fg,
                    get_window_title(fg)
                );
            }
        }

        cleanup_notepads();
    }
}

#[cfg(not(target_os = "windows"))]
mod integration_tests {
    #[test]
    #[ignore]
    fn test_placeholder() {
        eprintln!("Real window integration tests require Windows OS");
    }
}
