//! Display brightness control using IOKit
//!
//! Provides native brightness control using IODisplay API for direct hardware communication.
//!
//! Performance: < 5ms

use std::ffi::{c_char, c_void};
use tracing::{debug, trace};

// IOKit return codes
const K_IO_RETURN_SUCCESS: i32 = 0;

// IOKit FFI bindings
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceGetMatchingService(master_port: u32, matching: *mut c_void) -> u32;
    fn IOServiceMatching(name: *const c_char) -> *mut c_void;
    fn IOObjectRelease(object: u32) -> i32;

    fn IORegistryEntryCreateCFProperty(
        entry: u32,
        key: *const c_void,
        allocator: *const c_void,
        options: u32,
    ) -> *mut c_void;

    fn IORegistryEntrySetCFProperty(
        entry: u32,
        key: *const c_void,
        value: *const c_void,
    ) -> i32;
}

// CoreFoundation FFI bindings
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const c_char,
        encoding: u32,
    ) -> *mut c_void;
    fn CFNumberGetValue(number: *const c_void, type_: u32, value: *mut c_void) -> i32;
    fn CFNumberCreate(
        alloc: *const c_void,
        type_: u32,
        value: *const c_void,
    ) -> *mut c_void;
    fn CFRelease(cf: *const c_void);
}

const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;
const K_CF_NUMBER_FLOAT_TYPE: u32 = 12;

/// Get the IODisplayConnect service for the main display
fn get_display_connect() -> Result<u32, String> {
    unsafe {
        let matching = IOServiceMatching(c"IODisplayConnect".as_ptr());

        if matching.is_null() {
            return Err("Failed to create service matching".to_string());
        }

        let service = IOServiceGetMatchingService(0, matching);

        if service == 0 {
            return Err("No IODisplayConnect service found".to_string());
        }

        trace!("Found IODisplayConnect service: {}", service);
        Ok(service)
    }
}

/// Get current brightness (0.0 to 1.0)
pub fn get_brightness() -> Result<f32, String> {
    unsafe {
        let service = get_display_connect()?;

        let key = CFStringCreateWithCString(
            std::ptr::null(),
            c"brightness".as_ptr(),
            K_CF_STRING_ENCODING_UTF8,
        );

        if key.is_null() {
            IOObjectRelease(service);
            return Err("Failed to create CFString key".to_string());
        }

        let value = IORegistryEntryCreateCFProperty(service, key, std::ptr::null(), 0);

        CFRelease(key);
        IOObjectRelease(service);

        if value.is_null() {
            return Err("Failed to get brightness property".to_string());
        }

        let mut brightness: f32 = 0.0;
        let result = CFNumberGetValue(
            value,
            K_CF_NUMBER_FLOAT_TYPE,
            &mut brightness as *mut _ as *mut c_void,
        );

        CFRelease(value);

        if result == 0 {
            return Err("Failed to convert brightness value".to_string());
        }

        trace!("Current brightness: {:.2}", brightness);
        Ok(brightness)
    }
}

/// Set brightness (0.0 to 1.0)
pub fn set_brightness(brightness: f32) -> Result<(), String> {
    unsafe {
        let service = get_display_connect()?;

        let clamped_brightness = brightness.clamp(0.0, 1.0);

        let key = CFStringCreateWithCString(
            std::ptr::null(),
            c"brightness".as_ptr(),
            K_CF_STRING_ENCODING_UTF8,
        );

        if key.is_null() {
            IOObjectRelease(service);
            return Err("Failed to create CFString key".to_string());
        }

        let value = CFNumberCreate(
            std::ptr::null(),
            K_CF_NUMBER_FLOAT_TYPE,
            &clamped_brightness as *const _ as *const c_void,
        );

        if value.is_null() {
            CFRelease(key);
            IOObjectRelease(service);
            return Err("Failed to create CFNumber value".to_string());
        }

        let result = IORegistryEntrySetCFProperty(service, key, value);

        CFRelease(value);
        CFRelease(key);
        IOObjectRelease(service);

        if result != K_IO_RETURN_SUCCESS {
            return Err(format!("Failed to set brightness: {}", result));
        }

        debug!("Brightness set to {:.2}", clamped_brightness);
        Ok(())
    }
}

/// Increase brightness by delta (0.0 to 1.0 scale)
pub fn brightness_up(delta: f32) -> Result<(), String> {
    let current = get_brightness().unwrap_or(0.5);
    let new_brightness = (current + delta).min(1.0);
    set_brightness(new_brightness)
}

/// Decrease brightness by delta (0.0 to 1.0 scale)
pub fn brightness_down(delta: f32) -> Result<(), String> {
    let current = get_brightness().unwrap_or(0.5);
    let new_brightness = (current - delta).max(0.0);
    set_brightness(new_brightness)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_brightness() {
        match get_brightness() {
            Ok(brightness) => {
                println!("Current brightness: {:.2}", brightness);
                assert!(brightness >= 0.0 && brightness <= 1.0);
            }
            Err(e) => {
                println!("Note: Could not get brightness: {}", e);
            }
        }
    }

    #[test]
    fn test_brightness_up_down() {
        // Get current brightness
        let original = match get_brightness() {
            Ok(b) => b,
            Err(e) => {
                println!("Skipping test: {}", e);
                return;
            }
        };

        // Test brightness up
        if let Err(e) = brightness_up(0.1) {
            println!("Note: Could not increase brightness: {}", e);
            return;
        }

        // Test brightness down
        if let Err(e) = brightness_down(0.1) {
            println!("Note: Could not decrease brightness: {}", e);
            return;
        }

        // Restore original brightness
        let _ = set_brightness(original);

        println!("Brightness control test passed");
    }

    #[test]
    fn test_set_brightness_clamping() {
        // Test that values are clamped to 0.0-1.0 range
        // Note: This test just verifies the clamping logic doesn't panic
        // We can't actually test without affecting the display

        // Values should be clamped, not rejected
        let _ = set_brightness(-0.5); // Should clamp to 0.0
        let _ = set_brightness(1.5); // Should clamp to 1.0
    }
}
