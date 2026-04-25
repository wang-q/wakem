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
#[allow(unused_imports)]
pub use context::get_current as get_window_context;
pub use input::RawInputDevice as LegacyRawInputDevice;
#[allow(unused_imports)]
pub use input_device::RawInputDevice;
pub use launcher::Launcher;
#[allow(unused_imports)]
pub use output_device::SendInputDevice;
#[allow(unused_imports)]
pub use tray::{
    run_tray_message_loop, stop_tray, RealTrayApi, TrayApi, TrayIcon, TrayManager,
};
#[allow(unused_imports)]
pub use window_api::RealWindowApi;

// Re-export shared types from platform::traits
#[allow(unused_imports)]
pub use crate::platform::traits::{
    AppCommand, InputDeviceConfig, MenuAction, MonitorInfo, MonitorWorkArea,
    WindowFrame, WindowOperation, WindowState,
};

// Re-export WindowPreset from config
#[allow(unused_imports)]
pub use crate::config::WindowPreset;

// Mock implementations are only exported during tests
#[cfg(test)]
#[allow(unused_imports)]
pub use crate::platform::mock::MockInputDevice;
#[cfg(test)]
pub use crate::platform::mock::MockOutputDevice;
#[cfg(test)]
// MockOutputEvent is now in platform::mock
#[cfg(test)]
pub use window_api::MockWindowApi;

#[allow(unused_imports)]
pub use crate::platform::window_preset_common::WindowPresetApi;
pub use window_event_hook::{WindowEvent, WindowEventHook};
pub use window_manager::{MonitorDirection, WindowManager};
pub use window_preset::WindowPresetManager;

/// Get current modifier state for Windows
pub fn get_modifier_state() -> crate::types::ModifierState {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_MENU,
        VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_SHIFT,
    };

    let mut modifiers = crate::types::ModifierState::default();

    unsafe {
        if GetAsyncKeyState(VK_SHIFT.0 as i32) < 0
            || GetAsyncKeyState(VK_LSHIFT.0 as i32) < 0
            || GetAsyncKeyState(VK_RSHIFT.0 as i32) < 0
        {
            modifiers.shift = true;
        }

        if GetAsyncKeyState(VK_CONTROL.0 as i32) < 0
            || GetAsyncKeyState(VK_LCONTROL.0 as i32) < 0
            || GetAsyncKeyState(VK_RCONTROL.0 as i32) < 0
        {
            modifiers.ctrl = true;
        }

        if GetAsyncKeyState(VK_MENU.0 as i32) < 0
            || GetAsyncKeyState(VK_LMENU.0 as i32) < 0
            || GetAsyncKeyState(VK_RMENU.0 as i32) < 0
        {
            modifiers.alt = true;
        }

        if GetAsyncKeyState(0x5B) < 0 || GetAsyncKeyState(0x5C) < 0 {
            modifiers.meta = true;
        }
    }

    modifiers
}

pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    unsafe {
        let handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                .map_err(|e| anyhow::anyhow!("Failed to open process: {}", e))?;

        let mut buffer = [0u16; 260];
        let len = GetModuleBaseNameW(handle, None, &mut buffer);

        CloseHandle(handle).ok();

        if len == 0 {
            return Err(anyhow::anyhow!("Failed to get process name"));
        }

        Ok(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    unsafe {
        let handle =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
                .map_err(|e| anyhow::anyhow!("Failed to open process: {}", e))?;

        let mut buffer = [0u16; 260];
        let len = GetModuleFileNameExW(Some(handle), None, &mut buffer);

        CloseHandle(handle).ok();

        if len == 0 {
            return Err(anyhow::anyhow!("Failed to get executable path"));
        }

        Ok(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}
