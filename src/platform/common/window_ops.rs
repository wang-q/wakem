//! Common window operations shared across platforms
//!
//! This module provides platform-agnostic window management algorithms
//! that work with any platform-specific window manager implementation.

use crate::platform::types::{MonitorInfo, WindowFrame, WindowInfo};
use crate::types::{Alignment, Edge};

/// Find the monitor that contains the given point, falling back to the first monitor.
pub fn find_monitor_for_point(
    monitors: &[MonitorInfo],
    x: i32,
    y: i32,
) -> Option<&MonitorInfo> {
    monitors
        .iter()
        .find(|m| x >= m.x && x < m.x + m.width && y >= m.y && y < m.y + m.height)
        .or_else(|| monitors.first())
}

/// Find the next ratio in the cycle after the current one.
pub fn find_next_ratio(ratios: &[f32], current: f32) -> f32 {
    let closest_idx = ratios
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (current - **a)
                .abs()
                .partial_cmp(&(current - **b).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    ratios[(closest_idx + 1) % ratios.len()]
}

/// Calculate centered position for a window on its current monitor.
pub fn calc_centered_pos(
    window: &WindowInfo,
    monitors: &[MonitorInfo],
) -> Option<(i32, i32)> {
    let monitor = find_monitor_for_point(monitors, window.x, window.y)?;
    let frame = WindowFrame::new(window.x, window.y, window.width, window.height);
    let (x, y) = frame.center_in(monitor);
    Some((x, y))
}

/// Calculate position for moving window to edge of screen.
pub fn calc_edge_pos(
    window: &WindowInfo,
    monitors: &[MonitorInfo],
    edge: Edge,
) -> Option<(i32, i32)> {
    let monitor = find_monitor_for_point(monitors, window.x, window.y)?;

    let (x, y) = match edge {
        Edge::Left => (monitor.x, window.y),
        Edge::Right => (monitor.x + monitor.width - window.width, window.y),
        Edge::Top => (window.x, monitor.y),
        Edge::Bottom => (window.x, monitor.y + monitor.height - window.height),
    };

    Some((x, y))
}

/// Calculate half-screen dimensions and position.
pub fn calc_half_screen(
    window: &WindowInfo,
    monitors: &[MonitorInfo],
    edge: Edge,
) -> Option<(i32, i32, i32, i32)> {
    let monitor = find_monitor_for_point(monitors, window.x, window.y)?;

    let (x, y, w, h) = match edge {
        Edge::Left => (monitor.x, monitor.y, monitor.width / 2, monitor.height),
        Edge::Right => {
            let w = monitor.width / 2;
            (monitor.x + monitor.width - w, monitor.y, w, monitor.height)
        }
        Edge::Top => (monitor.x, monitor.y, monitor.width, monitor.height / 2),
        Edge::Bottom => {
            let h = monitor.height / 2;
            (monitor.x, monitor.y + monitor.height - h, monitor.width, h)
        }
    };

    Some((x, y, w, h))
}

/// Calculate next width in loop cycle.
pub fn calc_looped_width(
    window: &WindowInfo,
    monitors: &[MonitorInfo],
    align: Alignment,
) -> Option<(i32, i32, i32, i32)> {
    const WIDTH_RATIOS: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];

    let monitor = find_monitor_for_point(monitors, window.x, window.y)?;
    let current_ratio = window.width as f32 / monitor.width as f32;
    let next_ratio = find_next_ratio(&WIDTH_RATIOS, current_ratio);

    let new_width = (monitor.width as f32 * next_ratio) as i32;
    let new_x = match align {
        Alignment::Left => monitor.x,
        Alignment::Right => monitor.x + monitor.width - new_width,
        _ => window.x,
    };

    Some((new_x, window.y, new_width, window.height))
}

/// Calculate next height in loop cycle.
pub fn calc_looped_height(
    window: &WindowInfo,
    monitors: &[MonitorInfo],
    align: Alignment,
) -> Option<(i32, i32, i32, i32)> {
    const HEIGHT_RATIOS: [f32; 3] = [0.75, 0.5, 0.25];

    let monitor = find_monitor_for_point(monitors, window.x, window.y)?;
    let current_ratio = window.height as f32 / monitor.height as f32;
    let next_ratio = find_next_ratio(&HEIGHT_RATIOS, current_ratio);

    let new_height = (monitor.height as f32 * next_ratio) as i32;
    let new_y = match align {
        Alignment::Top => monitor.y,
        Alignment::Bottom => monitor.y + monitor.height - new_height,
        _ => window.y,
    };

    Some((window.x, new_y, window.width, new_height))
}

/// Calculate fixed ratio window dimensions.
pub fn calc_fixed_ratio(
    window: &WindowInfo,
    monitors: &[MonitorInfo],
    ratio: f32,
    scale_index: Option<usize>,
) -> Option<(i32, i32, i32, i32)> {
    const SCALES: [f32; 4] = [1.0, 0.9, 0.7, 0.5];

    let monitor = find_monitor_for_point(monitors, window.x, window.y)?;
    let base_size = std::cmp::min(monitor.width, monitor.height);
    let base_width = (base_size as f32 * ratio) as i32;
    let base_height = base_size;

    let next_scale = match scale_index {
        Some(idx) if idx < SCALES.len() => SCALES[idx],
        Some(_) => return None,
        None => {
            let current_scale = (window.width as f32 / base_width as f32
                + window.height as f32 / base_height as f32)
                / 2.0;
            find_next_ratio(&SCALES, current_scale)
        }
    };

    let new_width = (base_width as f32 * next_scale) as i32;
    let new_height = (base_height as f32 * next_scale) as i32;
    let new_x = monitor.x + (monitor.width - new_width) / 2;
    let new_y = monitor.y + (monitor.height - new_height) / 2;

    Some((new_x, new_y, new_width, new_height))
}

/// Calculate native ratio (screen ratio) window dimensions.
pub fn calc_native_ratio(
    window: &WindowInfo,
    monitors: &[MonitorInfo],
    scale_index: Option<usize>,
) -> Option<(i32, i32, i32, i32)> {
    let monitor = find_monitor_for_point(monitors, window.x, window.y)?;
    let ratio = monitor.width as f32 / monitor.height as f32;
    calc_fixed_ratio(window, monitors, ratio, scale_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_monitor() -> MonitorInfo {
        MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }

    fn test_window() -> WindowInfo {
        WindowInfo {
            id: 1,
            title: "Test".to_string(),
            process_name: "test.exe".to_string(),
            executable_path: None,
            x: 100,
            y: 100,
            width: 800,
            height: 600,
        }
    }

    #[test]
    fn test_find_monitor_for_point() {
        let monitors = vec![test_monitor()];
        let m = find_monitor_for_point(&monitors, 500, 500).unwrap();
        assert_eq!(m.width, 1920);
    }

    #[test]
    fn test_find_next_ratio() {
        let ratios: [f32; 5] = [0.75, 0.6, 0.5, 0.4, 0.25];
        assert!((find_next_ratio(&ratios, 0.75) - 0.6).abs() < 0.001);
        assert!((find_next_ratio(&ratios, 0.25) - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_calc_centered_pos() {
        let monitors = vec![test_monitor()];
        let window = test_window();
        let (x, y) = calc_centered_pos(&window, &monitors).unwrap();
        assert_eq!(x, 560); // (1920 - 800) / 2
        assert_eq!(y, 240); // (1080 - 600) / 2
    }

    #[test]
    fn test_calc_half_screen() {
        let monitors = vec![test_monitor()];
        let window = test_window();

        let (x, y, w, h) = calc_half_screen(&window, &monitors, Edge::Left).unwrap();
        assert_eq!(x, 0);
        assert_eq!(w, 960); // 1920 / 2
        assert_eq!(h, 1080);

        let (x, y, w, h) = calc_half_screen(&window, &monitors, Edge::Right).unwrap();
        assert_eq!(x, 960);
        assert_eq!(w, 960);
    }

    #[test]
    fn test_calc_looped_width() {
        let monitors = vec![test_monitor()];
        let window = WindowInfo {
            width: 960, // 50% of 1920
            ..test_window()
        };

        let (x, y, w, h) =
            calc_looped_width(&window, &monitors, Alignment::Left).unwrap();
        assert_eq!(w, 768); // 40% of 1920
    }

    #[test]
    fn test_calc_fixed_ratio() {
        let monitors = vec![test_monitor()];
        let window = test_window();

        let (x, y, w, h) =
            calc_fixed_ratio(&window, &monitors, 4.0 / 3.0, None).unwrap();
        // Based on min(1920, 1080) = 1080, ratio 4:3, scale 100%
        assert_eq!(h, 1080);
        assert_eq!(w, 1440); // 1080 * 4/3
    }
}
