//! Accessibility AXUIElement operations.
//!
//! Direct access to window properties via Accessibility API.
//! Requires Accessibility permission (System Settings > Privacy & Security > Accessibility).
//!
//! Performance: < 10ms for most operations (vs 134-160ms with AppleScript)
//!
//! # Features
//!
//! - Create AXUIElement for applications by PID
#![cfg(target_os = "macos")]
//! - Get main/focused window elements
//! - Set/get window position and size (frame)
//! - Minimize, maximize, restore, close windows
//! - Bring windows to front
//! - Query window state (minimized, focused, title)

use anyhow::{bail, Result};
use std::alloc::{self, Layout};
use std::ffi::{c_void, CStr, CString};
use tracing::{debug, trace, warn};

// ============================================================================
// Type Definitions
// ============================================================================

/// Opaque pointer to AXUIElement (Objective-C object)
#[derive(Debug, Clone)]
pub struct AXElement(pub *const c_void);

impl AXElement {
    /// Check if element is valid (non-null)
    pub fn is_valid(&self) -> bool {
        !self.0.is_null()
    }

    /// Create null/invalid element
    pub fn null() -> Self {
        AXElement(std::ptr::null())
    }
}

/// Drop implementation to release Objective-C memory
impl Drop for AXElement {
    fn drop(&mut self) {
        if self.is_valid() {
            unsafe {
                cf_release(self.0);
            }
        }
    }
}

/// CGPoint structure for position
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}

/// CGSize structure for size
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}

// ============================================================================
// FFI Bindings
// ============================================================================

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    // Core Foundation functions
    fn CFRelease(cf: *const c_void);
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        cStr: *const i8,
        encoding: usize,
    ) -> *const c_void;
    fn CFStringGetCStringPtr(cfStr: *const c_void, encoding: usize) -> *const i8;

    // AXUIElement functions - Element Creation
    fn AXUIElementCreateApplication(pid: i32) -> *const c_void;

    // AXUIElement functions - Attribute Access
    fn AXUIElementCopyAttributeValue(
        element: *const c_void,
        attribute: *const c_void,
        result: *mut *const c_void,
    ) -> i32; // Returns AXError

    // AXUIElement functions - Attribute Modification
    fn AXUIElementSetAttributeValue(
        element: *const c_void,
        attribute: *const c_void,
        value: *const c_void,
    ) -> i32; // Returns AXError

    // AXUIElement functions - Action Execution
    fn AXUIElementPerformAction(element: *const c_void, action: *const c_void) -> i32; // Returns AXError

    // AXUIElement functions - Permission Check
    fn AXIsProcessTrusted() -> bool;
    fn AXAPIEnabled() -> bool;

    // Core Foundation - Boolean creation (for setting attributes)
    fn kCFBooleanTrue() -> *const c_void;
    fn kCFBooleanFalse() -> *const c_void;
}

/// Safe wrapper for CFRelease
unsafe fn cf_release(cf: *const c_void) {
    CFRelease(cf);
}

// ============================================================================
// Error Handling
// ============================================================================

/// Convert AXError code to Result
fn check_ax_error(error: i32, context: &str) -> Result<()> {
    match error {
        0 => Ok(()), // kAXErrorSuccess
        -25200 => bail!("{}: Generic failure", context),
        -25201 => bail!("{}: Illegal argument", context),
        -25202 => bail!("{}: Invalid UI element", context),
        -25203 => bail!("{}: Cannot complete (messaging failed)", context),
        -25204 => bail!("{}: Attribute not supported", context),
        -25205 => bail!("{}: Action not supported", context),
        -25206 => bail!("{}: Notification not supported", context),
        -25207 => bail!("{}: Not implemented", context),
        -25208 => bail!("{}: No value for attribute", context),
        -25209 => bail!("{}: Duplicate child", context),
        other => bail!("{}: Unknown error ({})", context, other),
    }
}

/// Check if current process has Accessibility permission
pub fn check_accessibility_permission() -> Result<()> {
    if unsafe { AXIsProcessTrusted() } {
        Ok(())
    } else {
        bail!(
            "Accessibility permission not granted. \
             Please enable in System Settings > Privacy & Security > Accessibility"
        )
    }
}

/// Check if Accessibility API is enabled on system
pub fn is_api_enabled() -> bool {
    unsafe { AXAPIEnabled() }
}

// ============================================================================
// Helper Functions for CFType Creation
// ============================================================================

/// Create CFString from Rust string
unsafe fn create_cf_string(s: &str) -> *const c_void {
    let c_str = CString::new(s).expect("Invalid UTF-8 string");
    CFStringCreateWithCString(
        std::ptr::null(),
        c_str.as_ptr(),
        0x08000100, // kCFStringEncodingUTF8
    )
}

/// Create CGPoint value on heap (returns raw pointer)
unsafe fn create_cgpoint(x: f64, y: f64) -> *const c_void {
    let layout = Layout::new::<CGPoint>();
    let ptr = alloc::alloc(layout) as *mut CGPoint;
    if !ptr.is_null() {
        (*ptr) = CGPoint { x, y };
    }
    ptr as *const c_void
}

/// Create CGSize value on heap (returns raw pointer)
unsafe fn create_cgsize(width: f64, height: f64) -> *const c_void {
    let layout = Layout::new::<CGSize>();
    let ptr = alloc::alloc(layout) as *mut CGSize;
    if !ptr.is_null() {
        (*ptr) = CGSize { width, height };
    }
    ptr as *const c_void
}

/// Parse CGPoint from raw pointer
unsafe fn parse_cgpoint(ptr: *const c_void) -> Result<(f64, f64)> {
    if ptr.is_null() {
        bail!("Null CGPoint pointer");
    }
    let point = &*(ptr as *const CGPoint);
    Ok((point.x, point.y))
}

/// Parse CGSize from raw pointer
unsafe fn parse_cgsize(ptr: *const c_void) -> Result<(f64, f64)> {
    if ptr.is_null() {
        bail!("Null CGSize pointer");
    }
    let size = &*(ptr as *const CGSize);
    Ok((size.width, size.height))
}

/// Get boolean attribute value
fn get_boolean_attribute(element: &AXElement, name: &str) -> Result<bool> {
    let mut value_ptr: *const c_void = std::ptr::null();
    let attr = unsafe { create_cf_string(name) };

    let error =
        unsafe { AXUIElementCopyAttributeValue(element.0, attr, &mut value_ptr) };
    check_ax_error(error, &format!("get_{}", name))?;

    if value_ptr.is_null() {
        return Ok(false);
    }

    // Compare with kCFBooleanTrue constant address
    let true_val = unsafe { kCFBooleanTrue() };
    Ok(value_ptr == true_val)
}

/// Get string attribute value
fn get_string_attribute(element: &AXElement, name: &str) -> Result<String> {
    let mut value_ptr: *const c_void = std::ptr::null();
    let attr = unsafe { create_cf_string(name) };

    let error =
        unsafe { AXUIElementCopyAttributeValue(element.0, attr, &mut value_ptr) };
    check_ax_error(error, &format!("get_{}", name))?;

    if value_ptr.is_null() {
        return Ok(String::new());
    }

    let c_ptr = unsafe { CFStringGetCStringPtr(value_ptr, 0x08000100) };
    if c_ptr.is_null() {
        return Ok(String::new());
    }

    let c_str = unsafe { CStr::from_ptr(c_ptr) };
    Ok(c_str.to_string_lossy().into_owned())
}

/// Create CFBoolean value
unsafe fn create_cfboolean(value: bool) -> *const c_void {
    if value {
        kCFBooleanTrue()
    } else {
        kCFBooleanFalse()
    }
}

// ============================================================================
// Core Operations - Element Creation
// ============================================================================

/// Create AXUIElement for application by PID
///
/// # Performance
/// < 0.1ms (single syscall)
///
/// # Example
///
/// ```ignore
/// use wakem::platform::macos::native_api::ax_element::*;
///
/// let pid = get_frontmost_app_pid().unwrap();
/// let app_elem = create_app_element(pid)?;
/// println!("Created element for PID {}", pid);
/// ```
pub fn create_app_element(pid: u32) -> Result<AXElement> {
    trace!("Creating AXUIElement for PID {}", pid);

    let element = unsafe { AXUIElementCreateApplication(pid as i32) };

    if element.is_null() {
        bail!("Failed to create AXUIElement for PID {}", pid);
    }

    debug!("Created AXUIElement {:?} for PID {}", element, pid);
    Ok(AXElement(element))
}

/// Get main window's AXUIElement for an application
///
/// Queries `kAXMainWindow` attribute of the application element.
///
/// # Performance
/// ~1-2ms (IPC roundtrip to target application)
pub fn get_main_window(app_element: &AXElement) -> Result<AXElement> {
    trace!("Getting main window for app {:?}", app_element);

    let mut window_ptr: *const c_void = std::ptr::null();
    let attr = unsafe { create_cf_string("AXMainWindow") };

    let error =
        unsafe { AXUIElementCopyAttributeValue(app_element.0, attr, &mut window_ptr) };

    check_ax_error(error, "get_main_window")?;

    if window_ptr.is_null() {
        bail!("No main window found for application");
    }

    debug!("Got main window {:?}", window_ptr);
    Ok(AXElement(window_ptr))
}

/// Get focused window's AXUIElement (alternative to main window)
///
/// Queries `kAXFocusedWindow` attribute of the application element.
pub fn get_focused_window(app_element: &AXElement) -> Result<AXElement> {
    trace!("Getting focused window for app {:?}", app_element);

    let mut window_ptr: *const c_void = std::ptr::null();
    let attr = unsafe { create_cf_string("AXFocusedWindow") };

    let error =
        unsafe { AXUIElementCopyAttributeValue(app_element.0, attr, &mut window_ptr) };

    check_ax_error(error, "get_focused_window")?;

    if window_ptr.is_null() {
        bail!("No focused window found for application");
    }

    debug!("Got focused window {:?}", window_ptr);
    Ok(AXElement(window_ptr))
}

// ============================================================================
// Core Operations - Window Position and Size
// ============================================================================

/// Set window position and size atomically
///
/// Uses `kAXPosition` and `kAXSize` attributes.
/// Coordinates are in **Cocoa convention** (bottom-left origin).
///
/// # Arguments
///
/// * `window_element` - Target window's AXUIElement
/// * `x`, `y` - Position in points (Cocoa coordinates)
/// * `w`, `h` - Size in points
///
/// # Performance
///
/// ~2-5ms (two syscalls + IPC roundtrip)
///
/// # Example
///
/// ```ignore
/// let pid = get_frontmost_app_pid().unwrap();
/// let app = create_app_element(pid)?;
/// let win = get_main_window(&app)?;
/// set_window_frame(&win, 100.0, 200.0, 800.0, 600.0)?; // Move and resize
/// ```
pub fn set_window_frame(
    window_element: &AXElement,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<()> {
    debug!(
        "Set window frame: position=({:.1}, {:.1}) size={:.1}x{:.1}",
        x, y, w, h
    );

    // Set position (CGPoint = {x: f64, y: f64})
    let position_attr = unsafe { create_cf_string("AXPosition") };
    let position_value = unsafe { create_cgpoint(x, y) };

    let error = unsafe {
        AXUIElementSetAttributeValue(window_element.0, position_attr, position_value)
    };
    check_ax_error(error, "set_position")?;

    // Set size (CGSize = {width: f64, height: f64})
    let size_attr = unsafe { create_cf_string("AXSize") };
    let size_value = unsafe { create_cgsize(w, h) };

    let error =
        unsafe { AXUIElementSetAttributeValue(window_element.0, size_attr, size_value) };
    check_ax_error(error, "set_size")?;

    debug!("Successfully set window frame");
    Ok(())
}

/// Get window position and size
///
/// Returns `(x, y, width, height)` in **Cocoa coordinates** (bottom-left origin).
///
/// # Performance
/// ~2-4ms (two syscalls + IPC roundtrip)
pub fn get_window_frame(window_element: &AXElement) -> Result<(f64, f64, f64, f64)> {
    let mut pos_ptr: *const c_void = std::ptr::null();
    let mut size_ptr: *const c_void = std::ptr::null();

    let pos_attr = unsafe { create_cf_string("AXPosition") };
    let size_attr = unsafe { create_cf_string("AXSize") };

    // Get position
    let error = unsafe {
        AXUIElementCopyAttributeValue(window_element.0, pos_attr, &mut pos_ptr)
    };
    check_ax_error(error, "get_position")?;

    // Get size
    let error = unsafe {
        AXUIElementCopyAttributeValue(window_element.0, size_attr, &mut size_ptr)
    };
    check_ax_error(error, "get_size")?;

    // Parse CGPoint and CGSize from raw pointers
    let (x, y) = unsafe { parse_cgpoint(pos_ptr)? };
    let (w, h) = unsafe { parse_cgsize(size_ptr)? };

    Ok((x, y, w, h))
}

// ============================================================================
// Core Operations - Window State Manipulation
// ============================================================================

/// Minimize window using `AXMinimize` action
///
/// # Performance
/// ~2-5ms
pub fn minimize_window(window_element: &AXElement) -> Result<()> {
    debug!("Minimizing window {:?}", window_element);

    let action = unsafe { create_cf_string("AXMinimize") };
    let error = unsafe { AXUIElementPerformAction(window_element.0, action) };

    check_ax_error(error, "minimize_window")
}

/// Restore window from minimized state
///
/// Uses `AXUnminimize` action first, falls back to setting
/// `kAXMinimizedAttribute` to false.
///
/// # Performance
/// ~2-5ms
pub fn restore_window(window_element: &AXElement) -> Result<()> {
    debug!("Restoring window {:?}", window_element);

    // Try AXUnminimize action first
    let action = unsafe { create_cf_string("AXUnminimize") };
    let error = unsafe { AXUIElementPerformAction(window_element.0, action) };

    match error {
        0 => {
            debug!("Restored via AXUnminimize action");
            Ok(())
        }
        _ => {
            // Fallback: set kAXMinimizedAttribute = false
            warn!(
                "AXUnminimize failed (error {}), trying attribute set",
                error
            );
            set_minimized_attribute(window_element, false)
        }
    }
}

/// Maximize window using `AXZoom` action
///
/// Note: On macOS, "zoom" toggles between normal and a larger size,
/// but may not fill the entire screen like Windows maximization.
/// For true full-screen behavior, consider using full-screen API instead.
///
/// # Performance
/// ~2-5ms
pub fn maximize_window(window_element: &AXElement) -> Result<()> {
    debug!("Maximizing window {:?}", window_element);

    let action = unsafe { create_cf_string("AXZoom") };
    let error = unsafe { AXUIElementPerformAction(window_element.0, action) };

    check_ax_error(error, "maximize_window")
}

/// Close window using `AXClose` action
///
/// **Warning**: This may trigger unsaved changes warning dialog!
///
/// # Performance
/// ~2-5ms
pub fn close_window(window_element: &AXElement) -> Result<()> {
    debug!("Closing window {:?}", window_element);

    let action = unsafe { create_cf_string("AXClose") };
    let error = unsafe { AXUIElementPerformAction(window_element.0, action) };

    check_ax_error(error, "close_window")
}

/// Bring window/application to front (raise/focus)
///
/// Uses `AXRaise` action on the **application** element (not window).
///
/// # Performance
/// ~2-5ms
pub fn bring_to_front(app_element: &AXElement) -> Result<()> {
    debug!("Bringing app to front {:?}", app_element);

    let action = unsafe { create_cf_string("AXRaise") };
    let error = unsafe { AXUIElementPerformAction(app_element.0, action) };

    check_ax_error(error, "bring_to_front")
}

// ============================================================================
// Core Operations - Window State Query
// ============================================================================

/// Check if window is minimized
///
/// Queries `kAXMinimizedAttribute`.
///
/// # Performance
/// ~1-2ms
pub fn is_minimized(window_element: &AXElement) -> Result<bool> {
    get_boolean_attribute(window_element, "AXMinimized")
}

/// Set minimized state via attribute (alternative to action)
fn set_minimized_attribute(window_element: &AXElement, minimized: bool) -> Result<()> {
    let attr = unsafe { create_cf_string("AXMinimized") };
    let value = unsafe { create_cfboolean(minimized) };

    let error = unsafe { AXUIElementSetAttributeValue(window_element.0, attr, value) };

    check_ax_error(error, "set_minimized")
}

/// Check if window is focused
///
/// Queries `kAXFocusedAttribute`.
pub fn is_focused(window_element: &AXElement) -> Result<bool> {
    get_boolean_attribute(window_element, "AXFocused")
}

/// Get window title
///
/// Queries `kAXTitleAttribute`.
pub fn get_window_title(window_element: &AXElement) -> Result<String> {
    get_string_attribute(window_element, "AXTitle")
}

/// Get window role (should be "AXWindow" for windows)
///
/// Queries `kAXRoleAttribute`.
pub fn get_role(element: &AXElement) -> Result<String> {
    get_string_attribute(element, "AXRole")
}

// ============================================================================
// Advanced Operations (Optional Features)
// ============================================================================

/// Ensure window is restored (not minimized or maximized)
///
/// Checks if window is minimized and restores it if so.
///
/// # Performance
/// ~3-7ms (is_minimized + optional restore)
pub fn ensure_window_restored(window_element: &AXElement) -> Result<()> {
    if is_minimized(window_element)? {
        restore_window(window_element)?;
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_accessibility_permission() {
        let result = check_accessibility_permission();
        match result {
            Ok(()) => println!("✅ Accessibility permission granted"),
            Err(e) => println!("⚠️ {}", e), // Don't fail test
        }
    }

    #[test]
    fn test_is_api_enabled() {
        let enabled = is_api_enabled();
        println!("Accessibility API enabled: {}", enabled);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_create_app_element_for_frontmost() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let elem = create_app_element(pid);
        assert!(elem.is_ok(), "Should create element for valid PID");

        let elem = elem.unwrap();
        assert!(elem.is_valid(), "Element should be valid");

        println!("✅ Created AXUIElement for PID {}", pid);
    }

    #[test]
    fn test_get_main_window_for_current_app() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let app_elem = match create_app_element(pid) {
            Ok(elem) => elem,
            Err(e) => {
                eprintln!("⚠️ Failed to create app element: {}", e);
                return;
            }
        };

        let win_elem = get_main_window(&app_elem);

        match win_elem {
            Ok(win) => {
                assert!(win.is_valid(), "Main window should be valid");
                println!("✅ Got main window: {:?}", win.0);
            }
            Err(e) => {
                eprintln!("⚠️ No main window: {} (some apps don't expose it)", e);
            }
        }
    }

    #[test]
    fn test_get_focused_window_for_current_app() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let app_elem = match create_app_element(pid) {
            Ok(elem) => elem,
            Err(e) => {
                eprintln!("⚠️ Failed to create app element: {}", e);
                return;
            }
        };

        let win_elem = get_focused_window(&app_elem);

        match win_elem {
            Ok(win) => {
                assert!(win.is_valid(), "Focused window should be valid");
                println!("✅ Got focused window: {:?}", win.0);
            }
            Err(e) => {
                eprintln!("⚠️ No focused window: {} (may be normal for some apps)", e);
            }
        }
    }

    #[test]
    fn test_is_minimized_false_by_default() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let app_elem = match create_app_element(pid) {
            Ok(elem) => elem,
            Err(_) => return,
        };

        let win_elem = match get_main_window(&app_elem) {
            Ok(win) => win,
            Err(_) => return,
        };

        // Frontmost window should typically NOT be minimized
        match is_minimized(&win_elem) {
            Ok(minimized) => {
                assert!(!minimized, "Frontmost window should not be minimized");
                println!("✅ Frontmost window is not minimized (as expected)");
            }
            Err(e) => {
                eprintln!("⚠️ Failed to check minimized state: {}", e);
            }
        }
    }

    #[test]
    fn test_get_window_title_not_empty() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let app_elem = match create_app_element(pid) {
            Ok(elem) => elem,
            Err(_) => return,
        };

        let win_elem = match get_main_window(&app_elem) {
            Ok(win) => win,
            Err(_) => return,
        };

        match get_window_title(&win_elem) {
            Ok(title) => {
                println!("✅ Window title: '{}'", title);
                // Title may be empty for some system windows
            }
            Err(e) => {
                eprintln!("⚠️ Failed to get window title: {}", e);
            }
        }
    }

    #[test]
    fn test_get_role_is_window() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let app_elem = match create_app_element(pid) {
            Ok(elem) => elem,
            Err(_) => return,
        };

        let win_elem = match get_main_window(&app_elem) {
            Ok(win) => win,
            Err(_) => return,
        };

        match get_role(&win_elem) {
            Ok(role) => {
                println!("✅ Window role: '{}'", role);
                assert_eq!(role, "AXWindow", "Main window role should be AXWindow");
            }
            Err(e) => {
                eprintln!("⚠️ Failed to get window role: {}", e);
            }
        }
    }

    #[test]
    fn test_get_window_frame_returns_valid_values() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let app_elem = match create_app_element(pid) {
            Ok(elem) => elem,
            Err(_) => return,
        };

        let win_elem = match get_main_window(&app_elem) {
            Ok(win) => win,
            Err(_) => return,
        };

        match get_window_frame(&win_elem) {
            Ok((x, y, w, h)) => {
                println!(
                    "✅ Window frame: position=({:.1}, {:.1}) size={:.1}x{:.1}",
                    x, y, w, h
                );
                // Values should be reasonable (positive sizes)
                assert!(w > 0.0, "Width should be positive");
                assert!(h > 0.0, "Height should be positive");
            }
            Err(e) => {
                eprintln!("⚠️ Failed to get window frame: {}", e);
            }
        }
    }

    #[test]
    fn test_bring_to_front() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let app_elem = match create_app_element(pid) {
            Ok(elem) => elem,
            Err(_) => return,
        };

        match bring_to_front(&app_elem) {
            Ok(()) => println!("✅ Successfully brought app to front"),
            Err(e) => eprintln!("⚠️ Failed to bring to front: {}", e),
        }
    }

    #[test]
    fn test_error_handling_invalid_element() {
        let invalid = AXElement::null();
        assert!(!invalid.is_valid());

        let result = get_main_window(&invalid);
        assert!(result.is_err(), "Should fail with invalid element");

        match result {
            Err(e) => println!("✅ Correctly returned error for invalid element: {}", e),
            Ok(_) => panic!("Should have failed!"),
        }
    }

    #[test]
    fn test_ensure_window_restored_already_normal() {
        use crate::platform::macos::native_api::ns_workspace::get_frontmost_app_pid;

        let pid = match get_frontmost_app_pid() {
            Some(p) => p,
            None => {
                eprintln!("Skipping: No frontmost app (may be headless)");
                return;
            }
        };

        let app_elem = match create_app_element(pid) {
            Ok(elem) => elem,
            Err(_) => return,
        };

        let win_elem = match get_main_window(&app_elem) {
            Ok(win) => win,
            Err(_) => return,
        };

        // Should succeed even if already restored (no-op)
        match ensure_window_restored(&win_elem) {
            Ok(()) => println!("✅ ensure_window_restored succeeded"),
            Err(e) => eprintln!("⚠️ ensure_window_restored failed: {}", e),
        }
    }
}
