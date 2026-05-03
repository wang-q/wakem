//! Windows platform utilities and context provider
#![cfg(target_os = "windows")]

use crate::platform::traits::{ContextProvider, PlatformUtilities};
use crate::platform::types::WindowContext;

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
                return Err(anyhow::anyhow!("Failed to get process name"));
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
                return Err(anyhow::anyhow!("Failed to get executable path"));
            }

            Ok(String::from_utf16_lossy(&buffer[..len as usize]))
        }
    }

    fn parse_key_fallback(name: &str) -> Option<crate::types::KeyInfo> {
        use crate::types::KeyInfo;

        match name {
            "capslock" | "caps" => Some(KeyInfo::new(0x3A, 0x14)),
            "backspace" => Some(KeyInfo::new(0x0E, 0x08)),
            "enter" | "return" => Some(KeyInfo::new(0x1C, 0x0D)),
            "escape" | "esc" => Some(KeyInfo::new(0x01, 0x1B)),
            "space" => Some(KeyInfo::new(0x39, 0x20)),
            "tab" => Some(KeyInfo::new(0x0F, 0x09)),
            "grave" | "backtick" => Some(KeyInfo::new(0x29, 0xC0)),
            "left" => Some(KeyInfo::new(0x4B, 0x25)),
            "up" => Some(KeyInfo::new(0x48, 0x26)),
            "right" => Some(KeyInfo::new(0x4D, 0x27)),
            "down" => Some(KeyInfo::new(0x50, 0x28)),
            "home" => Some(KeyInfo::new(0x47, 0x24)),
            "end" => Some(KeyInfo::new(0x4F, 0x23)),
            "pageup" => Some(KeyInfo::new(0x49, 0x21)),
            "pagedown" => Some(KeyInfo::new(0x51, 0x22)),
            "delete" | "del" | "forwarddelete" | "forwarddel" => {
                Some(KeyInfo::new(0x53, 0x2E))
            }
            "insert" | "ins" => Some(KeyInfo::new(0x52, 0x2D)),
            "lshift" | "leftshift" => Some(KeyInfo::new(0x2A, 0xA0)),
            "rshift" | "rightshift" => Some(KeyInfo::new(0x36, 0xA1)),
            "lctrl" | "lcontrol" | "leftctrl" | "leftcontrol" => {
                Some(KeyInfo::new(0x1D, 0xA2))
            }
            "rctrl" | "rcontrol" | "rightctrl" | "rightcontrol" => {
                Some(KeyInfo::new(0xE01D, 0xA3))
            }
            "lalt" | "leftalt" => Some(KeyInfo::new(0x38, 0xA4)),
            "ralt" | "rightalt" => Some(KeyInfo::new(0xE038, 0xA5)),
            "lwin" | "lmeta" | "leftwin" | "leftmeta" => {
                Some(KeyInfo::new(0xE05B, 0x5B))
            }
            "rwin" | "rmeta" | "rightwin" | "rightmeta" => {
                Some(KeyInfo::new(0xE05C, 0x5C))
            }
            "a" => Some(KeyInfo::new(0x1E, 0x41)),
            "b" => Some(KeyInfo::new(0x30, 0x42)),
            "c" => Some(KeyInfo::new(0x2E, 0x43)),
            "d" => Some(KeyInfo::new(0x20, 0x44)),
            "e" => Some(KeyInfo::new(0x12, 0x45)),
            "f" => Some(KeyInfo::new(0x21, 0x46)),
            "g" => Some(KeyInfo::new(0x22, 0x47)),
            "h" => Some(KeyInfo::new(0x23, 0x48)),
            "i" => Some(KeyInfo::new(0x17, 0x49)),
            "j" => Some(KeyInfo::new(0x24, 0x4A)),
            "k" => Some(KeyInfo::new(0x25, 0x4B)),
            "l" => Some(KeyInfo::new(0x26, 0x4C)),
            "m" => Some(KeyInfo::new(0x32, 0x4D)),
            "n" => Some(KeyInfo::new(0x31, 0x4E)),
            "o" => Some(KeyInfo::new(0x18, 0x4F)),
            "p" => Some(KeyInfo::new(0x19, 0x50)),
            "q" => Some(KeyInfo::new(0x10, 0x51)),
            "r" => Some(KeyInfo::new(0x13, 0x52)),
            "s" => Some(KeyInfo::new(0x1F, 0x53)),
            "t" => Some(KeyInfo::new(0x14, 0x54)),
            "u" => Some(KeyInfo::new(0x16, 0x55)),
            "v" => Some(KeyInfo::new(0x2F, 0x56)),
            "w" => Some(KeyInfo::new(0x11, 0x57)),
            "x" => Some(KeyInfo::new(0x2D, 0x58)),
            "y" => Some(KeyInfo::new(0x15, 0x59)),
            "z" => Some(KeyInfo::new(0x2C, 0x5A)),
            "0" => Some(KeyInfo::new(0x0B, 0x30)),
            "1" => Some(KeyInfo::new(0x02, 0x31)),
            "2" => Some(KeyInfo::new(0x03, 0x32)),
            "3" => Some(KeyInfo::new(0x04, 0x33)),
            "4" => Some(KeyInfo::new(0x05, 0x34)),
            "5" => Some(KeyInfo::new(0x06, 0x35)),
            "6" => Some(KeyInfo::new(0x07, 0x36)),
            "7" => Some(KeyInfo::new(0x08, 0x37)),
            "8" => Some(KeyInfo::new(0x09, 0x38)),
            "9" => Some(KeyInfo::new(0x0A, 0x39)),
            "f1" => Some(KeyInfo::new(0x3B, 0x70)),
            "f2" => Some(KeyInfo::new(0x3C, 0x71)),
            "f3" => Some(KeyInfo::new(0x3D, 0x72)),
            "f4" => Some(KeyInfo::new(0x3E, 0x73)),
            "f5" => Some(KeyInfo::new(0x3F, 0x74)),
            "f6" => Some(KeyInfo::new(0x40, 0x75)),
            "f7" => Some(KeyInfo::new(0x41, 0x76)),
            "f8" => Some(KeyInfo::new(0x42, 0x77)),
            "f9" => Some(KeyInfo::new(0x43, 0x78)),
            "f10" => Some(KeyInfo::new(0x44, 0x79)),
            "f11" => Some(KeyInfo::new(0x57, 0x7A)),
            "f12" => Some(KeyInfo::new(0x58, 0x7B)),
            "comma" | "," => Some(KeyInfo::new(0x33, 0xBC)),
            "period" | "." => Some(KeyInfo::new(0x34, 0xBE)),
            "semicolon" | ";" => Some(KeyInfo::new(0x27, 0xBA)),
            "quote" | "'" | "apostrophe" => Some(KeyInfo::new(0x28, 0xDE)),
            "bracketleft" | "[" => Some(KeyInfo::new(0x1A, 0xDB)),
            "bracketright" | "]" => Some(KeyInfo::new(0x1B, 0xDD)),
            "backslash" | "\\" => Some(KeyInfo::new(0x2B, 0xDC)),
            "minus" | "-" => Some(KeyInfo::new(0x0C, 0xBD)),
            "equal" | "=" => Some(KeyInfo::new(0x0D, 0xBB)),
            "numpad0" | "num0" => Some(KeyInfo::new(0x52, 0x60)),
            "numpad1" | "num1" => Some(KeyInfo::new(0x4F, 0x61)),
            "numpad2" | "num2" => Some(KeyInfo::new(0x50, 0x62)),
            "numpad3" | "num3" => Some(KeyInfo::new(0x51, 0x63)),
            "numpad4" | "num4" => Some(KeyInfo::new(0x4B, 0x64)),
            "numpad5" | "num5" => Some(KeyInfo::new(0x4C, 0x65)),
            "numpad6" | "num6" => Some(KeyInfo::new(0x4D, 0x66)),
            "numpad7" | "num7" => Some(KeyInfo::new(0x47, 0x67)),
            "numpad8" | "num8" => Some(KeyInfo::new(0x48, 0x68)),
            "numpad9" | "num9" => Some(KeyInfo::new(0x49, 0x69)),
            "numpaddot" | "numdot" | "numpaddecimal" => Some(KeyInfo::new(0x53, 0x6E)),
            "numpadenter" | "numenter" => Some(KeyInfo::new(0x1C, 0x0C)),
            "numpadadd" | "numplus" => Some(KeyInfo::new(0x4E, 0x6B)),
            "numpadsub" | "numminus" => Some(KeyInfo::new(0x4A, 0x6D)),
            "numpadmul" | "nummul" | "numpadmultiply" => Some(KeyInfo::new(0x37, 0x6A)),
            "numpaddiv" | "numslash" | "numpaddivide" => Some(KeyInfo::new(0x35, 0x6F)),
            _ => None,
        }
    }
}

impl ContextProvider for WindowsPlatform {
    fn get_current_context() -> Option<WindowContext> {
        super::context::get_current()
    }
}

pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
    <WindowsPlatform as PlatformUtilities>::get_process_name_by_pid(pid)
}

pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
    <WindowsPlatform as PlatformUtilities>::get_executable_path_by_pid(pid)
}
