//! Window action execution — platform-agnostic dispatch of [`WindowAction`] variants.
//!
//! Extracted from [`KeyMapper`](super::KeyMapper) to keep the mapper focused on
//! rule matching and event processing.
//!
//! # Current Status
//!
//! This module provides a simplified implementation that supports basic window
//! operations available through [`WindowManagerTrait`]. Advanced operations
//! (Center, HalfScreen, LoopWidth, etc.) will be added when the corresponding
//! methods are implemented in the platform layer.

use crate::platform::traits::WindowManagerTrait;
use crate::types::{MonitorDirection, WindowAction};
use tracing::debug;

/// Execute a window action using the provided window manager.
///
/// Supports basic window operations:
/// - Move, Resize (position and size)
/// - Minimize, Maximize, Restore, Close (window state)
/// - MoveToMonitor (multi-monitor)
/// - ToggleTopmost (z-order)
///
/// Advanced operations (Center, HalfScreen, LoopWidth, etc.) log debug
/// messages but are not yet implemented.
#[allow(dead_code)]
pub fn execute_window_action(
    wm: &dyn WindowManagerTrait,
    action: &WindowAction,
) -> anyhow::Result<()> {
    let window_id = wm
        .get_foreground_window()
        .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

    match action {
        // Basic position and size operations
        WindowAction::Move { x, y } => {
            let info = wm.get_window_info(window_id)?;
            wm.set_window_pos(window_id, *x, *y, info.width, info.height)?;
        }
        WindowAction::Resize { width, height } => {
            let info = wm.get_window_info(window_id)?;
            wm.set_window_pos(window_id, info.x, info.y, *width, *height)?;
        }

        // Window state operations
        WindowAction::Minimize => {
            wm.minimize_window(window_id)?;
        }
        WindowAction::Maximize => {
            wm.maximize_window(window_id)?;
        }
        WindowAction::Restore => {
            wm.restore_window(window_id)?;
        }
        WindowAction::Close => {
            wm.close_window(window_id)?;
        }

        // Z-order operations
        WindowAction::ToggleTopmost => {
            let is_top = wm.is_window_valid(window_id); // Reuse as placeholder
            wm.set_topmost(window_id, !is_top)?;
        }

        // Multi-monitor operations
        WindowAction::MoveToMonitor(direction) => {
            execute_move_to_monitor(wm, window_id, direction)?;
        }

        // Advanced operations (not yet implemented)
        WindowAction::Center => {
            debug!("Center action: not yet implemented");
        }
        WindowAction::MoveToEdge(edge) => {
            debug!(?edge, "MoveToEdge action: not yet implemented");
        }
        WindowAction::HalfScreen(edge) => {
            debug!(?edge, "HalfScreen action: not yet implemented");
        }
        WindowAction::LoopWidth(align) => {
            debug!(?align, "LoopWidth action: not yet implemented");
        }
        WindowAction::LoopHeight(align) => {
            debug!(?align, "LoopHeight action: not yet implemented");
        }
        WindowAction::FixedRatio { ratio, scale_index } => {
            debug!(ratio, scale_index, "FixedRatio action: not yet implemented");
        }
        WindowAction::NativeRatio { scale_index } => {
            debug!(scale_index, "NativeRatio action: not yet implemented");
        }
        WindowAction::SwitchToNextWindow => {
            debug!("SwitchToNextWindow action: not yet implemented");
        }

        // Information and notification operations
        WindowAction::ShowDebugInfo => match wm.get_window_info(window_id) {
            Ok(info) => {
                debug!(?info, "Window debug info");
            }
            Err(e) => {
                debug!("Failed to get debug info: {}", e);
            }
        },
        WindowAction::ShowNotification { title, message } => {
            debug!(
                title,
                message, "ShowNotification: notification service not available"
            );
        }

        // Preset operations (require WindowPresetManager)
        WindowAction::SavePreset { name } => {
            debug!(name, "SavePreset: preset manager not available");
        }
        WindowAction::LoadPreset { name } => {
            debug!(name, "LoadPreset: preset manager not available");
        }
        WindowAction::ApplyPreset => {
            debug!("ApplyPreset: preset manager not available");
        }

        WindowAction::None => {}
    }

    Ok(())
}

#[allow(dead_code)]
fn execute_move_to_monitor(
    wm: &dyn WindowManagerTrait,
    window_id: usize,
    direction: &MonitorDirection,
) -> anyhow::Result<()> {
    let monitors = wm.get_monitors();
    if monitors.is_empty() {
        anyhow::bail!("No monitors found");
    }

    let monitor_index: usize = match direction {
        MonitorDirection::Index(idx) => {
            let idx = *idx as usize;
            if idx >= monitors.len() {
                anyhow::bail!(
                    "Monitor index {} out of range (0-{})",
                    idx,
                    monitors.len() - 1
                );
            }
            idx
        }
        MonitorDirection::Next | MonitorDirection::Prev => {
            let info = wm.get_window_info(window_id)?;
            let cx = info.x + info.width / 2;
            let cy = info.y + info.height / 2;

            let current_monitor_idx = find_monitor_index_for_point(&monitors, cx, cy);

            match direction {
                MonitorDirection::Next => (current_monitor_idx + 1) % monitors.len(),
                MonitorDirection::Prev => {
                    if current_monitor_idx == 0 {
                        monitors.len() - 1
                    } else {
                        current_monitor_idx - 1
                    }
                }
                _ => unreachable!(),
            }
        }
    };
    wm.move_to_monitor(window_id, monitor_index)?;
    Ok(())
}

/// Find which monitor contains the given point (center of window).
#[allow(dead_code)]
fn find_monitor_index_for_point(
    monitors: &[crate::platform::traits::MonitorInfo],
    x: i32,
    y: i32,
) -> usize {
    for (i, monitor) in monitors.iter().enumerate() {
        if x >= monitor.x
            && x < monitor.x + monitor.width
            && y >= monitor.y
            && y < monitor.y + monitor.height
        {
            return i;
        }
    }
    0
}
