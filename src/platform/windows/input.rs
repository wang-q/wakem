use crate::types::{
    InputEvent, KeyEvent, KeyState, ModifierState, MouseButton, MouseEvent,
    MouseEventType,
};
use anyhow::Result;
use std::cell::RefCell;
use std::sync::mpsc::Sender;
use tracing::{debug, trace};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::{
    GetRawInputData, RegisterRawInputDevices, RIDEV_INPUTSINK, RIDEV_NOLEGACY,
};
use windows::Win32::UI::Input::{
    RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, MSG, WM_CREATE, WM_DESTROY, WM_INPUT,
    WM_QUIT, WNDCLASSW, WS_EX_NOACTIVATE, WS_OVERLAPPEDWINDOW,
};

thread_local! {
    static CURRENT_SENDER: RefCell<Option<Sender<InputEvent>>> = const { RefCell::new(None) };
}

/// Raw Input device manager
pub struct RawInputDevice {
    hwnd: HWND,
    #[allow(dead_code)]
    event_sender: Sender<InputEvent>,
    #[allow(dead_code)]
    modifier_state: ModifierState,
    running: bool,
}

impl RawInputDevice {
    /// Create and initialize Raw Input device
    pub fn new(event_sender: Sender<InputEvent>) -> Result<Self> {
        // Set thread-local sender
        CURRENT_SENDER.with(|s| {
            *s.borrow_mut() = Some(event_sender.clone());
        });

        let hwnd = Self::create_message_window()?;

        // Register Raw Input devices
        Self::register_devices(hwnd)?;

        Ok(Self {
            hwnd,
            event_sender,
            modifier_state: ModifierState::default(),
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
                hinstance,
                None,
            );

            if hwnd.0 == 0 {
                return Err(anyhow::anyhow!("Failed to create window"));
            }

            debug!("Raw Input message window created: {:?}", hwnd);
            Ok(hwnd)
        }
    }

    /// Register Raw Input devices
    fn register_devices(hwnd: HWND) -> Result<()> {
        let devices = [
            RAWINPUTDEVICE {
                usUsagePage: 0x01, // Generic Desktop
                usUsage: 0x06,     // Keyboard
                dwFlags: RIDEV_INPUTSINK | RIDEV_NOLEGACY,
                hwndTarget: hwnd,
            },
            RAWINPUTDEVICE {
                usUsagePage: 0x01, // Generic Desktop
                usUsage: 0x02,     // Mouse
                dwFlags: RIDEV_INPUTSINK | RIDEV_NOLEGACY,
                hwndTarget: hwnd,
            },
        ];

        unsafe {
            RegisterRawInputDevices(
                &devices,
                std::mem::size_of::<RAWINPUTDEVICE>() as u32,
            )?;
        }

        debug!("Raw Input devices registered successfully");
        Ok(())
    }

    /// Run message loop
    pub fn run(&mut self) -> Result<()> {
        debug!("Starting Raw Input message loop");
        self.running = true;

        unsafe {
            let mut msg: MSG = std::mem::zeroed();

            while self.running && GetMessageW(&mut msg, None, 0, 0).into() {
                DispatchMessageW(&msg);
            }
        }

        debug!("Raw Input message loop ended");
        Ok(())
    }

    /// Stop message loop
    pub fn stop(&mut self) {
        self.running = false;
        let _ = unsafe {
            windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                self.hwnd,
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
                // Process Raw Input
                Self::handle_raw_input(lparam);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// Handle Raw Input message
    unsafe fn handle_raw_input(lparam: LPARAM) {
        let mut raw_data: [u8; 1024] = [0; 1024];
        let mut size: u32 = 1024;

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
            return;
        }

        let raw = &*(raw_data.as_ptr() as *const RAWINPUT);

        let device_type = raw.header.dwType;

        if device_type == RIM_TYPEKEYBOARD.0 {
            let keyboard = &raw.data.keyboard;

            // Ignore repeated keys and virtual keys
            if keyboard.Flags & 0x01 != 0 {
                // This is a virtual key, ignore
                return;
            }

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

            let event = KeyEvent::new(scan_code, keyboard.VKey, state);

            trace!(
                "Keyboard: scan_code={:04X}, vk={:04X}, state={:?}",
                scan_code,
                keyboard.VKey,
                state
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
                    MouseEventType::Wheel(delta / 120),
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

    /// Update modifier key state
    #[allow(dead_code)]
    fn update_modifier_state(&mut self, virtual_key: u16, pressed: bool) {
        if let Some((modifier, _)) =
            ModifierState::from_virtual_key(virtual_key, pressed)
        {
            self.modifier_state.merge(&modifier);
        }
    }

    /// Get current modifier key state
    #[allow(dead_code)]
    pub fn get_modifier_state(&self) -> &ModifierState {
        &self.modifier_state
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
