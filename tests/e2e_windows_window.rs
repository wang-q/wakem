// Windows Window Management E2E Tests

#[cfg(target_os = "windows")]
mod integration_tests {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;
    use wakem::platform::traits::WindowManager;
    use wakem::platform::windows::WindowManager as WindowsWindowManager;
    use wakem::types::{Alignment, Edge};
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, IsWindow, IsWindowVisible, PostMessageW, WM_CLOSE,
    };
    use windows_core::BOOL;

    fn launch_test_window() -> u32 {
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(100));

        let child = Command::new("notepad.exe")
            .spawn()
            .expect("Failed to launch notepad.exe");

        // Wait for window to appear
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if get_first_notepad_hwnd().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        child.id()
    }

    fn cleanup_test_windows() {
        unsafe {
            let mut windows_to_close: Vec<HWND> = Vec::new();
            let _ = EnumWindows(
                Some(enum_notepad_windows),
                LPARAM(&mut windows_to_close as *mut _ as isize),
            );
            for hwnd in windows_to_close {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }

        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "notepad.exe"])
            .output();
        thread::sleep(Duration::from_millis(200));
    }

    unsafe extern "system" fn enum_notepad_windows(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let windows = &mut *(lparam.0 as *mut Vec<HWND>);
        let mut buffer = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut buffer);
        if len > 0 {
            let title = String::from_utf16_lossy(&buffer[..len as usize]);
            if title.contains("Notepad")
                || title.contains("Untitled")
                || title.contains(".txt")
            {
                if IsWindowVisible(hwnd).as_bool() {
                    windows.push(hwnd);
                }
            }
        }
        BOOL(1)
    }

    fn get_first_notepad_hwnd() -> Option<HWND> {
        unsafe {
            let mut result: Option<HWND> = None;
            let _ = EnumWindows(
                Some(enum_first_notepad),
                LPARAM(&mut result as *mut _ as isize),
            );
            result
        }
    }

    unsafe extern "system" fn enum_first_notepad(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let result = &mut *(lparam.0 as *mut Option<HWND>);
        if result.is_some() {
            return BOOL(0);
        }
        let mut buffer = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut buffer);
        if len > 0 {
            let title = String::from_utf16_lossy(&buffer[..len as usize]);
            if title.contains("Notepad") || title.contains("Untitled") {
                if IsWindowVisible(hwnd).as_bool() && IsWindow(Some(hwnd)).as_bool() {
                    *result = Some(hwnd);
                    return BOOL(0);
                }
            }
        }
        BOOL(1)
    }

    fn wait_for_window_stable() {
        thread::sleep(Duration::from_millis(300));
    }

    fn setup() {
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(200));
    }

    fn teardown() {
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(200));
    }

    // Helper to convert HWND to WindowId
    fn hwnd_to_id(hwnd: HWND) -> usize {
        hwnd.0 as usize
    }

    // ==================== Window Information Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_foreground_window() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let foreground = wm.get_foreground_window();
        assert!(foreground.is_some(), "Should get foreground window");

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_window_info() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        let info = wm.get_window_info(window_id);
        assert!(info.is_ok(), "Should get window info");

        let info = info.unwrap();
        assert!(!info.title.is_empty(), "Window title should not be empty");
        assert!(info.width > 0, "Window width should be positive");
        assert!(info.height > 0, "Window height should be positive");

        teardown();
    }

    // ==================== Window Position Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_set_window_pos() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        let result = wm.set_window_pos(window_id, 100, 100, 800, 600);
        assert!(result.is_ok(), "Should set window position");

        wait_for_window_stable();

        let info = wm.get_window_info(window_id).unwrap();
        assert!(
            (info.x - 100).abs() < 10,
            "X should be near 100, got {}",
            info.x
        );
        assert!(
            (info.y - 100).abs() < 10,
            "Y should be near 100, got {}",
            info.y
        );
        assert!(
            (info.width - 800).abs() < 20,
            "Width should be near 800, got {}",
            info.width
        );
        assert!(
            (info.height - 600).abs() < 20,
            "Height should be near 600, got {}",
            info.height
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_move_to_center() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        let original = wm.get_window_info(window_id).unwrap();
        let result = wm.move_to_center(window_id);
        assert!(result.is_ok(), "Should move window to center");

        wait_for_window_stable();

        let new_info = wm.get_window_info(window_id).unwrap();
        assert!(
            new_info.x != original.x || new_info.y != original.y,
            "Window position should have changed"
        );
        assert_eq!(original.width, new_info.width, "Width should not change");
        assert_eq!(original.height, new_info.height, "Height should not change");

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_move_to_edge() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        // Test moving to left edge
        let result = wm.move_to_edge(window_id, Edge::Left);
        assert!(result.is_ok(), "Should move window to left edge");
        wait_for_window_stable();

        let info = wm.get_window_info(window_id).unwrap();
        assert!(
            info.x < 100,
            "Window should be near left edge, x={}",
            info.x
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_set_half_screen() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        // Get monitor info for comparison
        let monitors = wm.get_monitors();
        let monitor_width = monitors[0].width;

        // Set to left half
        let result = wm.set_half_screen(window_id, Edge::Left);
        assert!(result.is_ok(), "Should set window to left half");
        wait_for_window_stable();

        let info = wm.get_window_info(window_id).unwrap();
        assert!(info.x < 100, "Window should be at left edge, x={}", info.x);
        // Width should be approximately half the monitor width
        assert!(
            (info.width - monitor_width / 2).abs() < 50,
            "Width should be approximately half screen, got {} (monitor width: {})",
            info.width,
            monitor_width
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_loop_width_cycle() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        let original_width = wm.get_window_info(window_id).unwrap().width;

        // Cycle through widths
        wm.loop_width(window_id, Alignment::Left).unwrap();
        wait_for_window_stable();

        let new_width = wm.get_window_info(window_id).unwrap().width;
        assert!(
            new_width != original_width,
            "Width should have changed after loop_width"
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_loop_height_cycle() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        let original_height = wm.get_window_info(window_id).unwrap().height;

        // Cycle through heights
        wm.loop_height(window_id, Alignment::Top).unwrap();
        wait_for_window_stable();

        let new_height = wm.get_window_info(window_id).unwrap().height;
        assert!(
            new_height != original_height,
            "Height should have changed after loop_height"
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_set_fixed_ratio_16_9_and_4_3() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        // Test 16:9 ratio
        let result = wm.set_fixed_ratio(window_id, 16.0 / 9.0, None);
        assert!(result.is_ok(), "Should set 16:9 ratio");
        wait_for_window_stable();

        let info = wm.get_window_info(window_id).unwrap();
        let ratio = info.width as f32 / info.height as f32;
        assert!(
            (ratio - 16.0 / 9.0).abs() < 0.1,
            "Ratio should be approximately 16:9, got {}",
            ratio
        );

        // Test 4:3 ratio
        let result = wm.set_fixed_ratio(window_id, 4.0 / 3.0, None);
        assert!(result.is_ok(), "Should set 4:3 ratio");
        wait_for_window_stable();

        let info = wm.get_window_info(window_id).unwrap();
        let ratio = info.width as f32 / info.height as f32;
        assert!(
            (ratio - 4.0 / 3.0).abs() < 0.1,
            "Ratio should be approximately 4:3, got {}",
            ratio
        );

        teardown();
    }

    // ==================== Window State Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_minimize_and_restore_window() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        // Minimize
        let result = wm.minimize_window(window_id);
        assert!(result.is_ok(), "Should minimize window");
        wait_for_window_stable();

        // Restore
        let result = wm.restore_window(window_id);
        assert!(result.is_ok(), "Should restore window");
        wait_for_window_stable();

        // Window should be visible and valid
        unsafe {
            assert!(
                IsWindow(Some(hwnd)).as_bool(),
                "Window should still be valid"
            );
            assert!(
                IsWindowVisible(hwnd).as_bool(),
                "Window should be visible after restore"
            );
        }

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_maximize_and_restore_window() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        let original = wm.get_window_info(window_id).unwrap();

        // Maximize
        let result = wm.maximize_window(window_id);
        assert!(result.is_ok(), "Should maximize window");
        wait_for_window_stable();

        let maximized = wm.get_window_info(window_id).unwrap();
        // Maximized window should be larger than original
        assert!(
            maximized.width > original.width || maximized.height > original.height,
            "Maximized window should be larger"
        );

        // Restore
        let result = wm.restore_window(window_id);
        assert!(result.is_ok(), "Should restore window");
        wait_for_window_stable();

        let restored = wm.get_window_info(window_id).unwrap();
        // Size should be back to approximately original
        assert!(
            (restored.width - original.width).abs() < 50,
            "Width should be restored, original: {}, restored: {}",
            original.width,
            restored.width
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_toggle_topmost() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        // Toggle topmost on
        let result = wm.toggle_topmost(window_id);
        assert!(result.is_ok(), "Should toggle topmost");
        let is_topmost = result.unwrap();
        wait_for_window_stable();

        // Toggle again
        let result = wm.toggle_topmost(window_id);
        assert!(result.is_ok(), "Should toggle topmost again");
        let is_topmost_now = result.unwrap();
        wait_for_window_stable();

        // Should have toggled back
        assert_ne!(
            is_topmost, is_topmost_now,
            "Topmost state should have toggled"
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_close_window() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");
        let window_id = hwnd_to_id(hwnd);

        unsafe {
            assert!(
                IsWindow(Some(hwnd)).as_bool(),
                "Window should be valid before close"
            );
        }

        // Close the window
        let result = wm.close_window(window_id);
        assert!(result.is_ok(), "Should close window");

        // Wait longer for window to actually close
        thread::sleep(Duration::from_millis(1000));

        // Verify window is closed - check multiple times as Windows may delay
        let mut is_invalid = false;
        for _ in 0..5 {
            unsafe {
                if !IsWindow(Some(hwnd)).as_bool() {
                    is_invalid = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(200));
        }

        assert!(is_invalid, "Window should be invalid after close");

        // Skip teardown cleanup since window is already closed
        // Just kill any remaining notepad processes
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "notepad.exe"])
            .output();
        thread::sleep(Duration::from_millis(200));
    }

    // ==================== Window Focus Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_switch_to_next_window_of_same_process() {
        setup();

        // Launch two notepad windows
        let _pid1 = launch_test_window();
        thread::sleep(Duration::from_millis(500));
        let _pid2 = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();

        // Switch to next window - this should work regardless of foreground permissions
        let result = wm.switch_to_next_window_of_same_process();
        assert!(
            result.is_ok(),
            "Should switch to next window: {:?}",
            result.err()
        );
        wait_for_window_stable();

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_single_window_does_not_panic() {
        setup();

        // Launch only one notepad window
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();

        // Switch should not panic with single window
        let result = wm.switch_to_next_window_of_same_process();
        assert!(
            result.is_ok(),
            "Should not panic with single window: {:?}",
            result.err()
        );

        teardown();
    }

    // ==================== Window Enumeration Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_app_visible_windows() {
        setup();

        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();

        // Get notepad windows
        let windows = wm.get_app_visible_windows("notepad.exe");

        assert!(
            !windows.is_empty(),
            "Should find at least one notepad window"
        );

        // Verify all returned handles are valid
        for hwnd in &windows {
            unsafe {
                assert!(IsWindow(Some(*hwnd)).as_bool(), "Handle should be valid");
            }
        }

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_app_visible_windows_finds_notepad() {
        setup();

        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();

        // Get notepad windows
        let windows = wm.get_app_visible_windows("notepad.exe");

        assert!(!windows.is_empty(), "Should find notepad windows");

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_explorer_multi_process_window_enumeration() {
        // This test verifies that get_app_visible_windows correctly finds
        // Explorer windows even though they run in separate processes

        let wm = WindowsWindowManager::new();

        // Get explorer windows - this should work even with multi-process
        let windows = wm.get_app_visible_windows("explorer.exe");

        // We may or may not have Explorer windows open
        // The important thing is that the function doesn't panic
        // and correctly filters out system windows
        for hwnd in &windows {
            unsafe {
                assert!(IsWindow(Some(*hwnd)).as_bool(), "Handle should be valid");
                assert!(IsWindowVisible(*hwnd).as_bool(), "Window should be visible");
            }
        }
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_switch_cycles_through_three_windows() {
        setup();

        // Launch three notepad windows
        let _pid1 = launch_test_window();
        thread::sleep(Duration::from_millis(300));
        let _pid2 = launch_test_window();
        thread::sleep(Duration::from_millis(300));
        let _pid3 = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();

        // Get notepad windows
        let notepad_windows = wm.get_app_visible_windows("notepad.exe");

        assert!(
            notepad_windows.len() >= 3,
            "Should have at least 3 notepad windows, found {}",
            notepad_windows.len()
        );

        // Switch through all windows multiple times
        for _ in 0..notepad_windows.len() * 2 {
            let result = wm.switch_to_next_window_of_same_process();
            assert!(result.is_ok(), "Should switch window");
            wait_for_window_stable();
        }

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_switch_cycles_through_four_windows() {
        setup();

        // Launch four notepad windows
        let _pid1 = launch_test_window();
        thread::sleep(Duration::from_millis(300));
        let _pid2 = launch_test_window();
        thread::sleep(Duration::from_millis(300));
        let _pid3 = launch_test_window();
        thread::sleep(Duration::from_millis(300));
        let _pid4 = launch_test_window();
        wait_for_window_stable();

        let wm = WindowsWindowManager::new();

        // Get notepad windows
        let notepad_windows = wm.get_app_visible_windows("notepad.exe");

        assert!(
            notepad_windows.len() >= 4,
            "Should have at least 4 notepad windows, found {}",
            notepad_windows.len()
        );

        // Switch through all windows multiple times
        for _ in 0..notepad_windows.len() * 2 {
            let result = wm.switch_to_next_window_of_same_process();
            assert!(result.is_ok(), "Should switch window");
            wait_for_window_stable();
        }

        teardown();
    }

    // ==================== Monitor Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_monitors() {
        let wm = WindowsWindowManager::new();
        let monitors = wm.get_monitors();

        assert!(!monitors.is_empty(), "Should have at least one monitor");

        for monitor in &monitors {
            assert!(monitor.width > 0, "Monitor width should be positive");
            assert!(monitor.height > 0, "Monitor height should be positive");
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_only_placeholder() {
    // Windows-only tests
}
