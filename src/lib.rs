//! wakem - Window Adjust, Keyboard Enhance, and Mouse
//!
//! A cross-platform window management, keyboard enhancement, and mouse enhancement tool.

pub mod cli;
pub mod client;
pub mod config;
pub mod constants;
pub mod daemon;
pub mod ipc;
pub mod platform;
pub mod runtime;
pub mod shutdown;
pub mod types;

pub use config::Config;
