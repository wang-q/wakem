// Windows System Tray Tests

#[cfg(all(test, target_os = "windows"))]
mod tray_tests {
    use wakem::platform::common::tray::{menu_ids, MockTrayApi, TrayApi, TrayManager};
    use wakem::platform::types::MenuAction;

    #[tokio::test]
    async fn test_mock_tray_api_register() {
        let api = MockTrayApi::new();
        assert!(!api.is_registered());

        api.register(Some(12345)).await.unwrap();
        assert!(api.is_registered());
    }

    #[tokio::test]
    async fn test_mock_tray_api_unregister() {
        let api = MockTrayApi::new();
        api.register(Some(12345)).await.unwrap();
        assert!(api.is_registered());

        api.unregister().await.unwrap();
        assert!(!api.is_registered());
    }

    #[tokio::test]
    async fn test_mock_tray_api_notification() {
        let api = MockTrayApi::new();

        api.show_notification("Test Title", "Test Message")
            .await
            .unwrap();

        let notifications = api.get_notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].0, "Test Title");
        assert_eq!(notifications[0].1, "Test Message");
    }

    #[tokio::test]
    async fn test_mock_tray_api_multiple_notifications() {
        let api = MockTrayApi::new();

        api.show_notification("Title 1", "Message 1").await.unwrap();
        api.show_notification("Title 2", "Message 2").await.unwrap();
        api.show_notification("Title 3", "Message 3").await.unwrap();

        let notifications = api.get_notifications();
        assert_eq!(notifications.len(), 3);
    }

    #[tokio::test]
    async fn test_mock_tray_api_menu_selection() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![menu_ids::TOGGLE_ACTIVE, menu_ids::EXIT]);

        let result1 = api.show_menu().await.unwrap();
        assert_eq!(result1, menu_ids::TOGGLE_ACTIVE);

        let result2 = api.show_menu().await.unwrap();
        assert_eq!(result2, menu_ids::EXIT);

        let result3 = api.show_menu().await.unwrap();
        assert_eq!(result3, 0);
    }

    #[tokio::test]
    async fn test_mock_tray_api_active_state() {
        let api = MockTrayApi::new();

        assert!(api.is_active().await);

        api.set_active(false).await.unwrap();
        assert!(!api.is_active().await);

        api.set_active(true).await.unwrap();
        assert!(api.is_active().await);
    }

    #[tokio::test]
    async fn test_tray_manager_init() {
        let api = MockTrayApi::new();
        let manager = TrayManager::from_api(api);

        manager.init(Some(12345)).await.unwrap();
        assert!(manager.api().unwrap().is_registered());
    }

    #[tokio::test]
    async fn test_tray_manager_cleanup() {
        let api = MockTrayApi::new();
        let manager = TrayManager::from_api(api);

        manager.init(Some(12345)).await.unwrap();
        assert!(manager.api().unwrap().is_registered());

        manager.cleanup().await.unwrap();
        assert!(!manager.api().unwrap().is_registered());
    }

    #[tokio::test]
    async fn test_tray_manager_notify() {
        let api = MockTrayApi::new();
        let manager = TrayManager::from_api(api);

        manager
            .notify("Notification", "This is a test")
            .await
            .unwrap();

        let notifications = manager.api().unwrap().get_notifications();
        assert_eq!(notifications.len(), 1);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_toggle() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![menu_ids::TOGGLE_ACTIVE]);

        let manager = TrayManager::from_api(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::ToggleActive);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_reload() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![menu_ids::RELOAD]);

        let manager = TrayManager::from_api(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::Reload);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_open_config() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![menu_ids::OPEN_CONFIG]);

        let manager = TrayManager::from_api(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::OpenConfig);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_exit() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![menu_ids::EXIT]);

        let manager = TrayManager::from_api(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::Exit);
    }

    #[tokio::test]
    async fn test_tray_manager_toggle_active() {
        let api = MockTrayApi::new();
        let manager = TrayManager::from_api(api);

        assert!(manager.is_active().await);

        let new_state = manager.toggle_active().await.unwrap();
        assert!(!new_state);
        assert!(!manager.is_active().await);

        let new_state = manager.toggle_active().await.unwrap();
        assert!(new_state);
        assert!(manager.is_active().await);
    }

    #[test]
    fn test_menu_action_debug() {
        assert_eq!(format!("{:?}", MenuAction::None), "None");
        assert_eq!(format!("{:?}", MenuAction::ToggleActive), "ToggleActive");
        assert_eq!(format!("{:?}", MenuAction::Reload), "Reload");
        assert_eq!(format!("{:?}", MenuAction::OpenConfig), "OpenConfig");
        assert_eq!(format!("{:?}", MenuAction::Exit), "Exit");
    }

    #[test]
    fn test_menu_action_equality() {
        assert_eq!(MenuAction::ToggleActive, MenuAction::ToggleActive);
        assert_ne!(MenuAction::ToggleActive, MenuAction::Reload);
    }

    #[test]
    fn test_menu_ids() {
        assert_eq!(menu_ids::TOGGLE_ACTIVE, 100);
        assert_eq!(menu_ids::RELOAD, 101);
        assert_eq!(menu_ids::OPEN_CONFIG, 102);
        assert_eq!(menu_ids::EXIT, 103);
    }
}

#[cfg(not(target_os = "windows"))]
#[test]
fn test_windows_tray_only_placeholder() {
    // Windows-only tests
}
