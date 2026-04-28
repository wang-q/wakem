//! Shared `SendInputDevice` type definition used by both Windows and macOS
//! output device implementations. Platform-specific trait impls remain in
//! their respective `output_device.rs` files.

/// Platform output device backed by the operating system's input simulation API.
/// Windows uses `SendInput`; macOS uses `CGEvent`. The struct itself is
/// platform-agnostic (a unit struct).
#[derive(Debug, Clone)]
pub struct SendInputDevice;

impl SendInputDevice {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SendInputDevice {
    fn default() -> Self {
        Self::new()
    }
}
