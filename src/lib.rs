//! wakem - Window Adjust, Keyboard Enhance, and Mouse
//!
//! 一个跨平台的窗口管理、键盘增强、鼠标增强工具。

pub mod cli;
pub mod client;
pub mod config;
pub mod daemon;
pub mod ipc;
pub mod platform;
pub mod runtime;
pub mod shutdown;
pub mod types;

pub use config::Config;
pub use types::*;
