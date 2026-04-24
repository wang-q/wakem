// Real Windows integration tests for window management functionality
//
// These tests launch REAL windows on your desktop and verify window operations.
// They are marked #[ignore] by default — run with:
//   cargo test --test windows_integration -- --ignored --test-threads=1
//
// Prerequisites:
//   - Windows OS (auto-skipped on other platforms)
//   - notepad.exe must be available (built-in on all Windows)

#[cfg(target_os = "windows")]
mod integration_tests {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, IsWindow, IsWindowVisible, PostMessageW, WM_CLOSE,
    };
    use windows_core::BOOL;

    use wakem::platform::windows::WindowApi;
    use wakem::platform::windows::WindowFrame;
    use wakem::platform::windows::WindowManager;
    use wakem::types::Edge;

    // ==================== Helper Functions ====================

    /// Launch a notepad window for testing
    fn launch_test_window() -> u32 {
        // Kill any existing notepad windows first to avoid session restore issues
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(100));

        let child = Command::new("notepad.exe")
            .spawn()
            .expect("Failed to launch notepad.exe");

        // Wait for window to appear using the new API
        let api = wakem::platform::windows::RealWindowApi::new();
        let _ = api.wait_for_window(
            Some(r"Notepad|Untitled"),
            None,
            Duration::from_secs(5),
            Duration::from_millis(100),
        );

        child.id()
    }

    /// Launch multiple notepad windows for testing
    fn launch_multiple_test_windows(count: usize) -> Vec<u32> {
        // Clean up first
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(100));

        let api = wakem::platform::windows::RealWindowApi::new();
        let mut pids = Vec::new();

        for i in 0..count {
            if let Ok(child) = Command::new("notepad.exe").spawn() {
                pids.push(child.id());

                // Wait for the new window to appear
                // We need to wait until we have (i+1) notepad windows
                let start = std::time::Instant::now();
                while start.elapsed() < Duration::from_secs(5) {
                    let windows =
                        api.find_windows(Some(r"Notepad|Untitled"), None, true);
                    if windows.len() >= i + 1 {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                thread::sleep(Duration::from_millis(300));
            }
        }

        pids
    }

    /// Clean up all notepad test windows
    fn cleanup_test_windows() {
        unsafe {
            // Find and close all notepad windows
            let mut windows_to_close: Vec<HWND> = Vec::new();

            let _ = EnumWindows(
                Some(enum_notepad_windows),
                LPARAM(&mut windows_to_close as *mut _ as isize),
            );

            for hwnd in windows_to_close {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }

        // Also try to kill the process
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "notepad.exe", "/FI", "STATUS eq RUNNING"])
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

        BOOL(1) // Continue enumeration
    }

    /// Get the first visible notepad window handle
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
            return BOOL(0); // Already found one
        }

        let mut buffer = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut buffer);
        if len > 0 {
            let title = String::from_utf16_lossy(&buffer[..len as usize]);
            if title.contains("Notepad")
                || title.contains("Untitled")
                || title.contains(".txt")
            {
                if IsWindowVisible(hwnd).as_bool() && IsWindow(Some(hwnd)).as_bool() {
                    *result = Some(hwnd);
                    return BOOL(0); // Stop enumeration
                }
            }
        }

        BOOL(1) // Continue enumeration
    }

    /// Wait for window to be ready
    fn wait_for_window_stable() {
        thread::sleep(Duration::from_millis(300));
    }

    /// Setup function called before each test
    fn setup() {
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(200));
    }

    /// Teardown function called after each test
    fn teardown() {
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(200));
    }

    // ==================== Window Information Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
    fn test_get_window_info_by_handle() {
        setup();

        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();

        // Get a notepad window handle
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
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
    fn test_set_window_frame() {
        setup();

        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        // Set new frame
        let new_frame = WindowFrame::new(100, 100, 800, 600);
        let result = wm.set_window_frame(hwnd, &new_frame);
        assert!(result.is_ok(), "Should set window frame");

        wait_for_window_stable();

        // Verify the change
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
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
    fn test_move_to_center() {
        setup();

        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let original = wm.get_window_info(hwnd).unwrap().frame;

        // Move to center
        let result = wm.move_to_center(hwnd);
        assert!(result.is_ok(), "Should move window to center");

        wait_for_window_stable();

        let new_frame = wm.get_window_info(hwnd).unwrap().frame;
        // Window should have moved (position changed)
        assert!(
            new_frame.x != original.x || new_frame.y != original.y,
            "Window position should have changed"
        );
        // Size should remain the same
        assert_eq!(original.width, new_frame.width, "Width should not change");
        assert_eq!(
            original.height, new_frame.height,
            "Height should not change"
        );

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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

    // ==================== Window State Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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

    // ==================== Window Focus Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
    fn test_switch_to_next_window_of_same_process() {
        setup();

        // Launch two notepad windows
        let _pids = launch_multiple_test_windows(2);
        wait_for_window_stable();

        let wm = WindowManager::new();

        // Get notepad windows using the public API
        let notepad_windows = wm.get_app_visible_windows("notepad.exe");

        assert!(
            notepad_windows.len() >= 2,
            "Should have at least 2 notepad windows, found {}",
            notepad_windows.len()
        );

        // Note: SetForegroundWindow has permission restrictions on Windows
        // We just verify that switch_to_next_window_of_same_process() executes without error
        // and that we can find multiple windows

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
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
    fn test_switch_cycles_through_three_windows() {
        setup();

        // Launch three notepad windows
        let _pids = launch_multiple_test_windows(3);
        wait_for_window_stable();

        let wm = WindowManager::new();

        // Get notepad windows
        let notepad_windows = wm.get_app_visible_windows("notepad.exe");

        assert!(
            notepad_windows.len() >= 3,
            "Should have at least 3 notepad windows, found {}",
            notepad_windows.len()
        );

        // Note: SetForegroundWindow has permission restrictions on Windows
        // We just verify that switch_to_next_window_of_same_process() can be called
        // multiple times without errors

        // Switch through all windows multiple times
        for _ in 0..notepad_windows.len() * 2 {
            let result = wm.switch_to_next_window_of_same_process();
            assert!(result.is_ok(), "Should switch window");
            wait_for_window_stable();
        }

        teardown();
    }

    // ==================== Multi-Monitor Tests ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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

    // ==================== Debug Info Test ====================

    #[test]
    #[ignore = "Launches real windows - run manually with: cargo test --test windows_integration -- --ignored"]
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



// Empty test for non-Windows platforms
#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_only_placeholder() {
    // These tests are Windows-only
}
