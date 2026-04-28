// Window calculation tests
// Test window position and size calculation logic

use wakem::types::*;

/// Calculate center position of window
fn calculate_center(
    window_width: i32,
    window_height: i32,
    screen_width: i32,
    screen_height: i32,
) -> (i32, i32) {
    let x = (screen_width - window_width) / 2;
    let y = (screen_height - window_height) / 2;
    (x, y)
}

/// Calculate half-screen window position and size
fn calculate_half_screen(
    edge: Edge,
    screen_width: i32,
    screen_height: i32,
) -> (i32, i32, i32, i32) {
    match edge {
        Edge::Left => (0, 0, screen_width / 2, screen_height),
        Edge::Right => (screen_width / 2, 0, screen_width / 2, screen_height),
        Edge::Top => (0, 0, screen_width, screen_height / 2),
        Edge::Bottom => (0, screen_height / 2, screen_width, screen_height / 2),
    }
}

/// Calculate loop width
fn calculate_loop_width(
    current_width: i32,
    screen_width: i32,
    alignment: Alignment,
) -> i32 {
    let ratios = [0.75_f32, 0.6, 0.5, 0.4, 0.25];

    // Find current closest ratio
    let current_ratio = current_width as f32 / screen_width as f32;
    let mut closest_idx = 0;
    let mut min_diff = f32::MAX;

    for (i, &ratio) in ratios.iter().enumerate() {
        let diff = (current_ratio - ratio).abs();
        if diff < min_diff {
            min_diff = diff;
            closest_idx = i;
        }
    }

    // Select next ratio based on direction
    let next_idx = match alignment {
        Alignment::Left => {
            if closest_idx < ratios.len() - 1 {
                closest_idx + 1
            } else {
                0
            }
        }
        Alignment::Right => {
            if closest_idx > 0 {
                closest_idx - 1
            } else {
                ratios.len() - 1
            }
        }
        _ => closest_idx,
    };

    (screen_width as f32 * ratios[next_idx]) as i32
}

/// Test center calculation - standard screen
#[test]
fn test_center_standard_screen() {
    let (x, y) = calculate_center(800, 600, 1920, 1080);

    assert_eq!(x, 560); // (1920 - 800) / 2
    assert_eq!(y, 240); // (1080 - 600) / 2
}

/// Test center calculation - small window
#[test]
fn test_center_small_window() {
    let (x, y) = calculate_center(400, 300, 1920, 1080);

    assert_eq!(x, 760); // (1920 - 400) / 2
    assert_eq!(y, 390); // (1080 - 300) / 2
}

/// Test center calculation - fullscreen window
#[test]
fn test_center_fullscreen() {
    let (x, y) = calculate_center(1920, 1080, 1920, 1080);

    assert_eq!(x, 0);
    assert_eq!(y, 0);
}

/// Test half-screen calculation - left half
#[test]
fn test_half_screen_left() {
    let (x, y, w, h) = calculate_half_screen(Edge::Left, 1920, 1080);

    assert_eq!(x, 0);
    assert_eq!(y, 0);
    assert_eq!(w, 960); // 1920 / 2
    assert_eq!(h, 1080);
}

/// Test half-screen calculation - right half
#[test]
fn test_half_screen_right() {
    let (x, y, w, h) = calculate_half_screen(Edge::Right, 1920, 1080);

    assert_eq!(x, 960); // 1920 / 2
    assert_eq!(y, 0);
    assert_eq!(w, 960); // 1920 / 2
    assert_eq!(h, 1080);
}

/// Test half-screen calculation - top half
#[test]
fn test_half_screen_top() {
    let (x, _y, w, h) = calculate_half_screen(Edge::Top, 1920, 1080);

    assert_eq!(x, 0);
    assert_eq!(w, 1920);
    assert_eq!(h, 540); // 1080 / 2
}

/// Test half-screen calculation - bottom half
#[test]
fn test_half_screen_bottom() {
    let (_x, y, w, h) = calculate_half_screen(Edge::Bottom, 1920, 1080);

    assert_eq!(y, 540); // 1080 / 2
    assert_eq!(w, 1920);
    assert_eq!(h, 540); // 1080 / 2
}

/// Test half-screen calculation - different resolutions
#[test]
fn test_half_screen_different_resolution() {
    // 4K 屏幕
    let (_x, _y, w, h) = calculate_half_screen(Edge::Left, 3840, 2160);
    assert_eq!(w, 1920); // 3840 / 2
    assert_eq!(h, 2160);

    // 1440p 屏幕
    let (x, _y, w, _h) = calculate_half_screen(Edge::Right, 2560, 1440);
    assert_eq!(x, 1280); // 2560 / 2
    assert_eq!(w, 1280); // 2560 / 2
}

/// Test loop width - from left
#[test]
fn test_loop_width_from_left() {
    // 从 75% 开始，向左应该变为 60%
    let width = calculate_loop_width(1440, 1920, Alignment::Left);
    assert_eq!(width, 1152); // 1920 * 0.6
}

/// Test loop width - from right
#[test]
fn test_loop_width_from_right() {
    // 从 25% 开始，向右应该变为 40%
    let width = calculate_loop_width(480, 1920, Alignment::Right);
    assert_eq!(width, 768); // 1920 * 0.4
}

/// Test loop width - wrap boundary (left)
#[test]
fn test_loop_width_wrap_left() {
    // 从 25%（最小）开始，向左应该循环到 75%（最大）
    let width = calculate_loop_width(480, 1920, Alignment::Left);
    assert_eq!(width, 1440); // 1920 * 0.75
}

/// Test loop width - wrap boundary (right)
#[test]
fn test_loop_width_wrap_right() {
    // 从 75%（最大）开始，向右应该循环到 25%（最小）
    let width = calculate_loop_width(1440, 1920, Alignment::Right);
    assert_eq!(width, 480); // 1920 * 0.25
}

/// Test loop width - all ratios
#[test]
fn test_loop_width_all_ratios() {
    let screen_width = 1920;
    let expected_widths = [
        1440, // 75%
        1152, // 60%
        960,  // 50%
        768,  // 40%
        480,  // 25%
    ];

    // Test looping through all ratios from left
    let mut current_width = 1440;
    for expected in &expected_widths[1..] {
        current_width =
            calculate_loop_width(current_width, screen_width, Alignment::Left);
        assert_eq!(current_width, *expected);
    }

    // Last should wrap back to first
    current_width = calculate_loop_width(current_width, screen_width, Alignment::Left);
    assert_eq!(current_width, expected_widths[0]);
}

/// Test loop width - non-standard width (find closest)
#[test]
fn test_loop_width_find_closest() {
    // 1400 最接近 1440 (75%)
    let width = calculate_loop_width(1400, 1920, Alignment::Left);
    assert_eq!(width, 1152); // 应该变为 60%

    // 500 最接近 480 (25%)
    let width = calculate_loop_width(500, 1920, Alignment::Right);
    assert_eq!(width, 768); // 应该变为 40%
}

/// Test loop width - different screen sizes
#[test]
fn test_loop_width_different_screens() {
    // 4K 屏幕
    let width = calculate_loop_width(2880, 3840, Alignment::Left);
    assert_eq!(width, 2304); // 3840 * 0.6

    // 小屏幕
    let width = calculate_loop_width(600, 800, Alignment::Left);
    assert_eq!(width, 480); // 800 * 0.6
}

/// Test center calculation - odd dimensions
#[test]
fn test_center_odd_dimensions() {
    let (x, y) = calculate_center(801, 601, 1920, 1080);

    // Integer division truncates down
    assert_eq!(x, 559); // (1920 - 801) / 2 = 559.5 -> 559
    assert_eq!(y, 239); // (1080 - 601) / 2 = 239.5 -> 239
}

/// Test half-screen calculation - odd screen width
#[test]
fn test_half_screen_odd_width() {
    let (_x, _y, w, _h) = calculate_half_screen(Edge::Left, 1921, 1080);

    assert_eq!(w, 960); // 1921 / 2 = 960.5 -> 960

    let (x, _y, w, _h) = calculate_half_screen(Edge::Right, 1921, 1080);

    assert_eq!(x, 960); // 1921 / 2 = 960.5 -> 960
    assert_eq!(w, 960); // 1921 / 2 = 960.5 -> 960
}

// ==================== Window manager tests (from ut_window_manager.rs)====================

/// Test window frame calculation
#[test]
fn test_window_frame_calculation() {
    // Test window center calculation
    let screen_width = 1920;
    let screen_height = 1080;
    let window_width = 800;
    let window_height = 600;

    let expected_x = (screen_width - window_width) / 2;
    let expected_y = (screen_height - window_height) / 2;

    assert_eq!(expected_x, 560);
    assert_eq!(expected_y, 240);
}

/// Test loop size calculation
#[test]
fn test_loop_width_calculation() {
    let screen_width = 1920;
    let ratios = [0.75, 0.6, 0.5, 0.4, 0.25];

    let expected_widths: Vec<i32> = ratios
        .iter()
        .map(|r| (screen_width as f32 * r) as i32)
        .collect();

    assert_eq!(expected_widths[0], 1440); // 75%
    assert_eq!(expected_widths[1], 1152); // 60%
    assert_eq!(expected_widths[2], 960); // 50%
    assert_eq!(expected_widths[3], 768); // 40%
    assert_eq!(expected_widths[4], 480); // 25%
}

/// Test loop size next
#[test]
fn test_loop_next_ratio() {
    let ratios = [0.75f32, 0.6, 0.5, 0.4, 0.25];
    let current = 0.5f32;

    // Find current ratio
    let current_index = ratios.iter().position(|&r| (current - r).abs() < 0.01);
    assert_eq!(current_index, Some(2));

    // Next ratio
    let next_index = (current_index.unwrap() + 1) % ratios.len();
    assert_eq!(ratios[next_index], 0.4);
}

/// Test fixed ratio window calculation
#[test]
fn test_fixed_ratio_calculation() {
    let ratio = 4.0 / 3.0; // 4:3
    let base_size = 1080; // 屏幕较小边
    let scale = 1.0; // 100%

    let base_width = (base_size as f32 * ratio) as i32;
    let base_height = base_size;

    let width = (base_width as f32 * scale) as i32;
    let height = (base_height as f32 * scale) as i32;

    assert_eq!(width, 1440);
    assert_eq!(height, 1080);
}
