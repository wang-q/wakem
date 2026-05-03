//! macOS notification service
#![cfg(target_os = "macos")]

use crate::platform::traits::NotificationService;
use anyhow::Result;

crate::decl_notification_service!(MacosNotificationService);

impl NotificationService for MacosNotificationService {
    fn show(&self, title: &str, message: &str) -> Result<()> {
        super::native_api::notification::show_notification(title, message)
    }
}
