//! Coordinate system conversion utilities.
//!
//! macOS uses bottom-left origin (Core Graphics).
//! Windows/wakem uses top-left origin.
//! This module provides conversion functions.
#![cfg(target_os = "macos")]

/// Convert Y-coordinate from Windows-style (top-left) to CG-style (bottom-left)
pub fn windows_to_cg(y: f64, screen_height: f64) -> f64 {
    screen_height - y
}

/// Convert Y-coordinate from CG-style (bottom-left) to Windows-style (top-left)
pub fn cg_to_windows(y: f64, screen_height: f64) -> f64 {
    screen_height - y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_conversion() {
        let screen_height = 1080.0;

        assert_eq!(windows_to_cg(0.0, screen_height), 1080.0);
        assert_eq!(windows_to_cg(1080.0, screen_height), 0.0);
        assert_eq!(windows_to_cg(540.0, screen_height), 540.0);

        let original = 100.0;
        let converted =
            cg_to_windows(windows_to_cg(original, screen_height), screen_height);
        assert!((converted - original).abs() < 0.001);
    }
}
