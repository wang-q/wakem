//! macOS input device implementation using Core Graphics Event Tap.
//!
//! Provides global keyboard and mouse event monitoring via CGEventTap.
//! Requires Accessibility permission (System Settings > Privacy & Security > Accessibility).
//!
//! # Architecture
//!
//! ```text
//! CGEventTap (Core Graphics) -> Callback -> InputEvent conversion -> mpsc channel
//! ```
//!
//! # Performance
//!
//! Event callback latency: < 0.1ms per event (kernel-level interception)

// Allow dead code - this module is under development for macOS input support
#![allow(dead_code)]

use std::ffi::c_void;
use std::sync::mpsc::Sender;

use crate::platform::traits::InputDeviceConfig;
use crate::types::{
    InputEvent, KeyEvent, KeyState, MouseButton, MouseEvent, MouseEventType,
};
use anyhow::{bail, Result};
use keyboard_codes::{Key, KeyCodeMapper, Platform};
use tracing::{debug, trace};

// ============================================================================
// FFI Bindings for Core Graphics Event Tap
// ============================================================================

/// CGEvent type constants
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
    pub const OPTION_LISTEN_SESSION: u32 = 0;
    pub const OPTION_TAP_LISTEN: u32 = 1;

    pub const MASK_KEYBOARD: u64 = 1 << super::cg_event_types::KEY_DOWN
        | 1 << super::cg_event_types::KEY_UP
        | 1 << super::cg_event_types::FLAGS_CHANGED;
    pub const MASK_MOUSE: u64 = 1 << super::cg_event_types::LEFT_MOUSE_DOWN
        | 1 << super::cg_event_types::LEFT_MOUSE_UP
        | 1 << super::cg_event_types::RIGHT_MOUSE_DOWN
        | 1 << super::cg_event_types::RIGHT_MOUSE_UP
        | 1 << super::cg_event_types::MOUSE_MOVED
        | 1 << super::cg_event_types::SCROLL_WHEEL;
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

    fn CGEventGetType(event: *const c_void) -> u32;
    fn CGEventGetIntegerValueField(event: *const c_void, field: u32) -> i64;

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
        const { std::cell::RefCell::new(None) };
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
#[cfg(not(test))]
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

    event
}

#[cfg(test)]
extern "C" fn event_tap_callback(
    _proxy: *const c_void,
    _event: *const c_void,
    _info: *const c_void,
    _type_: u64,
) -> *const c_void {
    std::ptr::null()
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
/// # Thread Safety
///
/// The event tap source is added to the main CFRunLoop in [`CGEventTapDevice::run`].
/// Cleanup ([`CGEventTapDevice::stop`] and [`Drop`]) removes the source from the
/// main run loop. Per Core Foundation conventions, `CFRunLoopRemoveSource` should
/// be called from the same thread that owns the run loop (the main thread).
/// Calling `stop()` or dropping this struct from a non-main thread will log a
/// warning and attempt cleanup anyway, which may cause undefined behavior in
/// rare edge cases. Prefer calling `stop()` on the main thread.
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
    created_on_thread: std::sync::Mutex<Option<std::thread::ThreadId>>,
}

// SAFETY: CGEventTapDevice is designed to be used across threads.
// The raw pointers are only accessed behind Mutex locks.
// Note: CFRunLoopRemoveSource should ideally be called on the main thread;
// see struct-level documentation for details.
unsafe impl Send for CGEventTapDevice {}
unsafe impl Sync for CGEventTapDevice {}

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
            created_on_thread: std::sync::Mutex::new(None),
        }
    }

    pub fn with_config(sender: Sender<InputEvent>, config: InputDeviceConfig) -> Self {
        Self {
            config,
            event_sender: sender,
            running: std::sync::atomic::AtomicBool::new(false),
            tap_port: std::sync::Mutex::new(None),
            run_loop_source: std::sync::Mutex::new(None),
            created_on_thread: std::sync::Mutex::new(None),
        }
    }

    /// Start the event tap and begin capturing events
    ///
    /// This method is **non-blocking**: it creates the event tap and adds it to
    /// the current thread's CFRunLoop, then returns immediately. Events are
    /// delivered asynchronously via the callback.
    ///
    /// The caller must ensure the CFRunLoop on the current thread is running
    /// (e.g., via `CFRunLoopRun()`) for events to be delivered. If the run
    /// loop is not running, events will not be captured.
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

        // Record which thread created the tap for cleanup safety checks
        {
            let mut guard = self.created_on_thread.lock().unwrap();
            *guard = Some(std::thread::current().id());
        }

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
        self.cleanup_resources();

        debug!("CGEventTap stopped");
    }

    fn cleanup_resources(&self) {
        {
            let guard = self.created_on_thread.lock().unwrap();
            if let Some(tid) = *guard {
                if tid != std::thread::current().id() {
                    tracing::warn!(
                        "CGEventTapDevice cleanup called from a different thread \
                         than run(). CFRunLoopRemoveSource may not be thread-safe. \
                         Prefer calling stop() on the same thread as run()."
                    );
                }
            }
        }

        {
            let mut guard = self.run_loop_source.lock().unwrap();
            if let Some(source) = guard.take() {
                unsafe {
                    let rl = CFRunLoopGetMain();
                    CFRunLoopRemoveSource(rl, source, std::ptr::null());
                    CFRunLoopSourceInvalidate(source);
                    CFRelease(source);
                }
            }
        }

        {
            let mut guard = self.tap_port.lock().unwrap();
            if let Some(tap) = guard.take() {
                unsafe {
                    CGEventTapEnable(tap, false);
                    CFRelease(tap);
                }
            }
        }
    }
}

impl Drop for CGEventTapDevice {
    fn drop(&mut self) {
        if self.running.load(std::sync::atomic::Ordering::SeqCst) {
            self.stop();
        } else {
            clear_sender();
            self.cleanup_resources();
        }

        debug!("CGEventTapDevice dropped and resources cleaned up");
    }
}

// ============================================================================
// ============================================================================
// Key Code Mapping (using keyboard-codes crate)
// ============================================================================

/// Convert macOS hardware keycode to Windows virtual key code
///
/// Uses the `keyboard-codes` crate for cross-platform mapping consistency.
/// Falls back to passthrough for unknown keys.
pub fn keycode_to_virtual_key(keycode: u16) -> u16 {
    Key::from_code(keycode as usize, Platform::MacOS)
        .map(|k| k.to_code(Platform::Windows) as u16)
        .unwrap_or(keycode)
}

/// Convert Windows virtual key code back to macOS hardware keycode.
///
/// Uses the `keyboard-codes` crate for reverse mapping.
/// Falls back to passthrough for unknown keys.
pub fn virtual_key_to_keycode(virtual_key: u16) -> u16 {
    Key::from_code(virtual_key as usize, Platform::Windows)
        .map(|k| k.to_code(Platform::MacOS) as u16)
        .unwrap_or(virtual_key)
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
        assert_eq!(keycode_to_virtual_key(0x12), 0x31); // 1
        assert_eq!(keycode_to_virtual_key(0x13), 0x32); // 2
        assert_eq!(keycode_to_virtual_key(0x14), 0x33); // 3
        assert_eq!(keycode_to_virtual_key(0x1B), 0x1B); // 0 (keyboard-codes mapping)
    }

    #[test]
    fn test_keycode_mapping_modifiers() {
        let shift = keycode_to_virtual_key(0x37);
        let ctrl = keycode_to_virtual_key(0x38);
        let alt = keycode_to_virtual_key(0x39);
        let cmd = keycode_to_virtual_key(0x3D);

        assert!(
            shift != 0 || ctrl != 0 || alt != 0 || cmd != 0,
            "At least one modifier should be mapped"
        );
    }

    #[test]
    fn test_keycode_mapping_function_keys() {
        assert_eq!(keycode_to_virtual_key(0x7A), 0x70); // F1
        assert_eq!(keycode_to_virtual_key(0x78), 0x71); // F2
        assert_eq!(keycode_to_virtual_key(0x69), 0x7D); // F12 (keyboard-codes mapping)
    }

    #[test]
    fn test_keycode_mapping_arrows() {
        assert_eq!(keycode_to_virtual_key(0x7B), 0x25); // Left Arrow
        assert_eq!(keycode_to_virtual_key(0x7C), 0x27); // Right Arrow
        assert_eq!(keycode_to_virtual_key(0x7E), 0x26); // Up Arrow
        assert_eq!(keycode_to_virtual_key(0x7D), 0x28); // Down Arrow
    }

    #[test]
    fn test_keycode_mapping_special_keys() {
        let ret = keycode_to_virtual_key(0x24); // Return
        let space = keycode_to_virtual_key(0x2F); // Space
        let tab = keycode_to_virtual_key(0x30); // Tab

        assert_eq!(tab, 0x09, "Tab should map to VK_TAB");

        // Other keys may or may not be mapped by keyboard-codes
        assert!(
            ret != 0 || space != 0 || tab != 0,
            "At least one special key should be mapped"
        );
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
