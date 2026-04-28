//! Linux context provider (placeholder)

use crate::platform::traits::WindowContext;

/// Get the current foreground window context.
/// Returns `None` on Linux (not yet implemented).
pub fn get_current() -> Option<WindowContext> {
    None
}
