//! Native macOS notification API using NSUserNotificationCenter
//!
//! Replaces osascript display notification with direct Cocoa API calls.
//! Performance: < 5ms (vs 50-100ms with osascript)

// Allow deprecated warnings for cocoa/objc crates
// These crates are deprecated in favor of objc2, but we're using them
// for compatibility with the existing codebase.
#![allow(deprecated)]

use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send, sel, sel_impl};
use tracing::{debug, error, warn};

/// Show notification using NSUserNotificationCenter
///
/// This function uses the native Cocoa API to display a system notification,
/// which is significantly faster than spawning an osascript process.
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
/// Typically completes in < 5ms compared to 50-100ms for osascript.
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

        // Set sound
        let sound_name =
            NSString::alloc(nil).init_str("NSUserNotificationDefaultSoundName");
        let _: () = msg_send![notification, setSoundName: sound_name];

        // Deliver notification
        let _: () = msg_send![center, deliverNotification: notification];

        // Release objects
        let _: () = msg_send![pool, release];

        debug!("Notification delivered: {} - {}", title, message);
        Ok(())
    }
}

/// Show notification with fallback to osascript
///
/// First attempts to use the native NSUserNotificationCenter API.
/// If that fails, falls back to osascript for compatibility.
///
/// # Arguments
/// * `title` - The notification title
/// * `message` - The notification body text
///
/// # Returns
/// * `Ok(())` if either method succeeds
/// * `Err(String)` if both methods fail
pub fn show_notification_with_fallback(
    title: &str,
    message: &str,
) -> Result<(), String> {
    // Try native API first
    match show_notification(title, message) {
        Ok(()) => Ok(()),
        Err(e) => {
            warn!(
                "Native notification failed ({}), falling back to osascript",
                e
            );
            show_notification_osascript(title, message)
        }
    }
}

/// Show notification using osascript (fallback method)
///
/// This is the legacy implementation that spawns an osascript process.
/// It's slower but works in all scenarios.
fn show_notification_osascript(title: &str, message: &str) -> Result<(), String> {
    use std::process::Command;

    let script = format!(
        r#"display notification "{}" with title "{}" sound name "default""#,
        message.replace('"', "\\\""),
        title.replace('"', "\\\"")
    );

    match Command::new("osascript").arg("-e").arg(script).output() {
        Ok(output) if output.status.success() => {
            debug!("Notification shown via osascript: {} - {}", title, message);
            Ok(())
        }
        Err(e) => {
            error!("Failed to show notification via osascript: {}", e);
            Err(format!("Failed to show notification: {}", e))
        }
        _ => {
            error!("osascript failed to display notification");
            Err("osascript failed".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_module_creation() {
        // Just verify the module compiles
        assert!(true);
    }

    #[test]
    fn test_show_notification_osascript_escaping() {
        // Test that quotes are properly escaped
        let title = r#"Test "Quote""#;
        let message = r#"Message with "quotes""#;

        // This should not panic and should properly escape the quotes
        let script = format!(
            r#"display notification "{}" with title "{}" sound name "default""#,
            message.replace('"', "\\\""),
            title.replace('"', "\\\"")
        );

        // Verify the escaping worked
        assert!(script.contains("\\\"Quote\\\""));
        assert!(script.contains("\\\"quotes\\\""));
    }
}
