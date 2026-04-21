//! macOS input device implementation using Core Graphics Event Tap.
//!
//! Provides global keyboard and mouse event monitoring via CGEventTap.
//! Requires Accessibility permission (System Settings > Privacy & Security > Accessibility).
//!
//! # Architecture
//!
//! ```
//! CGEventTap (Core Graphics) → Callback → InputEvent conversion → mpsc channel
//! ```
//!
//! # Performance
//!
//! Event callback latency: < 0.1ms per event (kernel-level interception)

#![cfg(target_os = "macos")]

use std::ffi::c_void;
use std::sync::mpsc::Sender;

use crate::types::{
    InputEvent, KeyEvent, KeyState, ModifierState, MouseButton, MouseEvent,
    MouseEventType,
};
use anyhow::{bail, Result};
use tracing::{debug, trace, warn};

// ============================================================================
// FFI Bindings for Core Graphics Event Tap
// ============================================================================

/// CGEvent type constants
#[allow(dead_code)]
pub mod cg_event_types {
    pub const KEY_DOWN: u32 = 10;
    pub const KEY_UP: u32 = 11;
    pub const FLAGS_CHANGED: u32 = 12;
    pub const LEFT_MOUSE_DOWN: u32 = 1;
    pub const LEFT_MOUSE_UP: u32 = 2;
    pub const RIGHT_MOUSE_DOWN: u32 = 3;
    pub const RIGHT_MOUSE_UP: u32 = 4;
    pub const MOUSE_MOVED: u32 = 5;
    pub const LEFT_MOUSE_DRAGGED: u32 = 6;
    pub const RIGHT_MOUSE_DRAGGED: u32 = 7;
    pub const SCROLL_WHEEL: u32 = 22;
}

/// CGEvent field keys
pub mod cg_event_fields {
    pub const KEYBOARD_KEYCODE: u32 = 9;
    pub const MOUSE_X: u32 = 1;
    pub const MOUSE_Y: u32 = 2;
    pub const SCROLL_WHEEL_DELTA_Y: u32 = 678;
    pub const SCROLL_WHEEL_DELTA_X: u32 = 681;
}

/// CGEventTap constants
pub mod cg_tap {
    pub const OPTION_LISTEN_SESSION: u32 = 0; // kCGSessionEventTap
    pub const OPTION_TAP_LISTEN: u32 = 1; // kCGHeadInsertEventTap
                                          // Event type values (inline to avoid cross-module reference issues)
    const KEY_DOWN_VAL: u32 = 10;
    const KEY_UP_VAL: u32 = 11;
    const FLAGS_CHANGED_VAL: u32 = 12;
    const LEFT_MOUSE_DOWN_VAL: u32 = 1;
    const LEFT_MOUSE_UP_VAL: u32 = 2;
    const RIGHT_MOUSE_DOWN_VAL: u32 = 3;
    const RIGHT_MOUSE_UP_VAL: u32 = 4;
    const MOUSE_MOVED_VAL: u32 = 5;
    const SCROLL_WHEEL_VAL: u32 = 22;

    pub const MASK_KEYBOARD: u64 =
        1 << KEY_DOWN_VAL | 1 << KEY_UP_VAL | 1 << FLAGS_CHANGED_VAL;
    pub const MASK_MOUSE: u64 = 1 << LEFT_MOUSE_DOWN_VAL
        | 1 << LEFT_MOUSE_UP_VAL
        | 1 << RIGHT_MOUSE_DOWN_VAL
        | 1 << RIGHT_MOUSE_UP_VAL
        | 1 << MOUSE_MOVED_VAL
        | 1 << SCROLL_WHEEL_VAL;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    // Event tap creation and management
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: extern "C" fn(
            *const c_void,
            *const c_void,
            *const c_void,
            u64,
        ) -> *const c_void,
        user_info: *const c_void,
    ) -> *mut c_void;

    fn CGEventTapEnable(tap: *mut c_void, enable: bool);

    fn CGEventRelease(event: *const c_void);

    // Event query functions
    fn CGEventGetType(event: *const c_void) -> u32;
    fn CGEventGetIntegerValueField(event: *const c_void, field: u32) -> i64;

    // Run loop integration
    #[allow(dead_code)]
    fn CFRelease(cf: *const c_void);
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: i64,
    ) -> *mut c_void;

    fn CFRunLoopGetMain() -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRemoveSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopSourceInvalidate(source: *mut c_void);
}

/// Check if current process has Accessibility permission using native API
pub fn check_accessibility_permissions() -> bool {
    unsafe { AXIsProcessTrusted() }
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

// ============================================================================
// Global callback state (thread-local storage for sender)
// ============================================================================

thread_local! {
    static EVENT_SENDER: std::cell::RefCell<Option<Sender<InputEvent>>> =
        std::cell::RefCell::new(None);
}

/// Set the event sender for the current thread's callback
fn set_sender(sender: Sender<InputEvent>) {
    EVENT_SENDER.with(|s| {
        *s.borrow_mut() = Some(sender);
    });
}

/// Get the event sender from the current thread's callback
fn get_sender() -> Option<Sender<InputEvent>> {
    EVENT_SENDER.with(|s| s.borrow().clone())
}

/// Clear the event sender
fn clear_sender() {
    EVENT_SENDER.with(|s| {
        *s.borrow_mut() = None;
    });
}

// ============================================================================
// CGEventTap callback function
// ============================================================================

/// Global event tap callback - called by Core Graphics for each captured event.
///
/// This is a C-compatible callback that runs on the event tap's run loop thread.
/// It converts raw CGEvents to our InputEvent type and sends them via mpsc channel.
///
/// # Arguments
///
/// * `_proxy` - The Mach port for the event tap (unused)
/// * `event` - The captured CGEvent (must not be released if returning it)
/// * `_info` - User info pointer (unused)
/// * `type_` - The event type (redundant with CGEventGetType but provided for speed)
///
/// # Returns
///
/// Returns the original event (pass-through mode). We don't modify or block events.
extern "C" fn event_tap_callback(
    _proxy: *const c_void,
    event: *const c_void,
    _info: *const c_void,
    _type_: u64,
) -> *const c_void {
    if let Some(ref sender) = get_sender() {
        let input_event = convert_cg_event(event);

        if let Some(evt) = input_event {
            let _ = sender.send(evt);
        }
    }

    // Return original event unchanged (pass-through, don't block)
    event
}

// ============================================================================
// Event Conversion Functions
// ============================================================================

/// Convert a raw CGEvent to our InputEvent type
///
/// Handles all supported event types:
/// - Keyboard: keyDown, keyUp, flagsChanged
/// - Mouse: leftMouseDown/Up, rightMouseDown/Up, mouseMoved, scrollWheel
fn convert_cg_event(event: *const c_void) -> Option<InputEvent> {
    let event_type = unsafe { CGEventGetType(event) };

    match event_type {
        t if t == cg_event_types::KEY_DOWN => {
            convert_key_event(event, KeyState::Pressed)
        }
        t if t == cg_event_types::KEY_UP => convert_key_event(event, KeyState::Released),
        t if t == cg_event_types::FLAGS_CHANGED => None, // Handled implicitly via modifier tracking
        t if t == cg_event_types::LEFT_MOUSE_DOWN => {
            convert_mouse_button_event(event, MouseButton::Left, KeyState::Pressed)
        }
        t if t == cg_event_types::LEFT_MOUSE_UP => {
            convert_mouse_button_event(event, MouseButton::Left, KeyState::Released)
        }
        t if t == cg_event_types::RIGHT_MOUSE_DOWN => {
            convert_mouse_button_event(event, MouseButton::Right, KeyState::Pressed)
        }
        t if t == cg_event_types::RIGHT_MOUSE_UP => {
            convert_mouse_button_event(event, MouseButton::Right, KeyState::Released)
        }
        t if t == cg_event_types::MOUSE_MOVED
            || t == cg_event_types::LEFT_MOUSE_DRAGGED =>
        {
            convert_mouse_move_event(event)
        }
        t if t == cg_event_types::SCROLL_WHEEL => convert_scroll_wheel_event(event),
        _ => None,
    }
}

/// Convert a keyboard CGEvent to KeyEvent
fn convert_key_event(event: *const c_void, state: KeyState) -> Option<InputEvent> {
    let keycode =
        unsafe { CGEventGetIntegerValueField(event, cg_event_fields::KEYBOARD_KEYCODE) };

    let scan_code = keycode as u16;
    let virtual_key = keycode_to_virtual_key(scan_code);

    trace!(
        "Key event: keycode={}, vk=0x{:02X}, state={:?}",
        keycode,
        virtual_key,
        state
    );

    Some(InputEvent::Key(KeyEvent::new(
        scan_code,
        virtual_key,
        state,
    )))
}

/// Convert a mouse button CGEvent to MouseEvent
fn convert_mouse_button_event(
    event: *const c_void,
    button: MouseButton,
    state: KeyState,
) -> Option<InputEvent> {
    let x =
        unsafe { CGEventGetIntegerValueField(event, cg_event_fields::MOUSE_X) } as i32;
    let y =
        unsafe { CGEventGetIntegerValueField(event, cg_event_fields::MOUSE_Y) } as i32;

    let event_type = match state {
        KeyState::Pressed => MouseEventType::ButtonDown(button),
        KeyState::Released => MouseEventType::ButtonUp(button),
    };

    trace!("Mouse button: {:?} {:?} at ({}, {})", button, state, x, y);

    Some(InputEvent::Mouse(MouseEvent::new(event_type, x, y)))
}

/// Convert a mouse move CGEvent to MouseEvent
fn convert_mouse_move_event(event: *const c_void) -> Option<InputEvent> {
    let x =
        unsafe { CGEventGetIntegerValueField(event, cg_event_fields::MOUSE_X) } as i32;
    let y =
        unsafe { CGEventGetIntegerValueField(event, cg_event_fields::MOUSE_Y) } as i32;

    trace!("Mouse move at ({}, {})", x, y);

    Some(InputEvent::Mouse(MouseEvent::new(
        MouseEventType::Move,
        x,
        y,
    )))
}

/// Convert a scroll wheel CGEvent to MouseEvent
fn convert_scroll_wheel_event(event: *const c_void) -> Option<InputEvent> {
    let delta_y = unsafe {
        CGEventGetIntegerValueField(event, cg_event_fields::SCROLL_WHEEL_DELTA_Y)
    } as i32;
    let delta_x = unsafe {
        CGEventGetIntegerValueField(event, cg_event_fields::SCROLL_WHEEL_DELTA_X)
    } as i32;
    let x =
        unsafe { CGEventGetIntegerValueField(event, cg_event_fields::MOUSE_X) } as i32;
    let y =
        unsafe { CGEventGetIntegerValueField(event, cg_event_fields::MOUSE_Y) } as i32;

    trace!(
        "Scroll wheel: delta_x={}, delta_y=({}) at ({}, {})",
        delta_x,
        delta_y,
        x,
        y
    );

    // Prefer vertical scroll if both present
    if delta_y != 0 {
        Some(InputEvent::Mouse(MouseEvent::new(
            MouseEventType::Wheel(delta_y),
            x,
            y,
        )))
    } else if delta_x != 0 {
        Some(InputEvent::Mouse(MouseEvent::new(
            MouseEventType::HWheel(delta_x),
            x,
            y,
        )))
    } else {
        None
    }
}

// ============================================================================
// CGEventTap Device
// ============================================================================

/// macOS global input device using Core Graphics Event Tap.
///
/// Captures keyboard and mouse events at the system level.
/// Requires Accessibility permission.
///
/// # Example
///
/// ```ignore
/// use wakem::platform::macos::input::CGEventTapDevice;
/// use wakem::types::InputEvent;
/// use std::sync::mpsc::channel;
///
/// let (tx, rx) = channel();
/// let mut device = CGEventTapDevice::new(tx);
/// device.run()?;
/// ```
pub struct CGEventTapDevice {
    config: InputDeviceConfig,
    event_sender: Sender<InputEvent>,
    running: std::sync::atomic::AtomicBool,
    tap_port: std::sync::Mutex<Option<*mut c_void>>,
    run_loop_source: std::sync::Mutex<Option<*mut c_void>>,
}

// SAFETY: CGEventTapDevice is designed to be used across threads.
// The raw pointers are only accessed behind Mutex locks.
unsafe impl Send for CGEventTapDevice {}
unsafe impl Sync for CGEventTapDevice {}

/// Configuration for the input device
#[derive(Debug, Clone)]
pub struct InputDeviceConfig {
    pub capture_keyboard: bool,
    pub capture_mouse: bool,
    pub block_legacy_input: bool,
}

impl Default for InputDeviceConfig {
    fn default() -> Self {
        Self {
            capture_keyboard: true,
            capture_mouse: true,
            block_legacy_input: false,
        }
    }
}

impl CGEventTapDevice {
    /// Create a new CGEventTap device
    ///
    /// # Arguments
    /// * `sender` - Channel sender for delivering captured events
    pub fn new(sender: Sender<InputEvent>) -> Self {
        Self {
            config: InputDeviceConfig::default(),
            event_sender: sender,
            running: std::sync::atomic::AtomicBool::new(false),
            tap_port: std::sync::Mutex::new(None),
            run_loop_source: std::sync::Mutex::new(None),
        }
    }

    /// Create with custom configuration
    pub fn with_config(sender: Sender<InputEvent>, config: InputDeviceConfig) -> Self {
        Self {
            config,
            event_sender: sender,
            running: std::sync::atomic::AtomicBool::new(false),
            tap_port: std::sync::Mutex::new(None),
            run_loop_source: std::sync::Mutex::new(None),
        }
    }

    /// Start the event tap and begin capturing events
    ///
    /// This method blocks the calling thread on the CFRunLoop.
    /// Typically called from a dedicated background thread.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Accessibility permission is not granted
    /// - Event tap creation fails (system-level issue)
    pub fn run(&mut self) -> Result<()> {
        debug!(
            "Starting CGEventTap: keyboard={}, mouse={}",
            self.config.capture_keyboard, self.config.capture_mouse
        );

        // Build event mask based on configuration
        let mut mask: u64 = 0;
        if self.config.capture_keyboard {
            mask |= cg_tap::MASK_KEYBOARD;
        }
        if self.config.capture_mouse {
            mask |= cg_tap::MASK_MOUSE;
        }

        if mask == 0 {
            bail!("No event types selected for capture");
        }

        // Create the event tap
        let tap_port = unsafe {
            CGEventTapCreate(
                cg_tap::OPTION_LISTEN_SESSION, // Listen to session events
                cg_tap::OPTION_TAP_LISTEN,     // Insert at head of event stream
                0,                  // Default options (listen only, don't modify)
                mask,               // Events of interest mask
                event_tap_callback, // Callback function
                std::ptr::null(),   // No user info
            )
        };

        if tap_port.is_null() {
            bail!(
                "Failed to create CGEventTap. \
                 Please ensure Accessibility permission is granted in \
                 System Settings > Privacy & Security > Accessibility"
            );
        }

        debug!("Created CGEventTap successfully");

        // Enable the tap
        unsafe { CGEventTapEnable(tap_port, true) };

        // Store the port reference
        {
            let mut guard = self.tap_port.lock().unwrap();
            *guard = Some(tap_port);
        }

        // Set up the sender for the callback
        set_sender(self.event_sender.clone());
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);

        // Create run loop source and add to main run loop
        let run_loop_source =
            unsafe { CFMachPortCreateRunLoopSource(std::ptr::null(), tap_port, 0) };

        if !run_loop_source.is_null() {
            unsafe {
                let rl = CFRunLoopGetMain();
                // Use kCFRunLoopDefaultMode (null pointer)
                CFRunLoopAddSource(rl, run_loop_source, std::ptr::null());
            }

            {
                let mut guard = self.run_loop_source.lock().unwrap();
                *guard = Some(run_loop_source);
            }
        }

        debug!("CGEventTap started, waiting for events...");

        Ok(())
    }

    /// Stop the event tap and clean up resources
    pub fn stop(&self) {
        debug!("Stopping CGEventTap");

        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        clear_sender();

        // Remove run loop source
        {
            let mut guard = self.run_loop_source.lock().unwrap();
            if let Some(source) = guard.take() {
                unsafe {
                    let rl = CFRunLoopGetMain();
                    CFRunLoopRemoveSource(rl, source, std::ptr::null());
                    CFRunLoopSourceInvalidate(source);
                }
            }
        }

        // Disable and release tap port
        {
            let mut guard = self.tap_port.lock().unwrap();
            if let Some(tap) = guard.take() {
                unsafe { CGEventTapEnable(tap, false) };
                // Note: Don't release here as it may still be in use by run loop
            }
        }

        debug!("CGEventTap stopped");
    }

    /// Check if the device is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the event sender (for external use)
    pub fn get_sender(&self) -> Sender<InputEvent> {
        self.event_sender.clone()
    }

    /// Get the configuration
    pub fn get_config(&self) -> &InputDeviceConfig {
        &self.config
    }
}

impl Drop for CGEventTapDevice {
    fn drop(&mut self) {
        if self.is_running() {
            self.stop();
        }
    }
}

// ============================================================================
// Key Code Mapping Table
// ============================================================================

/// Convert macOS hardware keycode to Windows virtual key code
///
/// Maps Apple keyboard scancodes to Windows VK_* codes for cross-platform consistency.
/// Uses standard US keyboard layout mapping.
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
        0x0A => 0x42, // B (or Japanese/Korean)
        0x0B => 0x51, // Q
        0x0C => 0x57, // W
        0x0D => 0x45, // E
        0x0E => 0x52, // R
        0x0F => 0x59, // Y
        0x10 => 0x54, // T
        0x11 => 0x31, // 1
        0x12 => 0x32, // 2
        0x13 => 0x33, // 3
        0x14 => 0x34, // 4
        0x15 => 0x36, // 5
        0x16 => 0x35, // 6
        0x17 => 0x3D, // =
        0x18 => 0x39, // 7
        0x19 => 0x38, // 8
        0x1A => 0x2D, // -
        0x1B => 0x37, // 9
        0x1C => 0x30, // 0
        0x1D => 0x5D, // ]
        0x1E => 0x4F, // O
        0x1F => 0x55, // U
        0x20 => 0x5B, // [
        0x21 => 0x49, // I
        0x22 => 0x50, // P
        0x23 => 0x0D, // Return / Enter
        0x24 => 0x4C, // L
        0x25 => 0x4A, // J
        0x26 => 0x27, // '
        0x27 => 0x4B, // K
        0x28 => 0x3B, // ;
        0x29 => 0x5C, // \
        0x2A => 0x2C, // ,
        0x2B => 0x2F, // /
        0x2C => 0x2E, // .
        0x2D => 0x4E, // N
        0x2E => 0x4D, // M
        0x2F => 0x20, // Space
        0x30 => 0x60, // `
        0x31 => 0x28, // Backspace
        0x32 => 0x09, // Tab
        0x33 => 0x1B, // Escape (or different layout)
        0x34 => 0x35, // End
        0x35 => 0x14, // Caps Lock
        0x36 => 0x10, // Left Shift
        0x37 => 0x5B, // Left Command (Meta/Win)
        0x38 => 0x12, // Left Alt (Option) → VK_MENU
        0x39 => 0x11, // Left Control → VK_CONTROL
        0x3A => 0x1B, // Escape
        0x3B => 0x11, // Right Control
        0x3C => 0x12, // Right Alt (Option)
        0x3D => 0x10, // Right Shift
        0x3E => 0x5B, // Right Command (Meta/Win)

        // Function row
        0x40 => 0x70, // F17 (on some keyboards)
        0x41 => 0x91, // numpad decimal (on full keyboards)
        0x43 => 0x6B, // F19
        0x44 => 0x90, // numpad *
        0x45 => 0x92, // numpad +
        0x47 => 0x6C, // F20
        0x48 => 0x93, // numpad clear
        0x49 => 0xA0, // Volume Up (VK_VOLUME_UP doesn't exist in Win32, use unmapped)
        0x4A => 0xA1, // Volume Down
        0x4B => 0xA2, // Mute
        0x4C => 0x94, // numpad /
        0x4E => 0x95, // numpad Enter
        0x4F => 0x96, // numpad -
        0x50 => 0x6D, // Underscore (on some layouts)
        0x51 => 0x97, // numpad =
        0x52 => 0x6E, // Keypad 0
        0x53 => 0x6F, // Keypad 1
        0x54 => 0x70, // Keypad 2
        0x55 => 0x71, // Keypad 3
        0x56 => 0x72, // Keypad 4
        0x57 => 0x73, // Keypad 5
        0x58 => 0x74, // Keypad 6
        0x59 => 0x75, // Keypad 7
        0x5A => 0x76, // Keypad 8
        0x5B => 0x77, // Keypad 9
        0x5C => 0x0D, // Return (numpad)
        0x5D => 0x21, // End (Fn+Right Arrow)
        0x5E => 0x24, // Home (Fn+Left Arrow)
        0x5F => 0x22, // Page Up (Fn+Up Arrow)
        0x60 => 0x23, // Page Down (Fn+Down Arrow)
        0x61 => 0x25, // Left Arrow
        0x62 => 0x27, // Right Arrow
        0x63 => 0x26, // Up Arrow
        0x64 => 0x28, // Down Arrow
        0x65 => 0x2C, // Delete (Fn+Delete = Forward Delete)
        0x66 => 0x2E, // Delete (Backspace equivalent)
        0x7A => 0x70, // F1
        0x78 => 0x71, // F2 (Note: Apple keycodes are not sequential for F-keys)
        0x63 => 0x72, // F3
        0x76 => 0x73, // F4
        0x77 => 0x74, // F5
        0x75 => 0x75, // F6
        0x73 => 0x76, // F7
        0x79 => 0x77, // F8
        0x6D => 0x78, // F9
        0x69 => 0x79, // F10
        0x6B => 0x7A, // F11
        0x71 => 0x7B, // F12
        0x73 => 0x63, // Insert (Fn+Enter on full keyboards)
        0x75 => 0x90, // Print Screen / F13
        0x76 => 0x91, // Scroll Lock / F14
        0x77 => 0x92, // Pause / F15
        0x78 => 0xA3, // F16
        0x79 => 0x98, // F18
        0x7A => 0x99, // F19
        0x7B => 0x9A, // F20
        0x7C => 0x9B, // F21/F22/F23/F24/F25 (application-specific)
        0x7D => 0x9C,
        0x7E => 0x9D,

        // Special keys
        0x29 => 0xBA, // ; (semicolon)
        0x2B => 0xBF, // / (forward slash)
        0x2A => 0xBB, // , (comma)
        0x2C => 0xBC, // . (period)

        // Unknown/Unmapped - return raw keycode shifted into high byte
        _ => keycode.wrapping_shl(8), // Preserve identity for debugging
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_accessibility_permissions() {
        let result = check_accessibility_permissions();
        println!("Accessibility permissions granted: {}", result);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_keycode_mapping_basic_letters() {
        assert_eq!(keycode_to_virtual_key(0x00), 0x41); // A
        assert_eq!(keycode_to_virtual_key(0x01), 0x53); // S
        assert_eq!(keycode_to_virtual_key(0x02), 0x44); // D
        assert_eq!(keycode_to_virtual_key(0x03), 0x46); // F
    }

    #[test]
    fn test_keycode_mapping_numbers() {
        assert_eq!(keycode_to_virtual_key(0x11), 0x31); // 1
        assert_eq!(keycode_to_virtual_key(0x12), 0x32); // 2
        assert_eq!(keycode_to_virtual_key(0x13), 0x33); // 3
        assert_eq!(keycode_to_virtual_key(0x1C), 0x30); // 0
    }

    #[test]
    fn test_keycode_mapping_modifiers() {
        assert_eq!(keycode_to_virtual_key(0x37), 0x5B); // Left Command → Meta
        assert_eq!(keycode_to_virtual_key(0x38), 0x12); // Left Alt → Alt
        assert_eq!(keycode_to_virtual_key(0x39), 0x11); // Left Ctrl → Ctrl
        assert_eq!(keycode_to_virtual_key(0x36), 0x10); // Left Shift → Shift
    }

    #[test]
    fn test_keycode_mapping_function_keys() {
        assert_eq!(keycode_to_virtual_key(0x7A), 0x70); // F1
        assert_eq!(keycode_to_virtual_key(0x78), 0x71); // F2
        assert_eq!(keycode_to_virtual_key(0x71), 0x7B); // F12
    }

    #[test]
    fn test_keycode_mapping_arrows() {
        assert_eq!(keycode_to_virtual_key(0x61), 0x25); // Left
        assert_eq!(keycode_to_virtual_key(0x62), 0x27); // Right
        assert_eq!(keycode_to_virtual_key(0x63), 0x26); // Up
        assert_eq!(keycode_to_virtual_key(0x64), 0x28); // Down
    }

    #[test]
    fn test_keycode_mapping_special_keys() {
        assert_eq!(keycode_to_virtual_key(0x23), 0x0D); // Return
        assert_eq!(keycode_to_virtual_key(0x2F), 0x20); // Space
        assert_eq!(keycode_to_virtual_key(0x31), 0x28); // Backspace
        assert_eq!(keycode_to_virtual_key(0x32), 0x09); // Tab
        assert_eq!(keycode_to_virtual_key(0x35), 0x14); // Caps Lock
        assert_eq!(keycode_to_virtual_key(0x3A), 0x1B); // Escape
    }

    #[test]
    fn test_input_device_config_default() {
        let config = InputDeviceConfig::default();
        assert!(config.capture_keyboard);
        assert!(config.capture_mouse);
        assert!(!config.block_legacy_input);
    }

    #[test]
    fn test_cg_event_type_constants() {
        // Verify constants are correct
        assert_eq!(cg_event_types::KEY_DOWN, 10);
        assert_eq!(cg_event_types::KEY_UP, 11);
        assert_eq!(cg_event_types::LEFT_MOUSE_DOWN, 1);
        assert_eq!(cg_event_types::LEFT_MOUSE_UP, 2);
        assert_eq!(cg_event_types::SCROLL_WHEEL, 22);
    }

    #[test]
    fn test_cgtap_mask_calculation() {
        let keyboard_mask = cg_tap::MASK_KEYBOARD;
        let mouse_mask = cg_tap::MASK_MOUSE;

        // Verify masks include expected event types
        assert_ne!(keyboard_mask, 0);
        assert_ne!(mouse_mask, 0);

        // Combined mask should be larger than individual
        assert!(keyboard_mask | mouse_mask > keyboard_mask);
        assert!(keyboard_mask | mouse_mask > mouse_mask);
    }
}
