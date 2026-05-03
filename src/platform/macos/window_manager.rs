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
    use crate::platform::mock::mock_window_api::MockWindowApiBase;

    #[test]
    fn test_window_manager_creation() {
        let _wm = WindowManager::with_api(MockWindowApiBase::new());
    }
}
