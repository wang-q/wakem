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
            _config: $crate::platform::types::InputDeviceConfig,
            sender: Option<std::sync::mpsc::Sender<$crate::types::InputEvent>>,
        ) -> anyhow::Result<Self::InputDevice> {
            match sender {
                Some(tx) => <$input>::with_sender(tx),
                None => {
                    <$input>::new($crate::platform::types::InputDeviceConfig::default())
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
                $crate::platform::types::PlatformWindowEvent,
            >,
        ) -> Self::WindowEventHook {
            <$hook>::new(sender)
        }
    };
}

/// Macro implementing `TrayLifecycle` with the standard delegation pattern.
/// Both platforms delegate to module-level `tray::run_tray_message_loop` / `tray::stop_tray`.
/// Intended for use in the platform `mod.rs` where `tray` is a direct child module.
#[macro_export]
macro_rules! impl_tray_lifecycle {
    () => {
        fn run_tray_message_loop(
            callback: Box<dyn Fn($crate::platform::types::AppCommand) + Send>,
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
        fn get_current_context() -> Option<$crate::platform::types::WindowContext> {
            super::context::get_current()
        }
    };
}

/// Macro implementing [`WindowEventHookTrait`] with the standard delegation
/// pattern. The platform `WindowEventHook` types expose inherent methods
/// (`start_with_shutdown`, `stop`, `shutdown_flag`) that implement the
/// actual logic. This macro generates trait method bodies that delegate
/// to those inherent methods.
///
/// [`WindowEventHookTrait`]: crate::platform::traits::WindowEventHookTrait
#[macro_export]
macro_rules! impl_window_event_hook {
    () => {
        fn start_with_shutdown(
            &mut self,
            shutdown_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
        ) -> anyhow::Result<()> {
            self.start_with_shutdown(shutdown_flag)
        }

        fn stop(&mut self) {
            self.stop()
        }

        fn shutdown_flag(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
            self.shutdown_flag()
        }
    };
}

/// Macro declaring a unit-struct notification service with `new()` and
/// `Default`. For use when the notification service does not need internal
/// state (macOS). Platforms with stateful services (Windows) define
/// their struct manually.
#[macro_export]
macro_rules! decl_notification_service {
    ($name:ident) => {
        pub struct $name;

        impl $name {
            pub fn new() -> Self {
                Self
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Macro generating module-level delegate functions that forward to
/// `platform_utils` module. Both platforms expose the same two free
/// functions (`get_process_name_by_pid`, `get_executable_path_by_pid`)
/// that simply delegate to the `PlatformUtilities` trait implementation
/// via the `platform_utils` sub-module.
/// Intended for use in the platform `mod.rs` where `platform_utils` is a direct child module.
#[macro_export]
macro_rules! impl_platform_utils_delegates {
    () => {
        pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
            platform_utils::get_process_name_by_pid(pid)
        }

        pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
            platform_utils::get_executable_path_by_pid(pid)
        }
    };
}
