use anyhow::Result;
use std::cell::RefCell;
use std::sync::mpsc::Sender;
use tracing::{debug, trace};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, KBDLLHOOKSTRUCT_FLAGS,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};
use crate::types::{InputEvent, KeyEvent, KeyState};

thread_local! {
    static HOOK_SENDER: RefCell<Option<Sender<(InputEvent, bool)>>> = RefCell::new(None);
    static HOOK_HANDLE: RefCell<Option<HHOOK>> = RefCell::new(None);
}

/// 低级键盘钩子管理器
pub struct KeyboardHook {
    hook_handle: Option<HHOOK>,
}

impl KeyboardHook {
    /// 创建并安装键盘钩子
    pub fn new(event_sender: Sender<(InputEvent, bool)>) -> Result<Self> {
        // 设置线程本地存储的发送器
        HOOK_SENDER.with(|s| {
            *s.borrow_mut() = Some(event_sender);
        });

        unsafe {
            let hinstance = HINSTANCE(
                windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?.0
            );

            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(Self::hook_proc),
                hinstance,
                0,
            )?;

            HOOK_HANDLE.with(|h| {
                *h.borrow_mut() = Some(hook);
            });

            debug!("Low level keyboard hook installed");

            Ok(Self {
                hook_handle: Some(hook),
            })
        }
    }

    /// 钩子过程
    unsafe extern "system" fn hook_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code < 0 {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let kb_struct = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        
        // 忽略注入的事件（避免循环）
        if kb_struct.flags.0 & 0x10 != 0 {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        let vk_code = kb_struct.vkCode as u16;
        let scan_code = kb_struct.scanCode as u16;

        let state = match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => KeyState::Pressed,
            WM_KEYUP | WM_SYSKEYUP => KeyState::Released,
            _ => return CallNextHookEx(None, code, wparam, lparam),
        };

        let event = KeyEvent::new(scan_code, vk_code, state);
        
        trace!(
            "Hook: vk={:04X}, scan={:04X}, state={:?}",
            vk_code, scan_code, state
        );

        // 发送事件（非阻塞）
        // 注意：钩子中不能阻塞等待，所以采用"先发送后查询"的策略
        // 实际阻止逻辑在 process_input_event 中通过 OutputDevice 实现
        HOOK_SENDER.with(|s| {
            if let Some(ref sender) = *s.borrow() {
                let _ = sender.send((InputEvent::Key(event), false));
            }
        });

        // 暂时不阻止任何事件（后续通过反向按键抵消）
        CallNextHookEx(None, code, wparam, lparam)
    }

    /// 运行消息循环（必须在安装钩子的线程中运行）
    pub fn run_message_loop(&self) -> Result<()> {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetMessageW, DispatchMessageW, TranslateMessage, MSG,
        };

        debug!("Starting hook message loop");

        unsafe {
            let mut msg: MSG = std::mem::zeroed();

            while GetMessageW(&mut msg, None, 0, 0).into() {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        debug!("Hook message loop ended");
        Ok(())
    }

    /// 停止钩子
    pub fn uninstall(&mut self) {
        if let Some(hook) = self.hook_handle.take() {
            unsafe {
                let _ = UnhookWindowsHookEx(hook);
            }
            debug!("Keyboard hook uninstalled");
        }

        // 清理线程本地存储
        HOOK_SENDER.with(|s| {
            *s.borrow_mut() = None;
        });
        HOOK_HANDLE.with(|h| {
            *h.borrow_mut() = None;
        });
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        self.uninstall();
    }
}

/// 检查钩子是否已安装
pub fn is_hook_installed() -> bool {
    HOOK_HANDLE.with(|h| h.borrow().is_some())
}
