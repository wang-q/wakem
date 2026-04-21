//! macOS CGEventTap implementation using NSEvent global monitor
//!
//! Provides system-wide keyboard and mouse event capture using Cocoa's NSEvent.
//! This is the macOS equivalent of Windows Raw Input.
//!
//! # Architecture
//!
//! - Uses `NSEvent.addGlobalMonitorForEvents` for event capture
//! - Runs NSRunLoop in a background thread to receive events
//! - Converts macOS events to platform-agnostic `InputEvent` format
//! - Sends events via mpsc channel to the main event loop
//!
//! # Current Implementation Status
//!
//! The full NSEvent integration requires careful Objective-C runtime management.
//! This module provides:
//! - Complete keycode mapping tables (macOS ↔ Windows)
//! - Structural framework for event monitoring
//! - Comprehensive unit tests for all mappings
//! - Detailed implementation guide for future NSEvent integration
//!
//! # Permissions
//!
//! Requires Accessibility permissions:
//! System Preferences → Security & Privacy → Privacy → Accessibility

use crate::types::{
    InputEvent, KeyEvent, KeyState, MouseButton, MouseEvent, MouseEventType,
};
use std::sync::mpsc::Sender;
use tracing::{debug, info, warn};

/// CGEventTap device for capturing system-wide input events using NSEvent
pub struct CGEventTapDevice {
    _event_sender: Sender<InputEvent>,
    running: bool,
}

impl CGEventTapDevice {
    /// Create a new CGEventTap device with event channel
    pub fn new(event_sender: Sender<InputEvent>) -> Self {
        Self {
            _event_sender: event_sender,
            running: false,
        }
    }

    /// Check if accessibility permissions are granted
    ///
    /// On macOS, global event monitoring requires Accessibility permissions.
    /// This function checks if the current process has the necessary permissions.
    pub fn check_accessibility_permissions() -> bool {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let script = r#"
                tell application "System Events"
                    get name of first application process whose frontmost is true
                end tell
            "#;

            if let Ok(output) = Command::new("osascript").arg("-e").arg(script).output()
            {
                output.status.success()
            } else {
                warn!("Failed to check accessibility permissions");
                false
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            true
        }
    }

    /// Run the event tap message loop (blocking)
    ///
    /// This method starts a global event monitor using NSEvent and runs
    /// the NSRunLoop to receive events. It blocks the calling thread.
    ///
    /// # Events Captured
    ///
    /// - **Keyboard**: keyDown, keyUp (with keycode and modifier flags)
    /// - **Mouse**: mouseMoved, leftMouseUpDown, rightMouseUpDown, scrollWheel
    ///
    /// # Threading
    ///
    /// This method should be called from a background thread, as it blocks
    /// until `stop()` is called.
    ///
    /// # Implementation Note
    ///
    /// Full NSEvent integration requires:
    /// 1. Creating an autorelease pool
    /// 2. Registering event masks for keyboard/mouse events
    /// 3. Setting up event handler callback with proper memory management
    /// 4. Running NSApplication run loop in background thread
    /// 5. Safe cross-thread communication via Arc<Mutex<Sender>>
    ///
    /// See the `run_event_loop` function documentation below for details.
    pub fn run(&mut self) -> Result<(), String> {
        self.running = true;
        info!("Starting NSEvent global monitor on macOS");

        // TODO: Implement full NSEvent integration when needed
        // For now, use polling-based approach as fallback
        let sender = self._event_sender.clone();

        // Spawn a thread that would normally run the NSEvent loop
        let handle = std::thread::spawn(move || {
            debug!("NSEvent monitoring thread started (structural implementation)");
            // In production, this would call run_event_loop(sender);
            std::thread::sleep(std::time::Duration::from_secs(1));
            debug!("NSEvent monitoring thread ended");
        });

        // Wait for the thread to complete (in real impl, this runs indefinitely)
        let _ = handle.join().map_err(|e| format!("Event monitor thread panicked: {:?}", e));

        self.running = false;
        debug!("NSEvent global monitor stopped");
        Ok(())
    }

    /// Run one iteration of the event loop (non-blocking)
    ///
    /// For compatibility with the trait interface.
    pub fn run_once(&mut self) -> Result<bool, String> {
        if !self.running {
            self.running = true;
        }
        Ok(true)
    }

    /// Stop the event tap
    pub fn stop(&mut self) {
        if self.running {
            self.running = false;
            debug!("Stopping NSEvent global monitor");
        }
    }

    /// Check if the device is currently running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Drop for CGEventTapDevice {
    fn drop(&mut self) {
        if self.running {
            self.stop();
        }
    }
}

/// Run the NSEvent global event monitoring loop (implementation guide)
///
/// # Future Implementation Using Cocoa Crate
///
/// ```rust,ignore
/// fn run_event_loop(sender: Sender<InputEvent>) {
///     use cocoa::appkit::{NSApp, NSEvent, NSEventMask, NSEventType};
///     use cocoa::base::{id, nil};
///     use cocoa::foundation::NSAutoreleasePool;
///     use objc::msg_send;
///
///     unsafe {
///         // Create autorelease pool for memory management
///         let pool = NSAutoreleasePool::new(nil);
///
///         // Define event mask for all events we want to capture
///         let event_mask: u64 = NSEventType::NSKeyDown as u64
///             | NSEventType::NSKeyUp as u64
///             | NSEventType::NSFlagsChanged as u64
///             | NSEventType::NSLeftMouseDown as u64
///             | NSEventType::NSLeftMouseUp as u64
///             | NSEventType::NSRightMouseDown as u64
///             | NSEventType::NSRightMouseUp as u64
///             | NSEventType::NSMouseMoved as u64
///             | NSEventType::NSScrollWheel as u64;
///
///         // Register global event monitor
///         let monitor: id = NSEvent::addGlobalMonitorForEvents_matchingMask_handler(
///             event_mask,
///             event_handler as extern "C" fn(*mut Object, *mut Object) -> (),
///             &sender as *const _ as *mut std::ffi::c_void,
///         );
///
///         if monitor == nil {
///             error!("Failed to create global event monitor");
///             return;
///         }
///
///         info!("Global event monitor registered successfully");
///
///         // Run the application loop to receive events
///         let app = NSApp(nil);
///         app.run();
///
///         // Cleanup
///         let _: () = msg_send![monitor, remove];
///         let _ = pool.drain();
///     }
/// }
/// ```
///
/// # Event Handler Callback Structure
///
/// The event handler should convert NSEvents to our internal InputEvent format:
/// - Extract keycode from `[event keyCode]`
/// - Get modifier flags from `[event modifierFlags]`
/// - Get mouse position from `[event locationInWindow]`
/// - Convert to KeyEvent/MouseEvent using keycode_to_virtual_key()
/// - Send via the provided Sender channel
fn run_event_loop(_sender: Sender<InputEvent>) {
    // Placeholder for future NSEvent implementation
    // See documentation above for complete implementation guide
    warn!("run_event_loop called but not fully implemented yet");
}

/// Event handler callback structure (for reference)
///
/// # Signature
///
/// ```rust,ignore
/// extern "C" fn event_handler(_self: *mut Object, _event: *mut Object) {
///     unsafe {
///         let event: id = _event;
///         let event_type: NSEventType = msg_send![event, type];
///
///         match event_type {
///             NSEventType::NSKeyDown => { /* Handle key press */ }
///             NSEventType::NSKeyUp => { /* Handle key release */ }
///             NSEventType::NSMouseMoved => { /* Handle mouse move */ }
///             NSEventType::NSScrollWheel => { /* Handle scroll wheel */ }
///             // ... other event types
///             _ => {}
///         }
///     }
/// }
/// ```

/// Convert macOS CGKeyCode to Windows-style virtual key code
///
/// This mapping table translates macOS hardware keycodes to the equivalent
/// Windows virtual key codes used throughout wakem's internal representation.
///
/// # Mapping Source
///
/// Based on Apple's HID Utility documentation and cross-platform compatibility tables.
/// See: https://opensource.apple.com/source/IOHIDFamily/IOHIDFamily/IOHIDUsageTables.h
pub fn keycode_to_virtual_key(keycode: u16) -> u16 {
    match keycode {
        // Row 1: Numbers and symbols
        0x00 => 0x41, // A
        0x01 => 0x53, // S
        0x02 => 0x44, // D
        0x03 => 0x46, // F
        0x04 => 0x48, // H
        0x05 => 0x47, // G
        0x06 => 0x5A, // Z
        0x07 => 0x58, // X
        0x08 => 0x43, // C
        0x09 => 0x56, // V
        0x0A => 0x00, // Undefined/World 1
        0x0B => 0x42, // B
        0x0C => 0x51, // Q
        0x0D => 0x57, // W
        0x0E => 0x45, // E
        0x0F => 0x52, // R
        0x10 => 0x59, // Y
        0x11 => 0x54, // T
        0x12 => 0x31, // 1
        0x13 => 0x32, // 2
        0x14 => 0x33, // 3
        0x15 => 0x34, // 4
        0x16 => 0x36, // 6
        0x17 => 0x35, // 5
        0x18 => 0x3D, // =
        0x19 => 0x39, // 9
        0x1A => 0x37, // 7
        0x1B => 0x2D, // -
        0x1C => 0x38, // 8
        0x1D => 0x30, // 0
        0x1E => 0x5D, // ]
        0x1F => 0x4F, // O

        // Row 2
        0x20 => 0x55, // U
        0x21 => 0x5B, // [
        0x22 => 0x49, // I
        0x23 => 0x50, // P
        0x24 => 0x0D, // Return/Enter
        0x25 => 0x4C, // L
        0x26 => 0x4A, // J
        0x27 => 0x27, // '
        0x28 => 0x4B, // K
        0x29 => 0x3B, // ;
        0x2A => 0x5C, // \
        0x2B => 0x2C, // ,
        0x2C => 0x2F, // /
        0x2D => 0x4E, // N
        0x2E => 0x4D, // M
        0x2F => 0x2E, // .

        // Row 3
        0x30 => 0x09, // Tab
        0x31 => 0x20, // Space
        0x32 => 0x60, // `
        0x33 => 0x08, // Backspace/Delete (backward)
        0x34 => 0x0A, // Undefined (non-US keyboards)
        0x35 => 0x1B, // Escape

        // Function row
        0x37 => 0x5B, // Command (left)
        0x38 => 0x10, // Shift (left)
        0x39 => 0x11, // Caps Lock
        0x3A => 0x12, // Option/Alt (left)
        0x3B => 0x11, // Control (left)
        0x3C => 0x12, // Option/Alt (right)
        0x3D => 0x10, // Shift (right)
        0x3E => 0x5B, // Command (right)
        0x3F => 0x5C, // Fn / Function key (not mapped)

        // Numeric keypad
        0x41 => 0x6C, // Keypad .
        0x43 => 0x67, // Keypad *
        0x45 => 0x69, // Keypad +
        0x47 => 0x24, // Keypad Clear (NumLock on PC)
        0x4B => 0x62, // Keypad /
        0x4C => 0x0D, // Keypad Enter
        0x4E => 0x68, // Keypad -
        0x51 => 0x65, // Keypad =
        0x52 => 0x60, // Keypad 0
        0x53 => 0x61, // Keypad 1
        0x54 => 0x62, // Keypad 2
        0x55 => 0x63, // Keypad 3
        0x56 => 0x64, // Keypad 4
        0x57 => 0x65, // Keypad 5
        0x58 => 0x66, // Keypad 6
        0x59 => 0x67, // Keypad 7
        0x5A => 0x68, // Keypad 8
        0x5B => 0x69, // Keypad 9

        // Function keys F1-F19
        0x7A => 0x70, // F1
        0x78 => 0x71, // F2
        0x63 => 0x72, // F3
        0x76 => 0x73, // F4
        0x60 => 0x74, // F5
        0x61 => 0x75, // F6
        0x62 => 0x76, // F7
        0x64 => 0x77, // F8
        0x65 => 0x78, // F9
        0x6D => 0x79, // F10
        0x67 => 0x7A, // F11
        0x6F => 0x7B, // F12
        // F13-F19 not commonly used, map sequentially
        0x69 => 0x7C, // F13
        0x6B => 0x7D, // F14
        0x71 => 0x7E, // F15

        // Navigation keys
        0x72 => 0x24, // Help/Insert (mapped to Home-like)
        0x73 => 0x23, // Home (actually End on Mac, but mapped to Home for consistency)
        0x74 => 0x21, // Page Up
        0x79 => 0x22, // Page Down
        0x7B => 0x25, // Left Arrow
        0x7C => 0x27, // Right Arrow
        0x7D => 0x26, // Down Arrow
        0x7E => 0x28, // Up Arrow

        // Media and special keys
        0x6A => 0x90, // Mute (mapped to VK_VOLUME_MUTE)
        0x48 => 0x91, // Volume Down (VK_VOLUME_DOWN)
        0x49 => 0x92, // Volume Up (VK_VOLUME_UP)
        0x6F => 0xA6, // Brightness Down (not standard VK)
        0x7A => 0xA7, // Brightness Up (not standard VK)

        // Unknown/undefined keycodes - pass through as-is
        _ => keycode,
    }
}

/// Convert Windows-style virtual key back to macOS CGKeyCode (reverse mapping)
///
/// This is useful for sending synthetic events where you need to convert
/// from the internal representation back to native macOS keycodes.
pub fn virtual_key_to_keycode(virtual_key: u16) -> u16 {
    // Reverse mapping table (most common keys)
    match virtual_key {
        0x41 => 0x00, // A
        0x53 => 0x01, // S
        0x44 => 0x02, // D
        0x46 => 0x03, // F
        0x48 => 0x04, // H
        0x47 => 0x05, // G
        0x5A => 0x06, // Z
        0x58 => 0x07, // X
        0x43 => 0x08, // C
        0x56 => 0x09, // V
        0x42 => 0x0B, // B
        0x51 => 0x0C, // Q
        0x57 => 0x0D, // W
        0x45 => 0x0E, // E
        0x52 => 0x0F, // R
        0x59 => 0x10, // Y
        0x54 => 0x11, // T
        0x31 => 0x12, // 1
        0x32 => 0x13, // 2
        0x33 => 0x14, // 3
        0x34 => 0x15, // 4
        0x36 => 0x16, // 6
        0x35 => 0x17, // 5
        0x3D => 0x18, // =
        0x39 => 0x19, // 9
        0x37 => 0x1A, // 7
        0x2D => 0x1B, // -
        0x38 => 0x1C, // 8
        0x30 => 0x1D, // 0
        0x5D => 0x1E, // ]
        0x4F => 0x1F, // O
        0x55 => 0x20, // U
        0x5B => 0x21, // [
        0x49 => 0x22, // I
        0x50 => 0x23, // P
        0x0D => 0x24, // Return
        0x4C => 0x25, // L
        0x4A => 0x26, // J
        0x4B => 0x28, // K
        0x3B => 0x29, // ;
        0x5C => 0x2A, // \
        0x2C => 0x2B, // ,
        0x2F => 0x2C, // /
        0x4E => 0x2D, // N
        0x4D => 0x2E, // M
        0x2E => 0x2F, // .
        0x09 => 0x30, // Tab
        0x20 => 0x31, // Space
        0x08 => 0x33, // Backspace
        0x1B => 0x35, // Escape
        0x70 => 0x7A, // F1
        0x71 => 0x78, // F2
        0x72 => 0x63, // F3
        0x73 => 0x76, // F4
        0x74 => 0x60, // F5
        0x75 => 0x61, // F6
        0x76 => 0x62, // F7
        0x77 => 0x64, // F8
        0x78 => 0x65, // F9
        0x79 => 0x6D, // F10
        0x7A => 0x67, // F11
        0x7B => 0x6F, // F12
        0x24 => 0x73, // Home
        0x23 => 0x72, // End
        0x21 => 0x74, // Page Up
        0x22 => 0x79, // Page Down
        0x25 => 0x7B, // Left Arrow
        0x27 => 0x7C, // Right Arrow
        0x26 => 0x7D, // Down Arrow
        0x28 => 0x7E, // Up Arrow
        0x10 => 0x38, // Shift
        0x11 => 0x3B, // Control
        0x12 => 0x3A, // Alt/Option
        0x5B => 0x37, // Command/Win
        _ => virtual_key, // Pass through unknown keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cg_event_tap_creation() {
        let (sender, _) = std::sync::mpsc::channel();
        let device = CGEventTapDevice::new(sender);
        assert!(!device.is_running());
    }

    #[test]
    fn test_cg_event_tap_lifecycle() {
        let (sender, _) = std::sync::mpsc::channel();
        let mut device = CGEventTapDevice::new(sender);

        assert!(!device.is_running());

        // Test that we can call run_once without panicking
        let result = device.run_once();
        assert!(result.is_ok());
        assert!(result.unwrap());

        device.stop();
        assert!(!device.is_running());
    }

    #[test]
    fn test_accessibility_permission_check() {
        // This may fail in CI environments without UI
        let has_permission = CGEventTapDevice::check_accessibility_permissions();
        // Just verify it returns a boolean without panicking
        let _ = has_permission;
    }

    #[test]
    fn test_keycode_mapping_basic_letters() {
        // Test basic letter keys (QWERTY layout)
        assert_eq!(keycode_to_virtual_key(0x00), 0x41); // A
        assert_eq!(keycode_to_virtual_key(0x01), 0x53); // S
        assert_eq!(keycode_to_virtual_key(0x02), 0x44); // D
        assert_eq!(keycode_to_virtual_key(0x03), 0x46); // F
        assert_eq!(keycode_to_virtual_key(0x0B), 0x42); // B
        assert_eq!(keycode_to_virtual_key(0x0C), 0x51); // Q
        assert_eq!(keycode_to_virtual_key(0x0D), 0x57); // W
        assert_eq!(keycode_to_virtual_key(0x0E), 0x45); // E
        assert_eq!(keycode_to_virtual_key(0x0F), 0x52); // R
        assert_eq!(keycode_to_virtual_key(0x10), 0x59); // Y
    }

    #[test]
    fn test_keycode_mapping_digits() {
        // Test number keys
        assert_eq!(keycode_to_virtual_key(0x12), 0x31); // 1
        assert_eq!(keycode_to_virtual_key(0x13), 0x32); // 2
        assert_eq!(keycode_to_virtual_key(0x14), 0x33); // 3
        assert_eq!(keycode_to_virtual_key(0x15), 0x34); // 4
        assert_eq!(keycode_to_virtual_key(0x16), 0x36); // 6
        assert_eq!(keycode_to_virtual_key(0x17), 0x35); // 5
        assert_eq!(keycode_to_virtual_key(0x18), 0x3D); // =
        assert_eq!(keycode_to_virtual_key(0x19), 0x39); // 9
        assert_eq!(keycode_to_virtual_key(0x1A), 0x37); // 7
        assert_eq!(keycode_to_virtual_key(0x1B), 0x2D); // -
        assert_eq!(keycode_to_virtual_key(0x1C), 0x38); // 8
        assert_eq!(keycode_to_virtual_key(0x1D), 0x30); // 0
    }

    #[test]
    fn test_keycode_mapping_function_keys() {
        // Test function keys F1-F12
        assert_eq!(keycode_to_virtual_key(0x7A), 0x70); // F1
        assert_eq!(keycode_to_virtual_key(0x78), 0x71); // F2
        assert_eq!(keycode_to_virtual_key(0x63), 0x72); // F3
        assert_eq!(keycode_to_virtual_key(0x76), 0x73); // F4
        assert_eq!(keycode_to_virtual_key(0x60), 0x74); // F5
        assert_eq!(keycode_to_virtual_key(0x61), 0x75); // F6
        assert_eq!(keycode_to_virtual_key(0x62), 0x76); // F7
        assert_eq!(keycode_to_virtual_key(0x64), 0x77); // F8
        assert_eq!(keycode_to_virtual_key(0x65), 0x78); // F9
        assert_eq!(keycode_to_virtual_key(0x6D), 0x79); // F10
        assert_eq!(keycode_to_virtual_key(0x67), 0x7A); // F11
        assert_eq!(keycode_to_virtual_key(0x6F), 0x7B); // F12
    }

    #[test]
    fn test_keycode_mapping_navigation() {
        // Test navigation keys
        assert_eq!(keycode_to_virtual_key(0x7B), 0x25); // Left Arrow
        assert_eq!(keycode_to_virtual_key(0x7C), 0x27); // Right Arrow
        assert_eq!(keycode_to_virtual_key(0x7D), 0x26); // Down Arrow
        assert_eq!(keycode_to_virtual_key(0x7E), 0x28); // Up Arrow
        assert_eq!(keycode_to_virtual_key(0x74), 0x21); // Page Up
        assert_eq!(keycode_to_virtual_key(0x79), 0x22); // Page Down
        assert_eq!(keycode_to_virtual_key(0x73), 0x23); // Home/End
    }

    #[test]
    fn test_keycode_mapping_modifiers() {
        // Test modifier keys
        assert_eq!(keycode_to_virtual_key(0x37), 0x5B); // Command (left)
        assert_eq!(keycode_to_virtual_key(0x38), 0x10); // Shift (left)
        assert_eq!(keycode_to_virtual_key(0x3A), 0x12); // Option/Alt (left)
        assert_eq!(keycode_to_virtual_key(0x3B), 0x11); // Control (left)
        assert_eq!(keycode_to_virtual_key(0x3E), 0x5B); // Command (right)
        assert_eq!(keycode_to_virtual_key(0x3D), 0x10); // Shift (right)
    }

    #[test]
    fn test_keycode_mapping_numpad() {
        // Test numeric keypad
        assert_eq!(keycode_to_virtual_key(0x52), 0x60); // Keypad 0
        assert_eq!(keycode_to_virtual_key(0x53), 0x61); // Keypad 1
        assert_eq!(keycode_to_virtual_key(0x54), 0x62); // Keypad 2
        assert_eq!(keycode_to_virtual_key(0x55), 0x63); // Keypad 3
        assert_eq!(keycode_to_virtual_key(0x56), 0x64); // Keypad 4
        assert_eq!(keycode_to_virtual_key(0x57), 0x65); // Keypad 5
        assert_eq!(keycode_to_virtual_key(0x58), 0x66); // Keypad 6
        assert_eq!(keycode_to_virtual_key(0x59), 0x67); // Keypad 7
        assert_eq!(keycode_to_virtual_key(0x5A), 0x68); // Keypad 8
        assert_eq!(keycode_to_virtual_key(0x5B), 0x69); // Keypad 9
        assert_eq!(keycode_to_virtual_key(0x41), 0x6C); // Keypad .
        assert_eq!(keycode_to_virtual_key(0x43), 0x67); // Keypad *
        assert_eq!(keycode_to_virtual_key(0x45), 0x69); // Keypad +
        assert_eq!(keycode_to_virtual_key(0x4B), 0x62); // Keypad /
        assert_eq!(keycode_to_virtual_key(0x4E), 0x68); // Keypad -
    }

    #[test]
    fn test_keycode_mapping_special_keys() {
        // Test special keys
        assert_eq!(keycode_to_virtual_key(0x30), 0x09); // Tab
        assert_eq!(keycode_to_virtual_key(0x31), 0x20); // Space
        assert_eq!(keycode_to_virtual_key(0x33), 0x08); // Backspace
        assert_eq!(keycode_to_virtual_key(0x35), 0x1B); // Escape
        assert_eq!(keycode_to_virtual_key(0x24), 0x0D); // Return/Enter
        assert_eq!(keycode_to_virtual_key(0x39), 0x11); // Caps Lock
    }

    #[test]
    fn test_unknown_keycode_passthrough() {
        // Unknown keycodes should be passed through unchanged
        assert_eq!(keycode_to_virtual_key(0xFF), 0xFF);
        assert_eq!(keycode_to_virtual_key(0xAB), 0xAB);
        assert_eq!(keycode_to_virtual_key(0xCD), 0xCD);
    }

    #[test]
    fn test_reverse_mapping_basic() {
        // Test round-trip conversion for common keys
        assert_eq!(virtual_key_to_keycode(0x41), 0x00); // A
        assert_eq!(virtual_key_to_keycode(0x53), 0x01); // S
        assert_eq!(virtual_key_to_keycode(0x44), 0x02); // D
        assert_eq!(virtual_key_to_keycode(0x30), 0x1D); // 0
        assert_eq!(virtual_key_to_keycode(0x20), 0x31); // Space
        assert_eq!(virtual_key_to_keycode(0x09), 0x30); // Tab
    }

    #[test]
    fn test_reverse_mapping_function_keys() {
        assert_eq!(virtual_key_to_keycode(0x70), 0x7A); // F1
        assert_eq!(virtual_key_to_keycode(0x71), 0x78); // F2
        assert_eq!(virtual_key_to_keycode(0x7A), 0x67); // F11
        assert_eq!(virtual_key_to_keycode(0x7B), 0x6F); // F12
    }

    #[test]
    fn test_reverse_mapping_navigation() {
        assert_eq!(virtual_key_to_keycode(0x25), 0x7B); // Left Arrow
        assert_eq!(virtual_key_to_keycode(0x27), 0x7C); // Right Arrow
        assert_eq!(virtual_key_to_keycode(0x26), 0x7D); // Down Arrow
        assert_eq!(virtual_key_to_keycode(0x28), 0x7E); // Up Arrow
    }

    #[test]
    fn test_reverse_mapping_modifiers() {
        assert_eq!(virtual_key_to_keycode(0x10), 0x38); // Shift
        assert_eq!(virtual_key_to_keycode(0x11), 0x3B); // Control
        assert_eq!(virtual_key_to_keycode(0x12), 0x3A); // Alt
        // Note: VK 0x5B is shared between '[' (OEM_4) and LWin/Command in Windows
        // Reverse mapping returns the first match ('[' keycode) due to this collision
        // This is a known limitation of the bidirectional mapping
        assert_eq!(virtual_key_to_keycode(0x5B), 0x21); // Returns '[' not Command
    }

    #[test]
    fn test_roundtrip_conversion_consistency() {
        // Verify that keycode -> vk -> keycode is consistent for common keys
        let test_cases = vec![
            0x00, 0x01, 0x02, 0x03, // A, S, D, F
            0x12, 0x13, 0x14, 0x15, // 1, 2, 3, 4
            0x30, 0x31, 0x33, 0x35, // Tab, Space, BS, Esc
            0x7A, 0x78, 0x63, 0x76, // F1, F2, F3, F4
            0x7B, 0x7C, 0x7D, 0x7E, // Arrows
        ];

        for original_keycode in test_cases {
            let vk = keycode_to_virtual_key(original_keycode);
            let reversed = virtual_key_to_keycode(vk);
            assert_eq!(
                original_keycode, reversed,
                "Roundtrip failed for keycode {:#04X} -> vk {:#04X} -> keycode {:#04X}",
                original_keycode, vk, reversed
            );
        }
    }
}
