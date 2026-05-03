//! macOS platform utilities and context provider
#![cfg(target_os = "macos")]

use crate::platform::traits::{ContextProvider, PlatformUtilities};

pub fn get_modifier_state() -> crate::types::ModifierState {
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let mut modifiers = crate::types::ModifierState::default();

    if let Ok(source) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) {
        if let Ok(event) = core_graphics::event::CGEvent::new(source) {
            let flags = event.get_flags();

            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagShift) {
                modifiers.shift = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagControl) {
                modifiers.ctrl = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagAlternate) {
                modifiers.alt = true;
            }
            if flags.contains(core_graphics::event::CGEventFlags::CGEventFlagCommand) {
                modifiers.meta = true;
            }
        }
    }

    modifiers
}

pub fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
    <MacosPlatform as PlatformUtilities>::get_process_name_by_pid(pid)
}

pub fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
    <MacosPlatform as PlatformUtilities>::get_executable_path_by_pid(pid)
}

pub struct MacosPlatform;

impl PlatformUtilities for MacosPlatform {
    fn get_modifier_state() -> crate::types::ModifierState {
        get_modifier_state()
    }

    fn get_process_name_by_pid(pid: u32) -> anyhow::Result<String> {
        super::native_api::ns_workspace::get_process_name_by_pid(pid)
    }

    fn get_executable_path_by_pid(pid: u32) -> anyhow::Result<String> {
        super::native_api::ns_workspace::get_executable_path_by_pid(pid)
    }
}

impl ContextProvider for MacosPlatform {
    crate::impl_context_provider!();
}
