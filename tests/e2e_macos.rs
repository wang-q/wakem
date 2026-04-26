// macOS E2E Tests

#[cfg(target_os = "macos")]
mod macos_integration_tests {
    use std::process::Command;
    use std::thread;
    use std::time::Duration;
    use wakem::platform::macos::{RealMacosWindowApi, WindowManager};
    use wakem::platform::traits::{WindowId, WindowManagerTrait};
    use wakem::platform::window_manager_common::CommonWindowApi;
    use wakem::types::Edge;

    fn launch_test_window() {
        let _ = Command::new("open").args(["-a", "Terminal"]).output();
        thread::sleep(Duration::from_secs(2));
    }

    fn get_foreground_window_id(wm: &WindowManager) -> Option<WindowId> {
        WindowManagerTrait::get_foreground_window(wm)
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

        let wm = WindowManager::new(RealMacosWindowApi::new());
        let window_id = WindowManagerTrait::get_foreground_window(&wm);
        assert!(window_id.is_some());

        let info = WindowManagerTrait::get_window_info(&wm, window_id.unwrap());
        assert!(info.is_ok());
        let info = info.unwrap();
        assert!(!info.title.is_empty());
        assert!(info.width > 0);
        assert!(info.height > 0);

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually"]
    fn test_move_to_center() {
        setup();

        let wm = WindowManager::new(RealMacosWindowApi::new());
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => panic!("No foreground window found"),
        };

        let original = WindowManagerTrait::get_window_info(&wm, window_id).ok();

        let result = CommonWindowApi::move_to_center(&wm, window_id);
        assert!(result.is_ok());

        thread::sleep(Duration::from_millis(300));

        let new_info = WindowManagerTrait::get_window_info(&wm, window_id).ok();

        if let (Some(orig), Some(new)) = (original, new_info) {
            assert!(new.x != orig.x || new.y != orig.y);
        }

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually"]
    fn test_move_to_edge() {
        setup();

        let wm = WindowManager::new(RealMacosWindowApi::new());
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => panic!("No foreground window found"),
        };

        let result = CommonWindowApi::move_to_edge(&wm, window_id, Edge::Left);
        assert!(result.is_ok());

        thread::sleep(Duration::from_millis(300));

        let info = WindowManagerTrait::get_window_info(&wm, window_id).ok();

        if let Some(info) = info {
            assert!(info.x < 100);
        }

        teardown();
    }

    #[test]
    #[ignore = "Operates on real windows - run manually"]
    fn test_toggle_topmost() {
        setup();

        let wm = WindowManager::new(RealMacosWindowApi::new());
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => panic!("No foreground window found"),
        };

        let result1 = CommonWindowApi::toggle_topmost(&wm, window_id);
        assert!(result1.is_ok());
        let is_topmost1 = result1.unwrap();

        thread::sleep(Duration::from_millis(200));

        let result2 = CommonWindowApi::toggle_topmost(&wm, window_id);
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

        let wm = WindowManager::new(RealMacosWindowApi::new());
        let window_id = match get_foreground_window_id(&wm) {
            Some(id) => id,
            None => panic!("No foreground window found"),
        };

        let result = wm.switch_to_next_window_of_same_process(window_id);
        assert!(result.is_ok());

        teardown();
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn test_macos_only_placeholder() {
    // macOS-only tests
}
