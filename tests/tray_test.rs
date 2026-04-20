#[cfg(test)]
mod tray_tests {
    use wakem::platform::windows::{
        tray::{IDM_EXIT, IDM_OPEN_CONFIG, IDM_RELOAD, IDM_TOGGLE_ACTIVE},
        tray_api::*,
    };

    // ==================== MockTrayApi 测试 ====================

    #[tokio::test]
    async fn test_mock_tray_api_register() {
        let api = MockTrayApi::new();
        assert!(!api.is_registered());

        api.register(12345).await.unwrap();
        assert!(api.is_registered());
    }

    #[tokio::test]
    async fn test_mock_tray_api_unregister() {
        let api = MockTrayApi::new();
        api.register(12345).await.unwrap();
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
        assert_eq!(
            notifications[0],
            ("Title 1".to_string(), "Message 1".to_string())
        );
        assert_eq!(
            notifications[1],
            ("Title 2".to_string(), "Message 2".to_string())
        );
        assert_eq!(
            notifications[2],
            ("Title 3".to_string(), "Message 3".to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_tray_api_menu_selection() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![IDM_TOGGLE_ACTIVE, IDM_EXIT]);

        let result1 = api.show_menu().await.unwrap();
        assert_eq!(result1, IDM_TOGGLE_ACTIVE);

        let result2 = api.show_menu().await.unwrap();
        assert_eq!(result2, IDM_EXIT);

        // 没有更多预设选择时返回 0
        let result3 = api.show_menu().await.unwrap();
        assert_eq!(result3, 0);
    }

    #[tokio::test]
    async fn test_mock_tray_api_active_state() {
        let api = MockTrayApi::new();

        // 默认激活
        assert!(api.is_active().await);

        // 设置为非激活
        api.set_active(false).await.unwrap();
        assert!(!api.is_active().await);

        // 重新激活
        api.set_active(true).await.unwrap();
        assert!(api.is_active().await);
    }

    // ==================== TrayManager 测试 ====================

    #[tokio::test]
    async fn test_tray_manager_init() {
        let api = MockTrayApi::new();
        let manager = TrayManager::new(api);

        manager.init(12345).await.unwrap();
        assert!(manager.api.is_registered());
    }

    #[tokio::test]
    async fn test_tray_manager_cleanup() {
        let api = MockTrayApi::new();
        let manager = TrayManager::new(api);

        manager.init(12345).await.unwrap();
        assert!(manager.api.is_registered());

        manager.cleanup().await.unwrap();
        assert!(!manager.api.is_registered());
    }

    #[tokio::test]
    async fn test_tray_manager_notify() {
        let api = MockTrayApi::new();
        let manager = TrayManager::new(api);

        manager
            .notify("Notification", "This is a test")
            .await
            .unwrap();

        let notifications = manager.api.get_notifications();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].0, "Notification");
        assert_eq!(notifications[0].1, "This is a test");
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_toggle() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![IDM_TOGGLE_ACTIVE]);

        let manager = TrayManager::new(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::ToggleActive);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_reload() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![IDM_RELOAD]);

        let manager = TrayManager::new(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::Reload);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_open_config() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![IDM_OPEN_CONFIG]);

        let manager = TrayManager::new(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::OpenConfig);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_exit() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![IDM_EXIT]);

        let manager = TrayManager::new(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::Exit);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_none() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![9999]); // 未知 ID

        let manager = TrayManager::new(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::None);
    }

    #[tokio::test]
    async fn test_tray_manager_show_context_menu_cancelled() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![0]); // 0 表示取消

        let manager = TrayManager::new(api);
        let action = manager.show_context_menu().await.unwrap();

        assert_eq!(action, MenuAction::None);
    }

    #[tokio::test]
    async fn test_tray_manager_toggle_active() {
        let api = MockTrayApi::new();
        let manager = TrayManager::new(api);

        // 初始状态为激活
        assert!(manager.is_active().await);

        // 切换为非激活
        let new_state = manager.toggle_active().await.unwrap();
        assert!(!new_state);
        assert!(!manager.is_active().await);

        // 切换回激活
        let new_state = manager.toggle_active().await.unwrap();
        assert!(new_state);
        assert!(manager.is_active().await);
    }

    #[tokio::test]
    async fn test_tray_manager_workflow() {
        // 模拟完整的托盘图标工作流程
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![IDM_TOGGLE_ACTIVE, IDM_RELOAD, IDM_EXIT]);

        let manager = TrayManager::new(api);

        // 初始化
        manager.init(12345).await.unwrap();
        assert!(manager.is_active().await);

        // 显示通知
        manager
            .notify("Wakem", "Application started")
            .await
            .unwrap();

        // 用户点击"切换激活"
        let action1 = manager.show_context_menu().await.unwrap();
        assert_eq!(action1, MenuAction::ToggleActive);

        // 切换状态
        let active = manager.toggle_active().await.unwrap();
        assert!(!active);

        // 用户点击"重新加载"
        let action2 = manager.show_context_menu().await.unwrap();
        assert_eq!(action2, MenuAction::Reload);

        // 用户点击"退出"
        let action3 = manager.show_context_menu().await.unwrap();
        assert_eq!(action3, MenuAction::Exit);

        // 清理
        manager.cleanup().await.unwrap();
    }

    // ==================== MenuAction 测试 ====================

    #[test]
    fn test_menu_action_debug() {
        assert_eq!(format!("{:?}", MenuAction::None), "None");
        assert_eq!(format!("{:?}", MenuAction::ToggleActive), "ToggleActive");
        assert_eq!(format!("{:?}", MenuAction::Reload), "Reload");
        assert_eq!(format!("{:?}", MenuAction::OpenConfig), "OpenConfig");
        assert_eq!(format!("{:?}", MenuAction::Exit), "Exit");
    }

    #[test]
    fn test_menu_action_clone() {
        let action = MenuAction::Reload;
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_menu_action_copy() {
        let action = MenuAction::Exit;
        let copied = action;
        assert_eq!(action, copied); // Copy trait 允许这样做
    }

    #[test]
    fn test_menu_action_equality() {
        assert_eq!(MenuAction::ToggleActive, MenuAction::ToggleActive);
        assert_ne!(MenuAction::ToggleActive, MenuAction::Reload);
        assert_eq!(MenuAction::None, MenuAction::None);
        assert_ne!(MenuAction::Exit, MenuAction::OpenConfig);
    }

    // ==================== 菜单 ID 常量测试 ====================

    #[test]
    fn test_menu_ids() {
        assert_eq!(IDM_TOGGLE_ACTIVE, 100);
        assert_eq!(IDM_RELOAD, 101);
        assert_eq!(IDM_OPEN_CONFIG, 102);
        assert_eq!(IDM_EXIT, 103);
    }

    // ==================== 边界情况测试 ====================

    #[tokio::test]
    async fn test_mock_tray_api_empty_notifications() {
        let api = MockTrayApi::new();
        let notifications = api.get_notifications();
        assert!(notifications.is_empty());
    }

    #[tokio::test]
    async fn test_mock_tray_api_notification_with_unicode() {
        let api = MockTrayApi::new();

        api.show_notification("中文标题", "日本語メッセージ")
            .await
            .unwrap();
        api.show_notification("🎉 Emoji", "Special chars: @#$%")
            .await
            .unwrap();

        let notifications = api.get_notifications();
        assert_eq!(notifications.len(), 2);
        assert_eq!(notifications[0].0, "中文标题");
        assert_eq!(notifications[0].1, "日本語メッセージ");
        assert_eq!(notifications[1].0, "🎉 Emoji");
        assert_eq!(notifications[1].1, "Special chars: @#$%");
    }

    #[tokio::test]
    async fn test_mock_tray_api_multiple_menu_sequences() {
        let api = MockTrayApi::new();
        api.set_menu_selections(vec![IDM_TOGGLE_ACTIVE, IDM_RELOAD]);

        // 第一次菜单
        let result1 = api.show_menu().await.unwrap();
        assert_eq!(result1, IDM_TOGGLE_ACTIVE);

        // 第二次菜单
        let result2 = api.show_menu().await.unwrap();
        assert_eq!(result2, IDM_RELOAD);

        // 重置菜单选择
        api.set_menu_selections(vec![IDM_EXIT]);
        let result3 = api.show_menu().await.unwrap();
        assert_eq!(result3, IDM_EXIT);
    }
}
