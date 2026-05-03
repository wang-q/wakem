//! Windows window manager implementation
#![cfg(target_os = "windows")]

use crate::platform::windows::window_api::RealWindowApi;

pub type WindowManager =
    crate::platform::common::window_manager::WindowManager<RealWindowApi>;
