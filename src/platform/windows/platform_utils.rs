//! Windows platform utilities and context provider
#![cfg(target_os = "windows")]

use crate::platform::traits::{ContextProvider, PlatformUtilities};

pub struct WindowsPlatform;

impl WindowsPlatform {
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
}

impl PlatformUtilities for WindowsPlatform {
    fn get_modifier_state() -> crate::types::ModifierState {
        Self::get_modifier_state()
    }

    fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
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
                anyhow::bail!("Failed to get process name");
            }

            Ok(String::from_utf16_lossy(&buffer[..len as usize]))
        }
    }

    fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
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
                anyhow::bail!("Failed to get executable path");
            }

            Ok(String::from_utf16_lossy(&buffer[..len as usize]))
        }
    }
}

impl ContextProvider for WindowsPlatform {
    crate::impl_context_provider!();
}

pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
    <WindowsPlatform as PlatformUtilities>::get_process_name_by_pid(pid)
}

pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
    <WindowsPlatform as PlatformUtilities>::get_executable_path_by_pid(pid)
}
