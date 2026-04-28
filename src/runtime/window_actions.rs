//! Window action execution — platform-agnostic dispatch of [`WindowAction`] variants.
//!
//! Extracted from [`KeyMapper`](super::KeyMapper) to keep the mapper focused on
//! rule matching and event processing.

use crate::platform::traits::{WindowManagerExt, WindowManagerTrait};
use crate::types::{MonitorDirection, WindowAction};
use tracing::debug;

use super::mapper::{NotificationServiceRef, WindowPresetManagerRef};

/// Execute a window action using the provided window manager and optional services.
pub fn execute_window_action(
    wm: &dyn WindowManagerTrait,
    notification_service: Option<&NotificationServiceRef>,
    preset_manager: Option<&WindowPresetManagerRef>,
    action: &WindowAction,
) -> anyhow::Result<()> {
    let window_id = wm
        .get_foreground_window()
        .ok_or_else(|| anyhow::anyhow!("No foreground window"))?;

    match action {
        WindowAction::Center => {
            wm.move_to_center(window_id)?;
        }
        WindowAction::MoveToEdge(edge) => {
            wm.move_to_edge(window_id, *edge)?;
        }
        WindowAction::HalfScreen(edge) => {
            wm.set_half_screen(window_id, *edge)?;
        }
        WindowAction::LoopWidth(align) => {
            wm.loop_width(window_id, *align)?;
        }
        WindowAction::LoopHeight(align) => {
            wm.loop_height(window_id, *align)?;
        }
        WindowAction::FixedRatio { ratio, scale_index } => {
            wm.set_fixed_ratio(window_id, *ratio, Some(*scale_index))?;
        }
        WindowAction::NativeRatio { scale_index } => {
            wm.set_native_ratio(window_id, Some(*scale_index))?;
        }
        WindowAction::SwitchToNextWindow => {
            debug!("SwitchToNextWindow action triggered (not yet implemented)");
            if let Some(current_info) = wm
                .get_foreground_window()
                .and_then(|w| wm.get_window_info(w).ok())
            {
                debug!(
                    process_name = %current_info.process_name,
                    window_title = %current_info.title,
                    "SwitchToNextWindow: Would switch to next window of same process"
                );
            }
        }
        WindowAction::MoveToMonitor(direction) => {
            execute_move_to_monitor(wm, window_id, direction)?;
        }
        WindowAction::Move { x, y } => {
            wm.set_window_pos(window_id, *x, *y, 0, 0)?;
        }
        WindowAction::Resize { width, height } => {
            let info = wm.get_window_info(window_id)?;
            wm.set_window_pos(window_id, info.x, info.y, *width, *height)?;
        }
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
        WindowAction::ToggleTopmost => {
            wm.toggle_topmost(window_id)?;
        }
        WindowAction::ShowDebugInfo => match wm.get_window_info(window_id) {
            Ok(info) => {
                debug!(?info, "Window debug info");
            }
            Err(e) => {
                debug!("Failed to get debug info: {}", e);
            }
        },
        WindowAction::ShowNotification { title, message } => {
            if let Some(ns) = notification_service {
                let ns = ns.lock();
                if let Err(e) = ns.show(title, message) {
                    debug!("Failed to show notification: {}", e);
                }
            }
        }
        WindowAction::SavePreset { name } => {
            execute_save_preset(preset_manager, notification_service, name)?;
        }
        WindowAction::LoadPreset { name } => {
            if let Some(pm) = preset_manager {
                let pm = pm.read();
                if let Err(e) = pm.load_preset(name) {
                    debug!("Failed to load preset '{}': {}", name, e);
                } else {
                    debug!("Loaded preset '{}' for current window", name);
                }
            } else {
                debug!("WindowPresetManager not available, cannot load preset");
            }
        }
        WindowAction::ApplyPreset => {
            if let Some(pm) = preset_manager {
                let pm = pm.read();
                match pm.apply_preset_for_window_by_id(window_id) {
                    Ok(true) => debug!("Applied matching preset to current window"),
                    Ok(false) => debug!("No matching preset found for current window"),
                    Err(e) => debug!("Failed to apply preset: {}", e),
                }
            } else {
                debug!("WindowPresetManager not available, cannot apply preset");
            }
        }
        WindowAction::None => {}
    }

    Ok(())
}

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
            let current_monitor_idx =
                crate::platform::traits::find_monitor_index_for_point(&monitors, cx, cy);

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

fn execute_save_preset(
    preset_manager: Option<&WindowPresetManagerRef>,
    notification_service: Option<&NotificationServiceRef>,
    name: &str,
) -> anyhow::Result<()> {
    let Some(pm) = preset_manager else {
        debug!("WindowPresetManager not available, cannot save preset");
        return Ok(());
    };

    let mut pm = pm.write();
    match pm.get_foreground_window_info() {
        Some(Ok(_)) => {
            if let Err(e) = pm.save_preset(name.to_string()) {
                debug!("Failed to save preset '{}': {}", name, e);
            } else {
                debug!("Saved preset '{}' for current window", name);
                if let Some(ns) = notification_service {
                    let ns = ns.lock();
                    if let Err(e) = ns.show("wakem", &format!("Preset '{}' saved", name))
                    {
                        debug!("Failed to show preset saved notification: {}", e);
                    }
                }
            }
        }
        Some(Err(e)) => {
            debug!("Failed to get foreground window info: {}", e);
        }
        None => {
            debug!("No foreground window found");
        }
    }
    Ok(())
}
