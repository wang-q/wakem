//! Native macOS API wrappers for high-performance window operations.
//!
//! Replaces all AppleScript calls with direct system framework calls:
//! - Core Graphics: CGWindowList, CGDisplay
//! - Accessibility: AXUIElement API
//! - Cocoa: NSWorkspace, NSRunningApplication
//!
//! # Performance
//!
//! All operations complete in < 10ms (typically < 5ms), compared to 130-180ms with AppleScript.
//!
//! # Permissions
//!
//! Requires Accessibility permission:
//! System Settings → Privacy & Security → Accessibility

pub mod ax_element;
pub mod cg_window;
mod coordinate;
pub mod display;
pub mod notification;
pub mod ns_workspace;

// Re-export common types and functions for convenience
pub use coordinate::{cg_to_windows, windows_to_cg};
pub use notification::show_notification;
