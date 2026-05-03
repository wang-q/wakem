// macOS E2E Tests

#[cfg(target_os = "macos")]
mod macos_integration_tests {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;
    use wakem::platform::macos::WindowManager;
    use wakem::platform::traits::WindowId;
    use wakem::platform::traits::{
        ForegroundWindowOperations, WindowOperations, WindowSwitching,
    };
    use wakem::types::Edge;

    fn launch_test_window() {
        let _ = Command::new("open").args(["-a", "Terminal"]).output();
        thread::sleep(Duration::from_secs(2));
    }

    fn get_foreground_window_id(wm: &WindowManager) -> Option<WindowId> {
        wm.get_foreground_window()
    }

    fn setup() {
        launch_test_window();
        thread::sleep(Duration::from_millis(500));
    }

    fn teardown() {
        thread::sleep(Duration::from_millis(200));
    }

    #[test]
    #[ignore = "Operates on real windows - run manually"]
    fn test_get_foreground_window_info() {
        setup();

        let wm = WindowManager::new();
        let window_id = wm.get_foreground_window();
        assert!(window_id.is_some());

        if let Some(id) = window_id {
            let info = wm.get_window_info(id);
            assert!(info.is_ok());
            let info = info.unwrap();
            assert!(!info.title.is_empty());
            assert!(info.width > 0);
            assert!(info.height > 0);
        }

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually"]
    fn test_move_to_center() {
        setup();

        let wm = WindowManager::new();
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => panic!("No foreground window found"),
        };

        let original = wm.get_window_info(window_id).ok();

        use wakem::platform::traits::WindowManagerExt;
        let result = wm.move_to_center(window_id);
        assert!(result.is_ok());

        thread::sleep(Duration::from_millis(300));

        let new_info = wm.get_window_info(window_id).ok();

        if let (Some(orig), Some(new)) = (original, new_info) {
            assert!(new.x != orig.x || new.y != orig.y);
        }

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually"]
    fn test_move_to_edge() {
        setup();

        let wm = WindowManager::new();
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => panic!("No foreground window found"),
        };

        use wakem::platform::traits::WindowManagerExt;
        let result = wm.move_to_edge(window_id, Edge::Left);
        assert!(result.is_ok());

        thread::sleep(Duration::from_millis(300));

        let info = wm.get_window_info(window_id).ok();

        if let Some(info) = info {
            assert!(info.x < 100);
        }

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually"]
    fn test_toggle_topmost() {
        setup();

        let wm = WindowManager::new();
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => panic!("No foreground window found"),
        };

        use wakem::platform::traits::WindowManagerExt;
        let result1 = wm.toggle_topmost(window_id);
        assert!(result1.is_ok());
        let is_topmost1 = result1.unwrap();

        thread::sleep(Duration::from_millis(200));

        let result2 = wm.toggle_topmost(window_id);
        assert!(result2.is_ok());
        let is_topmost2 = result2.unwrap();

        assert_ne!(is_topmost1, is_topmost2);

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually"]
    fn test_switch_to_next_window_of_same_process() {
        setup();

        let _ = Command::new("open").args(["-a", "Terminal"]).output();
        thread::sleep(Duration::from_secs(2));

        let wm = WindowManager::new();

        let result = wm.switch_to_next_window_of_same_process();
        assert!(result.is_ok());

        teardown();
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_macos_only_placeholder() {
    // macOS-only tests
}
