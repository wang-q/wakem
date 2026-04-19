// 窗口计算测试
// 测试窗口位置和大小的计算逻辑

use wakem_common::types::*;

/// 计算居中窗口的位置
fn calculate_center(window_width: i32, window_height: i32, screen_width: i32, screen_height: i32) -> (i32, i32) {
    let x = (screen_width - window_width) / 2;
    let y = (screen_height - window_height) / 2;
    (x, y)
}

/// 计算半屏窗口的位置和大小
fn calculate_half_screen(edge: Edge, screen_width: i32, screen_height: i32) -> (i32, i32, i32, i32) {
    match edge {
        Edge::Left => (0, 0, screen_width / 2, screen_height),
        Edge::Right => (screen_width / 2, 0, screen_width / 2, screen_height),
        Edge::Top => (0, 0, screen_width, screen_height / 2),
        Edge::Bottom => (0, screen_height / 2, screen_width, screen_height / 2),
    }
}

/// 计算循环宽度
fn calculate_loop_width(current_width: i32, screen_width: i32, alignment: Alignment) -> i32 {
    let ratios = [0.75_f32, 0.6, 0.5, 0.4, 0.25];
    
    // 找到当前最接近的比例
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
    
    // 根据方向选择下一个比例
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

/// 测试居中计算 - 标准屏幕
#[test]
fn test_center_standard_screen() {
    let (x, y) = calculate_center(800, 600, 1920, 1080);
    
    assert_eq!(x, 560); // (1920 - 800) / 2
    assert_eq!(y, 240); // (1080 - 600) / 2
}

/// 测试居中计算 - 小窗口
#[test]
fn test_center_small_window() {
    let (x, y) = calculate_center(400, 300, 1920, 1080);
    
    assert_eq!(x, 760); // (1920 - 400) / 2
    assert_eq!(y, 390); // (1080 - 300) / 2
}

/// 测试居中计算 - 全屏窗口
#[test]
fn test_center_fullscreen() {
    let (x, y) = calculate_center(1920, 1080, 1920, 1080);
    
    assert_eq!(x, 0);
    assert_eq!(y, 0);
}

/// 测试半屏计算 - 左半屏
#[test]
fn test_half_screen_left() {
    let (x, y, w, h) = calculate_half_screen(Edge::Left, 1920, 1080);
    
    assert_eq!(x, 0);
    assert_eq!(y, 0);
    assert_eq!(w, 960);  // 1920 / 2
    assert_eq!(h, 1080);
}

/// 测试半屏计算 - 右半屏
#[test]
fn test_half_screen_right() {
    let (x, y, w, h) = calculate_half_screen(Edge::Right, 1920, 1080);
    
    assert_eq!(x, 960);  // 1920 / 2
    assert_eq!(y, 0);
    assert_eq!(w, 960);  // 1920 / 2
    assert_eq!(h, 1080);
}

/// 测试半屏计算 - 上半屏
#[test]
fn test_half_screen_top() {
    let (x, y, w, h) = calculate_half_screen(Edge::Top, 1920, 1080);
    
    assert_eq!(x, 0);
    assert_eq!(y, 0);
    assert_eq!(w, 1920);
    assert_eq!(h, 540);  // 1080 / 2
}

/// 测试半屏计算 - 下半屏
#[test]
fn test_half_screen_bottom() {
    let (x, y, w, h) = calculate_half_screen(Edge::Bottom, 1920, 1080);
    
    assert_eq!(x, 0);
    assert_eq!(y, 540);  // 1080 / 2
    assert_eq!(w, 1920);
    assert_eq!(h, 540);  // 1080 / 2
}

/// 测试半屏计算 - 不同分辨率
#[test]
fn test_half_screen_different_resolution() {
    // 4K 屏幕
    let (x, y, w, h) = calculate_half_screen(Edge::Left, 3840, 2160);
    assert_eq!(w, 1920);  // 3840 / 2
    assert_eq!(h, 2160);
    
    // 1440p 屏幕
    let (x, y, w, h) = calculate_half_screen(Edge::Right, 2560, 1440);
    assert_eq!(x, 1280);  // 2560 / 2
    assert_eq!(w, 1280);  // 2560 / 2
}

/// 测试循环宽度 - 从左开始
#[test]
fn test_loop_width_from_left() {
    // 从 75% 开始，向左应该变为 60%
    let width = calculate_loop_width(1440, 1920, Alignment::Left);
    assert_eq!(width, 1152); // 1920 * 0.6
}

/// 测试循环宽度 - 从右开始
#[test]
fn test_loop_width_from_right() {
    // 从 25% 开始，向右应该变为 40%
    let width = calculate_loop_width(480, 1920, Alignment::Right);
    assert_eq!(width, 768); // 1920 * 0.4
}

/// 测试循环宽度 - 循环边界（左）
#[test]
fn test_loop_width_wrap_left() {
    // 从 25%（最小）开始，向左应该循环到 75%（最大）
    let width = calculate_loop_width(480, 1920, Alignment::Left);
    assert_eq!(width, 1440); // 1920 * 0.75
}

/// 测试循环宽度 - 循环边界（右）
#[test]
fn test_loop_width_wrap_right() {
    // 从 75%（最大）开始，向右应该循环到 25%（最小）
    let width = calculate_loop_width(1440, 1920, Alignment::Right);
    assert_eq!(width, 480); // 1920 * 0.25
}

/// 测试循环宽度 - 所有比例
#[test]
fn test_loop_width_all_ratios() {
    let screen_width = 1920;
    let expected_widths = vec![
        1440, // 75%
        1152, // 60%
        960,  // 50%
        768,  // 40%
        480,  // 25%
    ];
    
    // 测试从左循环经过所有比例
    let mut current_width = 1440;
    for expected in &expected_widths[1..] {
        current_width = calculate_loop_width(current_width, screen_width, Alignment::Left);
        assert_eq!(current_width, *expected);
    }
    
    // 最后一个应该循环回第一个
    current_width = calculate_loop_width(current_width, screen_width, Alignment::Left);
    assert_eq!(current_width, expected_widths[0]);
}

/// 测试循环宽度 - 非标准宽度（找到最接近的）
#[test]
fn test_loop_width_find_closest() {
    // 1400 最接近 1440 (75%)
    let width = calculate_loop_width(1400, 1920, Alignment::Left);
    assert_eq!(width, 1152); // 应该变为 60%
    
    // 500 最接近 480 (25%)
    let width = calculate_loop_width(500, 1920, Alignment::Right);
    assert_eq!(width, 768); // 应该变为 40%
}

/// 测试循环宽度 - 不同屏幕尺寸
#[test]
fn test_loop_width_different_screens() {
    // 4K 屏幕
    let width = calculate_loop_width(2880, 3840, Alignment::Left);
    assert_eq!(width, 2304); // 3840 * 0.6
    
    // 小屏幕
    let width = calculate_loop_width(600, 800, Alignment::Left);
    assert_eq!(width, 480); // 800 * 0.6
}

/// 测试居中计算 - 奇数尺寸
#[test]
fn test_center_odd_dimensions() {
    let (x, y) = calculate_center(801, 601, 1920, 1080);
    
    // 整数除法会向下取整
    assert_eq!(x, 559); // (1920 - 801) / 2 = 559.5 -> 559
    assert_eq!(y, 239); // (1080 - 601) / 2 = 239.5 -> 239
}

/// 测试半屏计算 - 奇数屏幕宽度
#[test]
fn test_half_screen_odd_width() {
    let (x, y, w, h) = calculate_half_screen(Edge::Left, 1921, 1080);
    
    assert_eq!(w, 960); // 1921 / 2 = 960.5 -> 960
    
    let (x, y, w, h) = calculate_half_screen(Edge::Right, 1921, 1080);
    
    assert_eq!(x, 960);  // 1921 / 2 = 960.5 -> 960
    assert_eq!(w, 960);  // 1921 / 2 = 960.5 -> 960
}
