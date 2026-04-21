//! Coordinate system conversion utilities.
//!
//! macOS uses bottom-left origin (Core Graphics).
//! Windows/wakem uses top-left origin.
//! This module provides conversion functions.

/// Convert Y-coordinate from Windows-style (top-left) to CG-style (bottom-left)
pub fn windows_to_cg(y: f64, screen_height: f64) -> f64 {
    screen_height - y
}

/// Convert Y-coordinate from CG-style (bottom-left) to Windows-style (top-left)
pub fn cg_to_windows(y: f64, screen_height: f64) -> f64 {
    screen_height - y
}

/// Convert a CGRect from CG to Windows coordinates
#[allow(dead_code)]
pub fn cg_rect_to_windows(
    rect: core_graphics::geometry::CGRect,
    screen_height: f64,
) -> core_graphics::geometry::CGRect {
    core_graphics::geometry::CGRect {
        origin: core_graphics::geometry::CGPoint {
            x: rect.origin.x,
            y: cg_to_windows(rect.origin.y + rect.size.height, screen_height),
        },
        size: rect.size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_conversion() {
        let screen_height = 1080.0;

        // Top of screen (y=0 in Windows) should be bottom in CG
        assert_eq!(windows_to_cg(0.0, screen_height), 1080.0);

        // Bottom of screen (y=1080 in Windows) should be top in CG
        assert_eq!(windows_to_cg(1080.0, screen_height), 0.0);

        // Middle should stay middle
        assert_eq!(windows_to_cg(540.0, screen_height), 540.0);

        // Roundtrip should preserve value
        let original = 100.0;
        let converted =
            cg_to_windows(windows_to_cg(original, screen_height), screen_height);
        assert!((converted - original).abs() < 0.001);
    }

    #[test]
    fn test_cg_rect_conversion() {
        let screen_height = 1080.0;
        let cg_rect = core_graphics::geometry::CGRect {
            origin: core_graphics::geometry::CGPoint { x: 100.0, y: 0.0 },
            size: core_graphics::geometry::CGSize {
                width: 800.0,
                height: 600.0,
            },
        };

        let win_rect = cg_rect_to_windows(cg_rect, screen_height);
        assert_eq!(win_rect.origin.x, 100.0);
        assert_eq!(win_rect.origin.y, 480.0); // 1080 - (0 + 600) = 480
        assert_eq!(win_rect.size.width, 800.0);
        assert_eq!(win_rect.size.height, 600.0);
    }
}
