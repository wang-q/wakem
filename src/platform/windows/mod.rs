//! Windows platform implementation

pub mod context;
pub mod input;
pub mod input_device;
pub mod launcher;
pub mod output_device;
pub mod tray;
pub mod window_api;
pub mod window_event_hook;
pub mod window_manager;
pub mod window_preset;

// Re-export types for backward compatibility
pub use context::WindowContext;
pub use input::RawInputDevice as LegacyRawInputDevice;
#[allow(unused_imports)]
pub use input_device::{InputDevice, InputDeviceFactory, RawInputDevice};
pub use launcher::Launcher;
#[allow(unused_imports)]
pub use output_device::{OutputDevice, SendInputDevice, WindowsOutputDevice};
#[allow(unused_imports)]
pub use tray::{
    run_tray_message_loop, stop_tray, AppCommand, MenuAction, MockTrayApi, RealTrayApi,
    TrayApi, TrayIcon, TrayManager,
};
#[allow(unused_imports)]
pub use window_api::{
    MonitorInfo, MonitorWorkArea, RealWindowApi, WindowApi, WindowOperation, WindowState,
};

// Re-export shared types from platform::traits
#[allow(unused_imports)]
pub use crate::platform::traits::InputDeviceConfig;

// Mock implementations are only exported during tests
#[cfg(test)]
#[allow(unused_imports)]
pub use crate::platform::mock::MockInputDevice;
#[cfg(test)]
pub use crate::platform::mock::MockOutputDevice;
#[cfg(test)]
pub use output_device::MockOutputEvent;
#[cfg(test)]
pub use window_api::MockWindowApi;

pub use window_event_hook::{WindowEvent, WindowEventHook};
pub use window_manager::{MonitorDirection, WindowFrame, WindowManager};
pub use window_preset::WindowPresetManager;

/// Get current modifier state for Windows
pub fn get_modifier_state() -> crate::types::ModifierState {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU,
        VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SHIFT,
    };

    let mut modifiers = crate::types::ModifierState::default();

    unsafe {
        // Check Shift key
        if GetAsyncKeyState(VK_SHIFT.0 as i32) < 0
            || GetAsyncKeyState(VK_LSHIFT.0 as i32) < 0
            || GetAsyncKeyState(VK_RSHIFT.0 as i32) < 0
        {
            modifiers.shift = true;
        }

        // Check Ctrl key
        if GetAsyncKeyState(VK_CONTROL.0 as i32) < 0
            || GetAsyncKeyState(VK_LCONTROL.0 as i32) < 0
            || GetAsyncKeyState(VK_RCONTROL.0 as i32) < 0
        {
            modifiers.ctrl = true;
        }

        // Check Alt key
        if GetAsyncKeyState(VK_MENU.0 as i32) < 0
            || GetAsyncKeyState(VK_LMENU.0 as i32) < 0
            || GetAsyncKeyState(VK_RMENU.0 as i32) < 0
        {
            modifiers.alt = true;
        }

        // Check Win key (VK_LWIN = 0x5B, VK_RWIN = 0x5C)
        if GetAsyncKeyState(0x5B) < 0 || GetAsyncKeyState(0x5C) < 0 {
            modifiers.meta = true;
        }
    }

    modifiers
}
