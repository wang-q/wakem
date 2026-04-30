//! Unit tests for window_actions module
//!
//! Tests the platform-agnostic window action execution logic using
//! MockWindowApi to verify correct behavior without platform dependencies.

#[cfg(test)]
mod tests {
    use wakem::platform::mock::mock_window_api::MockWindowApi;
    use wakem::runtime::window_actions::execute_window_action;
    use wakem::types::{Alignment, Edge, MonitorDirection, WindowAction};

    /// Helper to create a mock window manager with a foreground window
    fn setup_mock_wm() -> MockWindowApi<usize> {
        let wm = MockWindowApi::new();
        let window_id = 1;

        // Set up a foreground window with initial position
        wm.set_foreground_window(window_id);
        wm.set_window_rect(
            window_id,
            wakem::platform::traits::WindowFrame::new(100, 100, 800, 600),
        );
        wm.set_monitor_info(
            window_id,
            wakem::platform::traits::MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        );

        wm
    }

    // ==================== Basic Position Operations ====================

    #[test]
    fn test_execute_move_action() {
        let wm = setup_mock_wm();

        let action = WindowAction::Move { x: 200, y: 300 };
        let result = execute_window_action(&wm, &action);

        assert!(result.is_ok(), "Move action should succeed");
    }

    #[test]
    fn test_execute_resize_action() {
        let wm = setup_mock_wm();

        let action = WindowAction::Resize {
            width: 1024,
            height: 768,
        };
        let result = execute_window_action(&wm, &action);

        assert!(result.is_ok(), "Resize action should succeed");
    }

    // ==================== Window State Operations ====================

    #[test]
    fn test_execute_minimize_action() {
        let wm = setup_mock_wm();

        let result = execute_window_action(&wm, &WindowAction::Minimize);

        assert!(result.is_ok(), "Minimize action should succeed");
    }

    #[test]
    fn test_execute_maximize_action() {
        let wm = setup_mock_wm();

        let result = execute_window_action(&wm, &WindowAction::Maximize);

        assert!(result.is_ok(), "Maximize action should succeed");
    }

    #[test]
    fn test_execute_restore_action() {
        let wm = setup_mock_wm();

        let result = execute_window_action(&wm, &WindowAction::Restore);

        assert!(result.is_ok(), "Restore action should succeed");
    }

    #[test]
    fn test_execute_close_action() {
        let wm = setup_mock_wm();

        let result = execute_window_action(&wm, &WindowAction::Close);

        assert!(result.is_ok(), "Close action should succeed");
    }

    // ==================== Z-order Operations ====================

    #[test]
    fn test_execute_toggle_topmost_action() {
        let wm = setup_mock_wm();

        let result = execute_window_action(&wm, &WindowAction::ToggleTopmost);

        assert!(result.is_ok(), "ToggleTopmost action should succeed");

        // Toggle again
        let result = execute_window_action(&wm, &WindowAction::ToggleTopmost);
        assert!(result.is_ok(), "Second ToggleTopmost should succeed");
    }

    // ==================== Multi-monitor Operations ====================

    #[test]
    fn test_execute_move_to_monitor_next() {
        let wm = setup_mock_wm();

        let action = WindowAction::MoveToMonitor(MonitorDirection::Next);
        let result = execute_window_action(&wm, &action);

        assert!(
            result.is_ok(),
            "MoveToMonitor Next should succeed with monitor info"
        );
    }

    #[test]
    fn test_execute_move_to_monitor_by_index_valid() {
        let wm = setup_mock_wm();

        let action = WindowAction::MoveToMonitor(MonitorDirection::Index(0));
        let result = execute_window_action(&wm, &action);

        assert!(result.is_ok(), "MoveToMonitor Index(0) should succeed");
    }

    #[test]
    fn test_execute_move_to_monitor_by_index_invalid() {
        let wm = setup_mock_wm();

        let action = WindowAction::MoveToMonitor(MonitorDirection::Index(99));
        let result = execute_window_action(&wm, &action);

        assert!(
            result.is_err(),
            "MoveToMonitor Index(99) should fail - out of range"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("out of range"),
            "Error should mention 'out of range', got: {}",
            err_msg
        );
    }

    #[test]
    fn test_execute_move_to_monitor_no_monitors() {
        let wm = MockWindowApi::new();
        wm.set_foreground_window(1);

        let action = WindowAction::MoveToMonitor(MonitorDirection::Next);
        let result = execute_window_action(&wm, &action);

        assert!(result.is_err(), "Should fail - no monitors found");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("No monitors"),
            "Error should mention 'No monitors', got: {}",
            err_msg
        );
    }

    // ==================== Error Handling ====================

    #[test]
    fn test_execute_no_foreground_window() {
        let wm = MockWindowApi::new();

        // No foreground window set
        let result = execute_window_action(&wm, &WindowAction::Minimize);

        assert!(result.is_err(), "Should fail without foreground window");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("No foreground window"),
            "Error should mention 'No foreground window', got: {}",
            err_msg
        );
    }

    #[test]
    fn test_execute_none_action() {
        let wm = setup_mock_wm();

        // None action should succeed and do nothing
        let result = execute_window_action(&wm, &WindowAction::None);

        assert!(result.is_ok(), "None action should succeed");
    }

    // ==================== Advanced Operations (Not Yet Implemented) ====================

    #[test]
    fn test_advanced_actions_succeed_without_error() {
        let wm = setup_mock_wm();

        // These actions should not fail, just log debug messages
        let advanced_actions: Vec<WindowAction> = vec![
            WindowAction::Center,
            WindowAction::MoveToEdge(Edge::Left),
            WindowAction::HalfScreen(Edge::Right),
            WindowAction::LoopWidth(Alignment::Center),
            WindowAction::LoopHeight(Alignment::Top),
            WindowAction::FixedRatio {
                ratio: 1.0,
                scale_index: 0,
            },
            WindowAction::NativeRatio { scale_index: 0 },
            WindowAction::SwitchToNextWindow,
        ];

        for (i, action) in advanced_actions.iter().enumerate() {
            let result = execute_window_action(&wm, action);
            assert!(
                result.is_ok(),
                "Advanced action #{} {:?} should not fail",
                i,
                action
            );
        }
    }

    #[test]
    fn test_show_debug_info_action() {
        let wm = setup_mock_wm();

        let result = execute_window_action(&wm, &WindowAction::ShowDebugInfo);
        assert!(result.is_ok(), "ShowDebugInfo should succeed");
    }

    #[test]
    fn test_show_notification_action() {
        let wm = setup_mock_wm();

        let action = WindowAction::ShowNotification {
            title: "Test".to_string(),
            message: "Hello".to_string(),
        };
        let result = execute_window_action(&wm, &action);

        assert!(
            result.is_ok(),
            "ShowNotification should succeed (logs debug)"
        );
    }

    #[test]
    fn test_preset_actions_succeed_without_error() {
        let wm = setup_mock_wm();

        let preset_actions: Vec<WindowAction> = vec![
            WindowAction::SavePreset {
                name: "test".to_string(),
            },
            WindowAction::LoadPreset {
                name: "test".to_string(),
            },
            WindowAction::ApplyPreset,
        ];

        for (i, action) in preset_actions.iter().enumerate() {
            let result = execute_window_action(&wm, action);
            assert!(
                result.is_ok(),
                "Preset action #{} {:?} should not fail",
                i,
                action
            );
        }
    }

    // ==================== Sequential Actions ====================

    #[test]
    fn test_multiple_sequential_actions() {
        let wm = setup_mock_wm();

        // Execute multiple actions in sequence
        let actions = vec![
            WindowAction::Minimize,
            WindowAction::Maximize,
            WindowAction::Restore,
            WindowAction::Close,
        ];

        for (i, action) in actions.iter().enumerate() {
            let result = execute_window_action(&wm, action);
            assert!(
                result.is_ok(),
                "Sequential action #{} {:?} should succeed",
                i,
                action
            );
        }
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_boundary_coordinates() {
        let wm = setup_mock_wm();

        // Test with boundary values
        let move_actions = vec![
            WindowAction::Move { x: 0, y: 0 },
            WindowAction::Move {
                x: i32::MAX,
                y: i32::MAX,
            },
            WindowAction::Move {
                x: i32::MIN,
                y: i32::MIN,
            },
        ];

        for (i, action) in move_actions.iter().enumerate() {
            let result = execute_window_action(&wm, action);
            assert!(result.is_ok(), "Boundary move action #{} should succeed", i);
        }
    }

    #[test]
    fn test_large_resize_dimensions() {
        let wm = setup_mock_wm();

        let action = WindowAction::Resize {
            width: 3840,
            height: 2160,
        }; // 4K resolution
        let result = execute_window_action(&wm, &action);

        assert!(result.is_ok(), "Large resize should succeed");
    }

    #[test]
    fn test_zero_size_resize() {
        let wm = setup_mock_wm();

        let action = WindowAction::Resize {
            width: 0,
            height: 0,
        };
        let result = execute_window_action(&wm, &action);

        // Should succeed (even if size is invalid, it's up to platform to validate)
        assert!(
            result.is_ok(),
            "Zero-size resize should not error in generic code"
        );
    }
}
