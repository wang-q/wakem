pub mod context;
pub mod input;
pub mod input_device;
pub mod launcher;
pub mod output;
pub mod output_device;
pub mod tray;
pub mod tray_api;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

#[allow(unused_imports)]
pub use context::WindowContext;
#[allow(unused_imports)]
pub use input::RawInputDevice as LegacyRawInputDevice;
#[allow(unused_imports)]
pub use input_device::{
    InputDevice, InputDeviceConfig, InputDeviceFactory, RawInputDevice,
};
#[allow(unused_imports)]
pub use launcher::Launcher;
#[allow(unused_imports)]
pub use output::OutputDevice as LegacyOutputDevice;
#[allow(unused_imports)]
pub use output_device::{OutputDevice, OutputEvent, SendInputDevice};
#[allow(unused_imports)]
pub use tray::TrayIcon;
#[allow(unused_imports)]
pub use tray_api::{
    MenuAction, RealTrayApi, TrayApi, TrayIcon as TrayIconAlias, TrayManager,
};
#[allow(unused_imports)]
pub use window_api::{
    MonitorInfo, MonitorWorkArea, RealWindowApi, WindowApi, WindowOperation, WindowState,
};

// Mock 实现仅在测试时导出
#[cfg(test)]
pub use input_device::MockInputDevice;
#[cfg(test)]
pub use output_device::MockOutputDevice;
#[cfg(test)]
pub use tray_api::MockTrayApi;
#[cfg(test)]
pub use window_api::MockWindowApi;

pub use window_event_hook::{WindowEvent, WindowEventHook};
pub use window_manager::{MonitorDirection, WindowFrame, WindowManager};
pub use window_preset::WindowPresetManager;
