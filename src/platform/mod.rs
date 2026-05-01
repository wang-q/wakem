//! Platform abstraction layer
//!
//! This module provides cross-platform abstractions for:
//! - Input device capture (keyboard/mouse)
//! - Output device simulation (sending input events)
//! - Window management
//! - System tray integration
//!
//! ## Architecture
//!
//! The module is organized in three layers:
//!
//! 1. **Types & traits** (`types.rs`, `traits.rs`, `macros.rs`) — shared
//!    data types, trait interfaces, and code-generation macros.
//!
//! 2. **Common implementations** (`common/`) — platform-agnostic
//!    implementations that work across all supported platforms.
//!
//! 3. **Platform modules** (`windows/`, `macos/`) —
//!    platform-specific code selected via conditional compilation.
//!
//! The module uses conditional compilation to select the appropriate
//! platform-specific implementation via the [`CurrentPlatform`] type alias.

// ---------------------------------------------------------------------------
// Layer 1: Types & traits (shared across all platforms)
// ---------------------------------------------------------------------------
pub mod macros;
pub mod traits;
pub mod types;

// ---------------------------------------------------------------------------
// Layer 2: Common implementations (platform-agnostic logic)
// ---------------------------------------------------------------------------
pub mod common;

// ---------------------------------------------------------------------------
// Layer 3: Platform-specific modules (selected via conditional compilation)
// ---------------------------------------------------------------------------
#[cfg(any(test, feature = "test-utils"))]
pub mod mock;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// ---------------------------------------------------------------------------
// Current platform type alias
// ---------------------------------------------------------------------------

/// Current platform's factory type
///
/// Use `CurrentPlatform::create_*()` to create platform-specific objects
/// without conditional compilation in business logic.
#[cfg(target_os = "windows")]
pub type CurrentPlatform = windows::WindowsPlatform;

#[cfg(target_os = "macos")]
pub type CurrentPlatform = macos::MacosPlatform;
