pub mod context;
pub mod input;
pub mod input_device;
pub mod launcher;
pub mod output;
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

pub use context::WindowContext;
pub use input::RawInputDevice as LegacyRawInputDevice;
pub use input_device::{InputDevice, RawInputDevice, InputDeviceConfig, InputDeviceFactory};
pub use launcher::Launcher;
pub use output::OutputDevice as LegacyOutputDevice;
pub use output_device::{OutputDevice, SendInputDevice, OutputEvent};
pub use tray::TrayIcon;
pub use window_api::{WindowApi, RealWindowApi, WindowOperation, WindowState, MonitorInfo, MonitorWorkArea};

// Mock 实现仅在测试时导出
#[cfg(test)]
pub use window_api::MockWindowApi;
#[cfg(test)]
pub use input_device::MockInputDevice;
#[cfg(test)]
pub use output_device::MockOutputDevice;

pub use window_event_hook::{WindowEvent, WindowEventHook};
pub use window_manager::{MonitorDirection, WindowFrame, WindowManager};
pub use window_preset::WindowPresetManager;
