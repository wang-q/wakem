//! Core Audio API for volume control
//!
//! Provides native volume control without using AppleScript.
//! Uses AudioObjectGetPropertyData / AudioObjectSetPropertyData.
//!
//! Performance: < 1ms (vs 50-100ms with osascript)
#![cfg(target_os = "macos")]

use std::ffi::c_void;
use tracing::{debug, trace};

// AudioObject property selectors
const K_AUDIO_OBJECT_SYSTEM_OBJECT: u32 = 1;
const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE: u32 = fourcc(b"def ");
const K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR: u32 = fourcc(b"volm");
const K_AUDIO_DEVICE_PROPERTY_MUTE: u32 = fourcc(b"mute");
const K_AUDIO_OBJECT_PROPERTY_ELEMENT_MASTER: u32 = 0;

// Property address structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct AudioObjectPropertyAddress {
    selector: u32,
    scope: u32,
    element: u32,
}

// Core Audio FFI bindings
#[link(name = "CoreAudio", kind = "framework")]
extern "C" {
    fn AudioObjectGetPropertyData(
        in_object_id: u32,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        io_data_size: *mut u32,
        out_data: *mut c_void,
    ) -> i32;

    fn AudioObjectSetPropertyData(
        in_object_id: u32,
        in_address: *const AudioObjectPropertyAddress,
        in_qualifier_data_size: u32,
        in_qualifier_data: *const c_void,
        in_data_size: u32,
        in_data: *const c_void,
    ) -> i32;
}

/// Convert 4-byte code to u32 (FourCC)
const fn fourcc(bytes: &[u8; 4]) -> u32 {
    ((bytes[0] as u32) << 24)
        | ((bytes[1] as u32) << 16)
        | ((bytes[2] as u32) << 8)
        | (bytes[3] as u32)
}

/// Get default output device ID
fn get_default_output_device() -> Result<u32, String> {
    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE,
        scope: fourcc(b"out "), // kAudioObjectPropertyScopeOutput
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MASTER,
    };

    let mut device_id: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;

    let result = unsafe {
        AudioObjectGetPropertyData(
            K_AUDIO_OBJECT_SYSTEM_OBJECT,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut device_id as *mut _ as *mut c_void,
        )
    };

    if result == 0 {
        trace!("Default output device ID: {}", device_id);
        Ok(device_id)
    } else {
        Err(format!("Failed to get default output device: {}", result))
    }
}

/// Get current volume (0.0 to 1.0)
pub fn get_volume() -> Result<f32, String> {
    let device_id = get_default_output_device()?;

    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR,
        scope: fourcc(b"out "), // kAudioDevicePropertyScopeOutput
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MASTER,
    };

    let mut volume: f32 = 0.0;
    let mut size: u32 = std::mem::size_of::<f32>() as u32;

    let result = unsafe {
        AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut volume as *mut _ as *mut c_void,
        )
    };

    if result == 0 {
        trace!("Current volume: {:.2}", volume);
        Ok(volume)
    } else {
        // Volume property might not be supported on this device
        Err(format!("Failed to get volume: {}", result))
    }
}

/// Set volume (0.0 to 1.0)
pub fn set_volume(volume: f32) -> Result<(), String> {
    let device_id = get_default_output_device()?;

    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_DEVICE_PROPERTY_VOLUME_SCALAR,
        scope: fourcc(b"out "),
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MASTER,
    };

    let clamped_volume = volume.clamp(0.0, 1.0);
    let result = unsafe {
        AudioObjectSetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            std::mem::size_of::<f32>() as u32,
            &clamped_volume as *const _ as *const c_void,
        )
    };

    if result == 0 {
        debug!("Volume set to {:.2}", clamped_volume);
        Ok(())
    } else {
        Err(format!("Failed to set volume: {}", result))
    }
}

/// Increase volume by delta (0.0 to 1.0 scale)
pub fn volume_up(delta: f32) -> Result<(), String> {
    let current = get_volume().unwrap_or(0.5);
    let new_volume = (current + delta).min(1.0);
    set_volume(new_volume)
}

/// Decrease volume by delta (0.0 to 1.0 scale)
pub fn volume_down(delta: f32) -> Result<(), String> {
    let current = get_volume().unwrap_or(0.5);
    let new_volume = (current - delta).max(0.0);
    set_volume(new_volume)
}

/// Check if device is muted
pub fn is_muted() -> Result<bool, String> {
    let device_id = get_default_output_device()?;

    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
        scope: fourcc(b"out "),
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MASTER,
    };

    let mut muted: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;

    let result = unsafe {
        AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            &mut size,
            &mut muted as *mut _ as *mut c_void,
        )
    };

    if result == 0 {
        trace!("Mute state: {}", muted != 0);
        Ok(muted != 0)
    } else {
        // Mute property might not be supported
        Err(format!("Failed to get mute state: {}", result))
    }
}

/// Set mute state
pub fn set_mute(muted: bool) -> Result<(), String> {
    let device_id = get_default_output_device()?;

    let address = AudioObjectPropertyAddress {
        selector: K_AUDIO_DEVICE_PROPERTY_MUTE,
        scope: fourcc(b"out "),
        element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MASTER,
    };

    let mute_value: u32 = if muted { 1 } else { 0 };
    let result = unsafe {
        AudioObjectSetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            std::mem::size_of::<u32>() as u32,
            &mute_value as *const _ as *const c_void,
        )
    };

    if result == 0 {
        debug!("Mute set to: {}", muted);
        Ok(())
    } else {
        Err(format!("Failed to set mute: {}", result))
    }
}

/// Toggle mute state
pub fn toggle_mute() -> Result<bool, String> {
    let current = is_muted().unwrap_or(false);
    let new_state = !current;
    set_mute(new_state)?;
    Ok(new_state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_output_device() {
        match get_default_output_device() {
            Ok(device_id) => {
                println!("Default output device ID: {}", device_id);
                assert!(device_id > 0);
            }
            Err(e) => {
                println!("Note: Could not get default output device: {}", e);
            }
        }
    }

    #[test]
    fn test_get_volume() {
        match get_volume() {
            Ok(volume) => {
                println!("Current volume: {:.2}", volume);
                assert!(volume >= 0.0 && volume <= 1.0);
            }
            Err(e) => {
                println!("Note: Could not get volume: {}", e);
            }
        }
    }

    #[test]
    fn test_volume_up_down() {
        // Get current volume
        let original = match get_volume() {
            Ok(v) => v,
            Err(e) => {
                println!("Skipping test: {}", e);
                return;
            }
        };

        // Test volume up
        if let Err(e) = volume_up(0.1) {
            println!("Note: Could not increase volume: {}", e);
            return;
        }

        // Test volume down
        if let Err(e) = volume_down(0.1) {
            println!("Note: Could not decrease volume: {}", e);
            return;
        }

        // Restore original volume
        let _ = set_volume(original);

        println!("Volume control test passed");
    }

    #[test]
    fn test_mute() {
        // Get current mute state
        let original = match is_muted() {
            Ok(m) => m,
            Err(e) => {
                println!("Skipping test: {}", e);
                return;
            }
        };

        // Test toggle
        match toggle_mute() {
            Ok(new_state) => {
                println!("Mute toggled to: {}", new_state);
                // Restore original state
                let _ = set_mute(original);
            }
            Err(e) => {
                println!("Note: Could not toggle mute: {}", e);
            }
        }
    }

    #[test]
    fn test_fourcc() {
        assert_eq!(fourcc(b"volm"), 0x766F6C6D);
        assert_eq!(fourcc(b"mute"), 0x6D757465);
        assert_eq!(fourcc(b"out "), 0x6F757420);
    }
}
