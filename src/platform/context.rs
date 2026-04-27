//! Platform context for unified platform resource management
//!
//! This module provides a centralized way to manage all platform-specific
//! resources, making it easier to initialize, access, and mock platform
//! services.

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::platform::traits::{
    LauncherTrait, NotificationService, OutputDeviceTrait, WindowPresetManagerTrait,
};

/// Platform context holding all platform-specific resources
///
/// This struct centralizes the creation and management of platform services,
/// providing a single point of access for all platform-specific functionality.
/// It uses the PlatformFactory trait to create platform-specific implementations.
///
/// # Example
///
/// ```rust,no_run
/// use wakem::platform::PlatformContext;
///
/// let ctx = PlatformContext::new().expect("Failed to create platform context");
/// // Use platform services through the context
/// ```
#[allow(dead_code)]
pub struct PlatformContext {
    /// Output device for simulating input events
    pub output_device: Arc<Mutex<Box<dyn OutputDeviceTrait + Send + Sync>>>,
    /// Application launcher
    pub launcher: Arc<Mutex<Box<dyn LauncherTrait>>>,
    /// Window preset manager for saving/loading window layouts
    pub window_preset_manager: Arc<RwLock<Box<dyn WindowPresetManagerTrait>>>,
    /// Notification service for showing system notifications
    pub notification_service: Arc<Mutex<Box<dyn NotificationService>>>,
}

impl PlatformContext {
    /// Create a new platform context with default platform-specific implementations
    ///
    /// This method uses the `CurrentPlatform` type alias to create the appropriate
    /// platform-specific implementations for the current target platform.
    ///
    /// # Errors
    ///
    /// Returns an error if any platform service fails to initialize.
    #[allow(dead_code)]
    pub fn new() -> anyhow::Result<Self> {
        use crate::platform::traits::PlatformFactory;
        use crate::platform::CurrentPlatform;

        Ok(Self {
            output_device: Arc::new(Mutex::new(Box::new(
                CurrentPlatform::create_output_device(),
            ))),
            launcher: Arc::new(Mutex::new(Box::new(CurrentPlatform::create_launcher()))),
            window_preset_manager: Arc::new(RwLock::new(Box::new(
                CurrentPlatform::create_window_preset_manager(),
            ))),
            notification_service: Arc::new(Mutex::new(Box::new(
                CurrentPlatform::create_notification_service(),
            ))),
        })
    }

    /// Create a platform context with custom implementations
    ///
    /// This is useful for testing or when you need to provide mock implementations.
    #[allow(dead_code)]
    pub fn with_services(
        output_device: Box<dyn OutputDeviceTrait + Send + Sync>,
        launcher: Box<dyn LauncherTrait>,
        window_preset_manager: Box<dyn WindowPresetManagerTrait>,
        notification_service: Box<dyn NotificationService>,
    ) -> Self {
        Self {
            output_device: Arc::new(Mutex::new(output_device)),
            launcher: Arc::new(Mutex::new(launcher)),
            window_preset_manager: Arc::new(RwLock::new(window_preset_manager)),
            notification_service: Arc::new(Mutex::new(notification_service)),
        }
    }
}

impl Default for PlatformContext {
    fn default() -> Self {
        // Note: This will panic if platform initialization fails.
        // For production code, use `PlatformContext::new()` and handle errors properly.
        Self::new().expect("Failed to create default platform context")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_context_creation() {
        // This test verifies that PlatformContext can be created
        // Note: In a real test environment, you might want to use mock implementations
        let ctx = PlatformContext::new();
        assert!(ctx.is_ok());
    }
}
