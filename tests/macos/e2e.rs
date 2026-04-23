// Real macOS integration tests for window management functionality
//
// These tests operate on REAL windows on your desktop and verify window operations.
// They are marked #[ignore] by default — run with:
//   cargo test --test macos_e2e -- --ignored --test-threads=1
//
// Prerequisites:
//   - macOS (auto-skipped on other platforms)
//   - Terminal.app or other testable apps must be available

#[cfg(target_os = "macos")]
mod macos_integration_tests {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    use wakem::platform::macos::window_manager::{MacosWindowManager, WindowId};
    use wakem::types::Edge;

    // ==================== Helper Functions ====================

    /// Launch a Terminal window for testing
    fn launch_test_window() {
        let _ = Command::new("open")
            .args(["-a", "Terminal"])
            .output();
        thread::sleep(Duration::from_secs(2));
    }

    /// Get the foreground window ID using WindowManager
    fn get_foreground_window_id(wm: &MacosWindowManager) -> Option<WindowId> {
        wm.get_foreground_window_info()
            .and_then(|r| r.ok())
            .map(|info| info.id)
    }

    /// Setup: launch test window
    fn setup() {
        launch_test_window();
        thread::sleep(Duration::from_millis(500));
    }

    /// Teardown: no cleanup needed on macOS (windows persist)
    fn teardown() {
        thread::sleep(Duration::from_millis(200));
    }

    // ==================== Window Information Tests ====================
    // (mirrors windows/e2e.rs: test_get_foreground_window_info, test_get_window_info_by_handle)

    #[test]
    #[ignore = "Operates on real windows - run manually with: cargo test --test macos_e2e -- --ignored"]
    fn test_get_foreground_window_info() {
        setup();

        let wm = MacosWindowManager::new_real();
        let info = wm.get_foreground_window_info();

        assert!(
            info.is_some(),
            "Should get foreground window info"
        );
        let result = info.unwrap();
        assert!(
            result.is_ok(),
            "Foreground window query should succeed: {:?}",
            result.err()
        );
        let info = result.unwrap();
        assert!(!info.title.is_empty(), "Window title should not be empty");
        assert!(info.width > 0, "Window width should be positive");
        assert!(info.height > 0, "Window height should be positive");

        teardown();
    }

    // ==================== Window Position Tests ====================
    // (mirrors windows/e2e.rs: test_set_window_frame, test_move_to_center, etc.)

    #[test]
    #[ignore = "Operates on real windows - run manually with: cargo test --test macos_e2e -- --ignored"]
    fn test_move_to_center() {
        setup();

        let wm = MacosWindowManager::new_real();
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => {
                panic!("No foreground window found");
            }
        };

        let original = wm.get_foreground_window_info()
            .and_then(|r| r.ok());

        let result = wm.move_to_center(window_id);
        assert!(
            result.is_ok(),
            "Should move window to center: {:?}",
            result.err()
        );

        thread::sleep(Duration::from_millis(300));

        let new_info = wm.get_foreground_window_info()
            .and_then(|r| r.ok());
        
        if let (Some(orig), Some(new)) = (original, new_info) {
            assert!(
                new.x != orig.x || new.y != orig.y,
                "Window position should have changed after centering"
            );
        }

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually with: cargo test --test macos_e2e -- --ignored"]
    fn test_move_to_edge() {
        setup();

        let wm = MacosWindowManager::new_real();
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => {
                panic!("No foreground window found");
            }
        };

        let result = wm.move_to_edge(window_id, Edge::Left);
        assert!(
            result.is_ok(),
            "Should move window to left edge: {:?}",
            result.err()
        );

        thread::sleep(Duration::from_millis(300));

        let info = wm.get_foreground_window_info()
            .and_then(|r| r.ok());
        
        if let Some(info) = info {
            assert!(
                info.x < 100,
                "Window should be near left edge, x={}",
                info.x
            );
        }

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually with: cargo test --test macos_e2e -- --ignored"]
    fn test_set_half_screen() {
        setup();

        let wm = MacosWindowManager::new_real();
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => {
                panic!("No foreground window found");
            }
        };

        let info_before = wm.get_foreground_window_info()
            .and_then(|r| r.ok());
        let monitor_width = info_before.as_ref().map(|i| i.width).unwrap_or(1920);

        let result = wm.set_half_screen(window_id, Edge::Left);
        assert!(
            result.is_ok(),
            "Should set window to half screen: {:?}",
            result.err()
        );

        thread::sleep(Duration::from_millis(300));

        let info = wm.get_foreground_window_info()
            .and_then(|r| r.ok());

        if let Some(info) = info {
            assert!(
                info.x < 100,
                "Window should be at left edge, x={}",
                info.x
            );
            assert!(
                (info.width as i32 - monitor_width / 2).abs() < 100,
                "Width should be approximately half screen, got {} (monitor: {})",
                info.width,
                monitor_width
            );
        }

        teardown();
    }

    // ==================== Window State Tests ====================
    // (mirrors windows/e2e.rs: test_toggle_topmost, etc.)

    #[test]
    #[ignore = "Operates on real windows - run manually with: cargo test --test macos_e2e -- --ignored"]
    fn test_toggle_topmost() {
        setup();

        let wm = MacosWindowManager::new_real();
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => {
                panic!("No foreground window found");
            }
        };

        let result1 = wm.toggle_topmost(window_id);
        assert!(
            result1.is_ok(),
            "Should toggle topmost: {:?}",
            result1.err()
        );
        let is_topmost1 = result1.unwrap();

        thread::sleep(Duration::from_millis(200));

        let result2 = wm.toggle_topmost(window_id);
        assert!(
            result2.is_ok(),
            "Should toggle topmost again: {:?}",
            result2.err()
        );
        let is_topmost2 = result2.unwrap();

        assert_ne!(
            is_topmost1, is_topmost2,
            "Topmost state should have toggled"
        );

        teardown();
    }

    // ==================== Window Switching Tests ====================
    // (mirrors windows/e2e.rs: test_switch_to_next_window_of_same_process)

    #[test]
    #[ignore = "Operates on real windows - run manually with: cargo test --test macos_e2e -- --ignored"]
    fn test_switch_to_next_window_of_same_process() {
        setup();

        let _ = Command::new("open")
            .args(["-a", "Terminal"])
            .output();
        thread::sleep(Duration::from_secs(2));

        let wm = MacosWindowManager::new_real();

        let result = wm.switch_to_next_window_of_same_process();
        assert!(
            result.is_ok(),
            "Should switch to next window of same process: {:?}",
            result.err()
        );

        teardown();
    }

    // ==================== Debug Info Test ====================
    // (mirrors windows/e2e.rs: test_get_debug_info)

    #[test]
    #[ignore = "Operates on real windows - run manually with: cargo test --test macos_e2e -- --ignored"]
    fn test_get_debug_info() {
        setup();

        let wm = MacosWindowManager::new_real();

        let debug_info = wm.get_debug_info();
        assert!(
            !debug_info.is_empty(),
            "Debug info should not be empty"
        );

        teardown();
    }
}

// Empty test for non-macOS platforms
#[cfg(not(target_os = "macos"))]
#[test]
fn test_macos_only_placeholder() {
    // These tests are macOS-only
}
