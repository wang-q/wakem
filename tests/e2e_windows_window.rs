// Windows 窗口管理 E2E 测试

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
    use wakem::platform::windows::{WindowApi, WindowFrame, WindowManager};
    use wakem::types::Edge;

    fn launch_test_window() -> u32 {
        cleanup_test_windows();
        thread::sleep(Duration::from_millis(100));

        let child = Command::new("notepad.exe")
            .spawn()
            .expect("Failed to launch notepad.exe");

        let api = wakem::platform::windows::RealWindowApi::new();
        let _ = api.wait_for_window(
            Some(r"Notepad|Untitled"),
            None,
            Duration::from_secs(5),
            Duration::from_millis(100),
        );

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
            if title.contains("Notepad") || title.contains("Untitled") || title.contains(".txt") {
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

    #[test]
    #[ignore = "Launches real windows - run manually"]
    fn test_get_foreground_window_info() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let info = wm.get_foreground_window_info();
        assert!(info.is_ok());
        let info = info.unwrap();
        assert!(!info.title.is_empty());
        assert!(info.frame.width > 0);
        assert!(info.frame.height > 0);

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually"]
    fn test_set_window_frame() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let new_frame = WindowFrame::new(100, 100, 800, 600);
        let result = wm.set_window_frame(hwnd, &new_frame);
        assert!(result.is_ok());

        wait_for_window_stable();

        let info = wm.get_window_info(hwnd).unwrap();
        assert!((info.frame.x - 100).abs() < 10);
        assert!((info.frame.y - 100).abs() < 10);

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually"]
    fn test_move_to_center() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let original = wm.get_window_info(hwnd).unwrap().frame;
        let result = wm.move_to_center(hwnd);
        assert!(result.is_ok());

        wait_for_window_stable();

        let new_frame = wm.get_window_info(hwnd).unwrap().frame;
        assert!(new_frame.x != original.x || new_frame.y != original.y);
        assert_eq!(original.width, new_frame.width);

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually"]
    fn test_minimize_and_restore_window() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        let result = wm.minimize_window(hwnd);
        assert!(result.is_ok());
        wait_for_window_stable();

        let result = wm.restore_window(hwnd);
        assert!(result.is_ok());
        wait_for_window_stable();

        unsafe {
            assert!(IsWindow(Some(hwnd)).as_bool());
            assert!(IsWindowVisible(hwnd).as_bool());
        }

        teardown();
    }

    #[test]
    #[ignore = "Launches real windows - run manually"]
    fn test_close_window() {
        setup();
        let _pid = launch_test_window();
        wait_for_window_stable();

        let wm = WindowManager::new();
        let hwnd = get_first_notepad_hwnd().expect("Should find notepad window");

        unsafe {
            assert!(IsWindow(Some(hwnd)).as_bool());
        }

        let result = wm.close_window(hwnd);
        assert!(result.is_ok());

        thread::sleep(Duration::from_millis(1000));

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

        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "notepad.exe"])
            .output();
        thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_only_placeholder() {
    // Windows-only tests
}
