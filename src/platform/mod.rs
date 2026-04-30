//! Platform abstraction layer
//!
//! This module provides a three-layer architecture for cross-platform support:
//!
//! 1. **Traits** (`traits.rs`): Platform-agnostic interfaces
//! 2. **Common** (`common/`): Cross-platform implementations shared between platforms
//! 3. **Platform-specific** (`windows/`, `macos/`): Platform-specific implementations
//!
//! # Module Organization
//!
//! - `traits.rs` - Core trait definitions (WindowApi, InputDevice, etc.)
//! - `types.rs` - Shared platform types
//! - `macros.rs` - Shared macros
//! - `mock.rs` - Mock implementations for testing
//! - `common/` - Cross-platform implementations
//!   - `input_device.rs`
//!   - `launcher.rs`
//!   - `output_helpers.rs`
//!   - `tray.rs`
//!   - `window_preset.rs`
//! - `windows/` - Windows-specific implementations
//! - `macos/` - macOS-specific implementations

// Core modules
pub mod common;
pub mod macros;
pub mod mock;
pub mod traits;
pub mod types;

// Platform-specific modules
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

/// Current platform's factory type
///
/// Use `CurrentPlatform::create_*()` to create platform-specific objects
/// without conditional compilation in business logic.
#[cfg(target_os = "windows")]
pub type CurrentPlatform = windows::WindowsPlatform;

#[cfg(target_os = "macos")]
pub type CurrentPlatform = macos::MacosPlatform;
