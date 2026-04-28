//! Shared macros for platform module implementations.
//!
//! These macros reduce boilerplate in platform-specific `mod.rs` files by
//! centralizing the repetitive trait method delegation patterns that are
//! identical across Windows and macOS.

/// Macro to implement `PlatformFactory` with the six boilerplate factory methods
/// that are identical across platforms. Only the associated types differ.
#[macro_export]
macro_rules! impl_platform_factory_methods {
    ($platform:ty, $input:ty, $output:ty, $wm:ty, $wpm:ty, $notif:ty,
     $launcher:ty, $hook:ty) => {
        fn create_input_device(
            _config: $crate::platform::traits::InputDeviceConfig,
            sender: Option<std::sync::mpsc::Sender<$crate::types::InputEvent>>,
        ) -> anyhow::Result<Self::InputDevice> {
            match sender {
                Some(tx) => <$input>::with_sender(tx),
                None => {
                    <$input>::new($crate::platform::traits::InputDeviceConfig::default())
                }
            }
        }

        fn create_output_device() -> Self::OutputDevice {
            <$output>::new()
        }

        fn create_window_manager() -> Self::WindowManager {
            <$wm>::new()
        }

        fn create_window_preset_manager() -> Self::WindowPresetManager {
            <$wpm>::new(<$wm>::new())
        }

        fn create_notification_service() -> Self::NotificationService {
            <$notif>::new()
        }

        fn create_launcher() -> Self::Launcher {
            <$launcher>::new()
        }

        fn create_window_event_hook(
            sender: std::sync::mpsc::Sender<
                $crate::platform::traits::PlatformWindowEvent,
            >,
        ) -> Self::WindowEventHook {
            <$hook>::new(sender)
        }
    };
}

/// Macro implementing `TrayLifecycle` with the standard delegation pattern.
/// Both platforms delegate to module-level `tray::run_tray_message_loop` / `tray::stop_tray`.
#[macro_export]
macro_rules! impl_tray_lifecycle {
    () => {
        fn run_tray_message_loop(
            callback: Box<dyn Fn($crate::platform::traits::AppCommand) + Send>,
        ) -> anyhow::Result<()> {
            tray::run_tray_message_loop(callback)
        }

        fn stop_tray() {
            tray::stop_tray()
        }
    };
}

/// Macro implementing `ContextProvider` with the standard delegation pattern.
/// Both platforms delegate to module-level `context::get_current()`.
#[macro_export]
macro_rules! impl_context_provider {
    () => {
        fn get_current_context() -> Option<$crate::platform::traits::WindowContext> {
            context::get_current()
        }
    };
}
