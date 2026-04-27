// Windows Window Management E2E Tests

#[cfg(target_os = "windows")]
mod integration_tests {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;
    use wakem::platform::traits::WindowFrame;
    use wakem::platform::window_manager_common::CommonWindowApi;
    use wakem::platform::windows::WindowManager;
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

    /// Launch multiple notepad windows for testing
    fn launch_multiple_test_windows(count: usize) -> Vec<u32> {
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(100));

        let mut pids = Vec::new();

        for _ in 0..count {
            if let Ok(child) = Command::new("notepad.exe").spawn() {
                pids.push(child.id());

                // Wait for the new window to appear
                let start = std::time::Instant::now();
                while start.elapsed() < Duration::from_secs(5) {
                    let wm = WindowManager::new();
                    let windows = wm.get_app_visible_windows("notepad.exe");
                    if windows.len() >= pids.len() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                thread::sleep(Duration::from_millis(300));
            }
        }

        pids
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

    // ==================== Window Information Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_foreground_window_info() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let info = wm.get_foreground_window_info();
        assert!(
            info.is_ok(),
            "Should get foreground window info: {:?}",
            info.err()
        );
        let info = info.unwrap();
        assert!(!info.title.is_empty(), "Window title should not be empty");
        assert!(info.frame.width > 0, "Window width should be positive");
        assert!(info.frame.height > 0, "Window height should be positive");

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_window_info_by_handle() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let info = wm.get_window_info(hwnd);
        assert!(info.is_ok(), "Should get window info");

        let info = info.unwrap();
        assert!(
            info.title.contains("Notepad") || info.title.contains("Untitled"),
            "Title should contain 'Notepad' or 'Untitled', got: {}",
            info.title
        );
        assert!(info.frame.width > 0, "Width should be positive");
        assert!(info.frame.height > 0, "Height should be positive");

        teardown();
    }

    // ==================== Window Position Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_set_window_frame() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let new_frame = WindowFrame::new(100, 100, 800, 600);
        let result = wm.set_window_frame(hwnd, &new_frame);
        assert!(result.is_ok(), "Should set window frame");

        wait_for_window_stable();

        let info = wm.get_window_info(hwnd).unwrap();
        assert!(
            (info.frame.x - 100).abs() < 10,
            "X should be near 100, got {}",
            info.frame.x
        );
        assert!(
            (info.frame.y - 100).abs() < 10,
            "Y should be near 100, got {}",
            info.frame.y
        );
        assert!(
            (info.frame.width - 800).abs() < 20,
            "Width should be near 800, got {}",
            info.frame.width
        );
        assert!(
            (info.frame.height - 600).abs() < 20,
            "Height should be near 600, got {}",
            info.frame.height
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_move_to_center() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let original = wm.get_window_info(hwnd).unwrap().frame;
        let result = wm.move_to_center(hwnd);
        assert!(result.is_ok(), "Should move window to center");

        wait_for_window_stable();

        let new_frame = wm.get_window_info(hwnd).unwrap().frame;
        assert!(
            new_frame.x != original.x || new_frame.y != original.y,
            "Window position should have changed"
        );
        assert_eq!(original.width, new_frame.width, "Width should not change");
        assert_eq!(
            original.height, new_frame.height,
            "Height should not change"
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_move_to_edge() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        // Test moving to left edge
        let result = wm.move_to_edge(hwnd, Edge::Left);
        assert!(result.is_ok(), "Should move window to left edge");
        wait_for_window_stable();

        let frame = wm.get_window_info(hwnd).unwrap().frame;
        assert!(
            frame.x < 100,
            "Window should be near left edge, x={}",
            frame.x
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_set_half_screen() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        // Get monitor info for comparison
        let info = wm.get_window_info(hwnd).unwrap();
        let monitor_width = info.work_area.width;

        // Set to left half
        let result = wm.set_half_screen(hwnd, Edge::Left);
        assert!(result.is_ok(), "Should set window to left half");
        wait_for_window_stable();

        let frame = wm.get_window_info(hwnd).unwrap().frame;
        assert!(
            frame.x < 100,
            "Window should be at left edge, x={}",
            frame.x
        );
        // Width should be approximately half the monitor width
        assert!(
            (frame.width - monitor_width / 2).abs() < 50,
            "Width should be approximately half screen, got {} (monitor width: {})",
            frame.width,
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

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let original_width = wm.get_window_info(hwnd).unwrap().frame.width;

        // Cycle through widths
        wm.loop_width(hwnd, Alignment::Left).unwrap();
        wait_for_window_stable();

        let new_width = wm.get_window_info(hwnd).unwrap().frame.width;
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

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let original_height = wm.get_window_info(hwnd).unwrap().frame.height;

        // Cycle through heights
        wm.loop_height(hwnd, Alignment::Top).unwrap();
        wait_for_window_stable();

        let new_height = wm.get_window_info(hwnd).unwrap().frame.height;
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

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        // Test 16:9 ratio
        let result = wm.set_fixed_ratio(hwnd, 16.0 / 9.0);
        assert!(result.is_ok(), "Should set 16:9 ratio");
        wait_for_window_stable();

        let frame = wm.get_window_info(hwnd).unwrap().frame;
        let ratio = frame.width as f32 / frame.height as f32;
        assert!(
            (ratio - 16.0 / 9.0).abs() < 0.1,
            "Ratio should be approximately 16:9, got {}",
            ratio
        );

        // Test 4:3 ratio
        let result = wm.set_fixed_ratio(hwnd, 4.0 / 3.0);
        assert!(result.is_ok(), "Should set 4:3 ratio");
        wait_for_window_stable();

        let frame = wm.get_window_info(hwnd).unwrap().frame;
        let ratio = frame.width as f32 / frame.height as f32;
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

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        // Minimize
        let result = wm.minimize_window(hwnd);
        assert!(result.is_ok(), "Should minimize window");
        wait_for_window_stable();

        // Restore
        let result = wm.restore_window(hwnd);
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

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let original = wm.get_window_info(hwnd).unwrap().frame;

        // Maximize
        let result = wm.maximize_window(hwnd);
        assert!(result.is_ok(), "Should maximize window");
        wait_for_window_stable();

        let maximized = wm.get_window_info(hwnd).unwrap().frame;
        // Maximized window should be larger than original
        assert!(
            maximized.width > original.width || maximized.height > original.height,
            "Maximized window should be larger"
        );

        // Restore
        let result = wm.restore_window(hwnd);
        assert!(result.is_ok(), "Should restore window");
        wait_for_window_stable();

        let restored = wm.get_window_info(hwnd).unwrap().frame;
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

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        // Toggle topmost on
        let result = wm.toggle_topmost(hwnd);
        assert!(result.is_ok(), "Should toggle topmost");
        let is_topmost = result.unwrap();
        wait_for_window_stable();

        // Toggle again
        let result = wm.toggle_topmost(hwnd);
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

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        unsafe {
            assert!(
                IsWindow(Some(hwnd)).as_bool(),
                "Window should be valid before close"
            );
        }

        // Close the window
        let result = wm.close_window(hwnd);
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

    // ==================== Window Enumeration Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_app_visible_windows() {
        setup();

        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();

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

        let wm = WindowManager::new();

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

        let wm = WindowManager::new();

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

    // ==================== Debug Info Test ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test e2e_windows_window -- --ignored"]
    fn test_get_debug_info() {
        setup();

        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();

        let debug_info = wm.get_debug_info();
        assert!(debug_info.is_ok(), "Should get debug info");

        let info = debug_info.unwrap();
        assert!(!info.is_empty(), "Debug info should not be empty");

        teardown();
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_only_placeholder() {
    // Windows-only tests
}
