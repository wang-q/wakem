//! Windows Raw Input implementation
#![cfg(target_os = "windows")]

use crate::constants::WHEEL_DELTA;
use crate::types::{
    InputEvent, KeyEvent, KeyState, ModifierState, MouseButton, MouseEvent,
    MouseEventType,
};
use anyhow::Result;
use std::cell::RefCell;
use std::sync::mpsc::Sender;
use tracing::{debug, trace, warn};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::{
    GetRawInputData, GetRegisteredRawInputDevices, RegisterRawInputDevices,
    RAWINPUTDEVICE_FLAGS, RIDEV_INPUTSINK,
};
use windows::Win32::UI::Input::{
    RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, RegisterClassW,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, MSG, WM_CREATE, WM_DESTROY, WM_INPUT,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW,
    WS_EX_NOACTIVATE, WS_OVERLAPPEDWINDOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MsgWaitForMultipleObjectsEx, PeekMessageW, MWMO_INPUTAVAILABLE, PM_REMOVE,
    QS_ALLINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL,
};

thread_local! {
    static CURRENT_SENDER: RefCell<Option<Sender<InputEvent>>> = const { RefCell::new(None) };
}

/// Raw Input device manager
pub struct RawInputDevice {
    hwnd: HWND,
    keyboard_hook: Option<windows::Win32::UI::WindowsAndMessaging::HHOOK>,
    running: bool,
}

impl RawInputDevice {
    pub fn new(event_sender: Sender<InputEvent>) -> Result<Self> {
        CURRENT_SENDER.with(|s| {
            *s.borrow_mut() = Some(event_sender);
        });

        let hwnd = Self::create_message_window()?;

        Self::register_devices(hwnd)?;

        let keyboard_hook = Self::install_keyboard_hook()?;

        Ok(Self {
            hwnd,
            keyboard_hook: Some(keyboard_hook),
            running: false,
        })
    }

    /// Create message window (for receiving Raw Input messages)
    fn create_message_window() -> Result<HWND> {
        unsafe {
            let class_name = windows::core::w!("WakemRawInputWindow");
            let hinstance =
                windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?;

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(Self::window_proc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
                style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            };

            RegisterClassW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_NOACTIVATE,
                class_name,
                windows::core::w!("Wakem Raw Input"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                None,
                None,
                Some(windows::Win32::Foundation::HINSTANCE(hinstance.0)),
                None,
            )
            .map_err(|e| anyhow::anyhow!("Failed to create window: {}", e))?;

            debug!("Raw Input message window created: {:?}", hwnd);
            Ok(hwnd)
        }
    }

    /// Register Raw Input devices
    fn register_devices(hwnd: HWND) -> Result<()> {
        let devices = [
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x06,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            },
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x02,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            },
        ];

        unsafe {
            RegisterRawInputDevices(
                &devices,
                std::mem::size_of::<RAWINPUTDEVICE>() as u32,
            )?;
        }

        let mut registered = [RAWINPUTDEVICE {
            usUsagePage: 0,
            usUsage: 0,
            dwFlags: RAWINPUTDEVICE_FLAGS(0),
            hwndTarget: HWND(std::ptr::null_mut()),
        }; 2];
        let mut count = 2u32;
        unsafe {
            let result = GetRegisteredRawInputDevices(
                Some(registered.as_mut_ptr()),
                &mut count,
                std::mem::size_of::<RAWINPUTDEVICE>() as u32,
            );
            if result == u32::MAX {
                warn!("GetRegisteredRawInputDevices failed");
            } else {
                debug!(
                    "Verified {} registered devices: [0]=({:#06x},{:#06x}) [1]=({:#06x},{:#06x})",
                    result,
                    registered[0].usUsagePage,
                    registered[0].usUsage,
                    registered[1].usUsagePage,
                    registered[1].usUsage,
                );
            }
        }

        debug!("Raw Input devices registered successfully");
        Ok(())
    }

    fn install_keyboard_hook() -> Result<windows::Win32::UI::WindowsAndMessaging::HHOOK>
    {
        unsafe {
            let hook =
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(Self::keyboard_proc), None, 0)
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to install keyboard hook: {}", e)
                    })?;
            debug!("Low-level keyboard hook installed");
            Ok(hook)
        }
    }

    unsafe extern "system" fn keyboard_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code >= 0 {
            let kb_struct = &*(lparam.0
                as *const windows::Win32::UI::WindowsAndMessaging::KBDLLHOOKSTRUCT);

            let is_key_down =
                wparam.0 == WM_KEYDOWN as usize || wparam.0 == WM_SYSKEYDOWN as usize;
            let is_key_up =
                wparam.0 == WM_KEYUP as usize || wparam.0 == WM_SYSKEYUP as usize;

            if is_key_down || is_key_up {
                let state = if is_key_down {
                    KeyState::Pressed
                } else {
                    KeyState::Released
                };

                let modifiers = Self::get_current_modifier_state();

                let mut event = KeyEvent::new(
                    kb_struct.scanCode as u16,
                    kb_struct.vkCode as u16,
                    state,
                );
                event.modifiers = modifiers;

                debug!(
                    "Keyboard hook: scan_code={:04X}, vk={:04X}, state={:?}, modifiers={:?}",
                    kb_struct.scanCode,
                    kb_struct.vkCode,
                    state,
                    modifiers
                );

                CURRENT_SENDER.with(|s| {
                    if let Some(ref sender) = *s.borrow() {
                        let _ = sender.send(InputEvent::Key(event));
                    }
                });
            }
        }

        CallNextHookEx(None, code, wparam, lparam)
    }

    /// Run one iteration of the message loop (non-blocking)
    /// Returns Ok(true) if should continue, Ok(false) if WM_QUIT received
    pub fn run_once(&mut self) -> Result<bool> {
        if !self.running {
            self.running = true;
        }

        unsafe {
            let mut msg: MSG = std::mem::zeroed();

            let wait_result =
                MsgWaitForMultipleObjectsEx(None, 1, QS_ALLINPUT, MWMO_INPUTAVAILABLE);

            if wait_result.0 == 0 {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                    if msg.message == WM_QUIT {
                        return Ok(false);
                    }
                    DispatchMessageW(&msg);
                }
            }
        }

        Ok(true)
    }

    /// Stop message loop
    pub fn stop(&mut self) {
        self.running = false;
        if let Some(hook) = self.keyboard_hook.take() {
            unsafe {
                let _ = UnhookWindowsHookEx(hook);
            }
            debug!("Keyboard hook uninstalled");
        }
        let _ = unsafe {
            windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                Some(self.hwnd),
                WM_QUIT,
                WPARAM(0),
                LPARAM(0),
            )
        };
    }

    /// Window procedure
    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => {
                debug!("Raw Input window created");
                LRESULT(0)
            }
            WM_DESTROY => {
                debug!("Raw Input window destroyed");
                LRESULT(0)
            }
            WM_INPUT => {
                Self::handle_raw_input(lparam);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn get_current_modifier_state() -> ModifierState {
        super::get_modifier_state()
    }

    /// Handle Raw Input message
    unsafe fn handle_raw_input(lparam: LPARAM) {
        let mut raw_data: std::mem::MaybeUninit<RAWINPUT> =
            std::mem::MaybeUninit::uninit();
        let mut size: u32 = std::mem::size_of::<RAWINPUT>() as u32;

        let hrawinput: windows::Win32::UI::Input::HRAWINPUT =
            std::mem::transmute(lparam.0);
        let result = GetRawInputData(
            hrawinput,
            RID_INPUT,
            Some(raw_data.as_mut_ptr() as *mut _),
            &mut size,
            std::mem::size_of::<RAWINPUTHEADER>() as u32,
        );

        if result == u32::MAX || result == 0 {
            warn!("GetRawInputData failed: result={}", result);
            return;
        }

        let raw = raw_data.assume_init_ref();

        let device_type = raw.header.dwType;
        debug!("WM_INPUT received: device_type={}", device_type);

        if device_type == RIM_TYPEKEYBOARD.0 {
            let keyboard = &raw.data.keyboard;

            let scan_code = if keyboard.MakeCode != 0 {
                keyboard.MakeCode
            } else {
                // Map from virtual key code
                keyboard.VKey
            };

            let state = if keyboard.Flags & 0x01 == 0 {
                KeyState::Pressed
            } else {
                KeyState::Released
            };

            // Get current modifier state
            let modifiers = Self::get_current_modifier_state();

            let event =
                KeyEvent::new(scan_code, keyboard.VKey, state).with_modifiers(modifiers);

            trace!(
                "Keyboard: scan_code={:04X}, vk={:04X}, state={:?}, modifiers={:?}",
                scan_code,
                keyboard.VKey,
                state,
                modifiers
            );

            // Send event
            CURRENT_SENDER.with(|s| {
                if let Some(ref sender) = *s.borrow() {
                    let _ = sender.send(InputEvent::Key(event));
                }
            });
        } else if device_type == RIM_TYPEMOUSE.0 {
            let mouse = &raw.data.mouse;
            let mouse_inner = mouse.Anonymous.Anonymous;

            // Process mouse events
            if mouse_inner.usButtonFlags != 0 {
                // Button event
                let button = if mouse_inner.usButtonFlags & 0x0001 != 0
                    || mouse_inner.usButtonFlags & 0x0002 != 0
                {
                    Some(MouseButton::Left)
                } else if mouse_inner.usButtonFlags & 0x0004 != 0
                    || mouse_inner.usButtonFlags & 0x0008 != 0
                {
                    Some(MouseButton::Right)
                } else if mouse_inner.usButtonFlags & 0x0010 != 0
                    || mouse_inner.usButtonFlags & 0x0020 != 0
                {
                    Some(MouseButton::Middle)
                } else if mouse_inner.usButtonFlags & 0x0040 != 0
                    || mouse_inner.usButtonFlags & 0x0080 != 0
                {
                    Some(MouseButton::X1)
                } else if mouse_inner.usButtonFlags & 0x0100 != 0
                    || mouse_inner.usButtonFlags & 0x0200 != 0
                {
                    Some(MouseButton::X2)
                } else {
                    None
                };

                if let Some(btn) = button {
                    let is_down = mouse_inner.usButtonFlags & 0x0001 != 0
                        || mouse_inner.usButtonFlags & 0x0004 != 0
                        || mouse_inner.usButtonFlags & 0x0010 != 0
                        || mouse_inner.usButtonFlags & 0x0040 != 0
                        || mouse_inner.usButtonFlags & 0x0100 != 0;

                    let event_type = if is_down {
                        MouseEventType::ButtonDown(btn)
                    } else {
                        MouseEventType::ButtonUp(btn)
                    };

                    let event = MouseEvent::new(event_type, mouse.lLastX, mouse.lLastY);

                    trace!(
                        "Mouse button: {:?}, down={}, x={}, y={}",
                        btn,
                        is_down,
                        mouse.lLastX,
                        mouse.lLastY
                    );

                    // Send event
                    CURRENT_SENDER.with(|s| {
                        if let Some(ref sender) = *s.borrow() {
                            let _ = sender.send(InputEvent::Mouse(event));
                        }
                    });
                }
            }

            // Process wheel
            if mouse_inner.usButtonFlags & 0x0400 != 0 {
                let delta = mouse_inner.usButtonData as i16 as i32;
                let event = MouseEvent::new(
                    MouseEventType::Wheel(delta / WHEEL_DELTA),
                    mouse.lLastX,
                    mouse.lLastY,
                );

                trace!("Mouse wheel: delta={}", delta);

                // Send event
                CURRENT_SENDER.with(|s| {
                    if let Some(ref sender) = *s.borrow() {
                        let _ = sender.send(InputEvent::Mouse(event));
                    }
                });
            }
        }
    }
}

impl Drop for RawInputDevice {
    fn drop(&mut self) {
        self.stop();
        // Clean up thread-local storage
        CURRENT_SENDER.with(|s| {
            *s.borrow_mut() = None;
        });
    }
}
