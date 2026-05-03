//! macOS window manager implementation
//!
//! Provides window management operations on macOS.
#![cfg(target_os = "macos")]

use crate::platform::macos::window_api::RealMacosWindowApi;

pub type WindowManager =
    crate::platform::common::window_manager::WindowManager<RealMacosWindowApi>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::macos::window_api::MockMacosWindowApi;

    #[test]
    fn test_window_manager_creation() {
        let _wm = WindowManager::with_api(MockMacosWindowApi::new());
    }
}
