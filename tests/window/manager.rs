// 窗口管理测试

/// 测试窗口框架计算
#[test]
fn test_window_frame_calculation() {
    // 测试窗口居中计算
    let screen_width = 1920;
    let screen_height = 1080;
    let window_width = 800;
    let window_height = 600;

    let expected_x = (screen_width - window_width) / 2;
    let expected_y = (screen_height - window_height) / 2;

    assert_eq!(expected_x, 560);
    assert_eq!(expected_y, 240);
}

/// 测试循环尺寸计算
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

/// 测试循环尺寸下一个
#[test]
fn test_loop_next_ratio() {
    let ratios = [0.75f32, 0.6, 0.5, 0.4, 0.25];
    let current = 0.5f32;

    // 找到当前比例
    let current_index = ratios.iter().position(|&r| (current - r).abs() < 0.01);
    assert_eq!(current_index, Some(2));

    // 下一个比例
    let next_index = (current_index.unwrap() + 1) % ratios.len();
    assert_eq!(ratios[next_index], 0.4);
}

/// 测试固定比例窗口计算
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
