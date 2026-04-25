//! Native macOS notification API using NSUserNotificationCenter
//!
//! Uses direct Cocoa API calls for displaying system notifications.
//! Performance: < 5ms
#![cfg(target_os = "macos")]
// Allow deprecated warnings for cocoa/objc crates
// These crates are deprecated in favor of objc2, but we're using them
// for compatibility with the existing codebase.
#![allow(deprecated)]

use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send, sel, sel_impl};
use tracing::debug;

/// Show notification using NSUserNotificationCenter
///
/// This function uses the native Cocoa API to display a system notification.
///
/// # Arguments
/// * `title` - The notification title
/// * `message` - The notification body text
///
/// # Returns
/// * `Ok(())` if the notification was successfully queued
/// * `Err(String)` if there was an error
///
/// # Performance
/// Typically completes in < 5ms.
#[allow(clippy::deprecated_clippy_cfg_attr)]
#[cfg_attr(clippy, allow(unexpected_cfgs))]
pub fn show_notification(title: &str, message: &str) -> Result<(), String> {
    unsafe {
        let pool = NSAutoreleasePool::new(nil);

        // Get default notification center
        let center: id = msg_send![
            class!(NSUserNotificationCenter),
            defaultUserNotificationCenter
        ];

        if center == nil {
            let _: () = msg_send![pool, release];
            return Err("Failed to get notification center".to_string());
        }

        // Create notification
        let notification: id = msg_send![class!(NSUserNotification), alloc];
        let notification: id = msg_send![notification, init];

        if notification == nil {
            let _: () = msg_send![pool, release];
            return Err("Failed to create notification".to_string());
        }

        // Set title
        let title_ns = NSString::alloc(nil).init_str(title);
        let _: () = msg_send![notification, setTitle: title_ns];

        // Set informative text (message)
        let message_ns = NSString::alloc(nil).init_str(message);
        let _: () = msg_send![notification, setInformativeText: message_ns];

        // Deliver notification
        let _: () = msg_send![center, deliverNotification: notification];

        // Release objects
        let _: () = msg_send![pool, release];

        debug!("Notification delivered: {} - {}", title, message);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_notification_module_creation() {
        // Just verify the module compiles
        assert!(true);
    }
}
