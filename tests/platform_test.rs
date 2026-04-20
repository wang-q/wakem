// Platform Windows 测试 - 窗口管理器纯逻辑和类型测试

use wakem::platform::windows::{MonitorInfo, WindowFrame};
use wakem::types::{Alignment, Edge};

// ==================== WindowFrame 测试 ====================

/// 测试 WindowFrame 创建
#[test]
fn test_window_frame_new() {
    let frame = WindowFrame::new(100, 200, 800, 600);

    assert_eq!(frame.x, 100);
    assert_eq!(frame.y, 200);
    assert_eq!(frame.width, 800);
    assert_eq!(frame.height, 600);
}

/// 测试 WindowFrame 从 RECT 创建
#[test]
fn test_window_frame_from_rect() {
    use windows::Win32::Foundation::RECT;

    let rect = RECT {
        left: 50,
        top: 60,
        right: 850,
        bottom: 660,
    };

    let frame = WindowFrame::from_rect(&rect);

    assert_eq!(frame.x, 50);
    assert_eq!(frame.y, 60);
    assert_eq!(frame.width, 800); // right - left
    assert_eq!(frame.height, 600); // bottom - top
}

/// 测试 WindowFrame 边界值
#[test]
fn test_window_frame_boundary_values() {
    // 零尺寸
    let frame = WindowFrame::new(0, 0, 0, 0);
    assert_eq!(frame.x, 0);
    assert_eq!(frame.width, 0);

    // 负坐标（可能出现在多显示器场景）
    let frame = WindowFrame::new(-100, -200, 800, 600);
    assert_eq!(frame.x, -100);
    assert_eq!(frame.y, -200);

    // 大尺寸（4K 显示器）
    let frame = WindowFrame::new(0, 0, 3840, 2160);
    assert_eq!(frame.width, 3840);
    assert_eq!(frame.height, 2160);
}

/// 测试 WindowFrame Clone 和 Copy
#[test]
fn test_window_frame_clone_and_copy() {
    let frame1 = WindowFrame::new(100, 200, 800, 600);

    // Copy
    let frame2 = frame1;
    assert_eq!(frame2.x, 100);
    assert_eq!(frame2.width, 800);

    // Clone
    let frame3 = frame1.clone();
    assert_eq!(frame3.x, 100);
    assert_eq!(frame3.height, 600);
}

// ==================== Edge 枚举测试 ====================

/// 测试 Edge 变体
#[test]
fn test_edge_variants() {
    let edges = [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom];

    for edge in &edges {
        match edge {
            Edge::Left => {}
            Edge::Right => {}
            Edge::Top => {}
            Edge::Bottom => {}
        }
    }

    assert_eq!(edges.len(), 4);
}

/// 测试 Edge 匹配逻辑
#[test]
fn test_edge_matching() {
    fn is_horizontal(edge: &Edge) -> bool {
        matches!(edge, Edge::Left | Edge::Right)
    }

    fn is_vertical(edge: &Edge) -> bool {
        matches!(edge, Edge::Top | Edge::Bottom)
    }

    assert!(is_horizontal(&Edge::Left));
    assert!(is_horizontal(&Edge::Right));
    assert!(!is_horizontal(&Edge::Top));
    assert!(!is_horizontal(&Edge::Bottom));

    assert!(is_vertical(&Edge::Top));
    assert!(is_vertical(&Edge::Bottom));
    assert!(!is_vertical(&Edge::Left));
    assert!(!is_vertical(&Edge::Right));
}

// ==================== Alignment 枚举测试 ====================

/// 测试 Alignment 变体
#[test]
fn test_alignment_variants() {
    let alignments = [
        Alignment::Left,
        Alignment::Right,
        Alignment::Top,
        Alignment::Bottom,
        Alignment::Center,
    ];

    for align in &alignments {
        match align {
            Alignment::Left => {}
            Alignment::Right => {}
            Alignment::Top => {}
            Alignment::Bottom => {}
            Alignment::Center => {}
        }
    }

    assert_eq!(alignments.len(), 5);
}

/// 测试 Alignment 分类
#[test]
fn test_alignment_classification() {
    fn is_edge_alignment(align: &Alignment) -> bool {
        matches!(
            align,
            Alignment::Left | Alignment::Right | Alignment::Top | Alignment::Bottom
        )
    }

    fn is_center_alignment(align: &Alignment) -> bool {
        matches!(align, Alignment::Center)
    }

    assert!(is_edge_alignment(&Alignment::Left));
    assert!(is_edge_alignment(&Alignment::Right));
    assert!(is_edge_alignment(&Alignment::Top));
    assert!(is_edge_alignment(&Alignment::Bottom));
    assert!(!is_edge_alignment(&Alignment::Center));

    assert!(is_center_alignment(&Alignment::Center));
    assert!(!is_center_alignment(&Alignment::Left));
}

// ==================== MonitorInfo 测试 ====================

/// 测试 MonitorInfo 创建
#[test]
fn test_monitor_info_creation() {
    let monitor = MonitorInfo {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    assert_eq!(monitor.x, 0);
    assert_eq!(monitor.y, 0);
    assert_eq!(monitor.width, 1920);
    assert_eq!(monitor.height, 1080);
}

/// 测试多显示器配置的 MonitorInfo
#[test]
fn test_monitor_info_multi_monitor() {
    // 主显示器（左侧）
    let primary = MonitorInfo {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    // 副显示器（右侧）
    let secondary = MonitorInfo {
        x: 1920,
        y: 0,
        width: 1920,
        height: 1080,
    };

    // 验证副显示器的位置在主显示器右边
    assert_eq!(secondary.x, primary.x + primary.width);
    assert_eq!(secondary.y, primary.y);

    // 垂直堆叠的显示器
    let top_monitor = MonitorInfo {
        x: 0,
        y: -1080,
        width: 1920,
        height: 1080,
    };

    assert_eq!(top_monitor.y, -primary.height);
}

/// 测试不同分辨率的显示器
#[test]
fn test_monitor_info_different_resolutions() {
    let resolutions = vec![
        (1920, 1080), // Full HD
        (2560, 1440), // QHD
        (3840, 2160), // 4K UHD
        (1366, 768),  // 常见笔记本分辨率
        (1280, 720),  // HD
        (2560, 1600), // MacBook Pro
    ];

    for (w, h) in resolutions {
        let monitor = MonitorInfo {
            x: 0,
            y: 0,
            width: w,
            height: h,
        };
        assert_eq!(monitor.width, w);
        assert_eq!(monitor.height, h);
    }
}

// ==================== 窗口位置计算辅助函数 ====================

/// 计算居中位置的辅助函数（用于验证算法正确性）
#[test]
fn test_center_calculation_formula() {
    // 居中公式：center_pos = work_area_start + (work_area_size - window_size) / 2

    // 场景 1：1920x1080 工作区，800x600 窗口
    let work_width = 1920i32;
    let work_height = 1080i32;
    let win_width = 800i32;
    let win_height = 600i32;

    let center_x = (work_width - win_width) / 2; // (1920-800)/2 = 560
    let center_y = (work_height - win_height) / 2; // (1080-600)/2 = 240

    assert_eq!(center_x, 560);
    assert_eq!(center_y, 240);

    // 场景 2：窗口大于工作区
    let win_width = 2560i32;
    let win_height = 1440i32;

    let center_x = (work_width - win_width) / 2; // (1920-2560)/2 = -320
    let center_y = (work_height - win_height) / 2; // (1080-1440)/2 = -180

    assert_eq!(center_x, -320);
    assert_eq!(center_y, -180);

    // 场景 3：窗口等于工作区大小
    let win_width = 1920i32;
    let win_height = 1080i32;

    let center_x = (work_width - win_width) / 2; // (1920-1920)/2 = 0
    let center_y = (work_height - win_height) / 2; // (1080-1080)/2 = 0

    assert_eq!(center_x, 0);
    assert_eq!(center_y, 0);
}

/// 测试半屏计算公式
#[test]
fn test_half_screen_calculation_formula() {
    // 半宽公式：half_width = work_width / 2

    // 偶数宽度
    let work_width = 1920i32;
    let half_width = work_width / 2;
    assert_eq!(half_width, 960);

    // 左半屏起始 x = work_x
    // 右半屏起始 x = work_x + work_width - half_width
    let left_x = 0;
    let right_x = work_width - half_width; // 1920 - 960 = 960
    assert_eq!(right_x, 960);

    // 奇数宽度
    let work_width = 1921i32;
    let half_width = work_width / 2; // 整数除法向下取整
    assert_eq!(half_width, 960); // 1921/2 = 960 (向下取整)
}

/// 测试边缘对齐计算
#[test]
fn test_edge_alignment_formula() {
    let work_area = MonitorInfo {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    let window = WindowFrame::new(500, 300, 800, 600);

    // 左边缘：x = work_area.x
    let left_x = work_area.x;
    assert_eq!(left_x, 0);

    // 右边缘：x = work_area.x + work_area.width - window.width
    let right_x = work_area.x + work_area.width - window.width; // 0 + 1920 - 800 = 1120
    assert_eq!(right_x, 1120);

    // 上边缘：y = work_area.y
    let top_y = work_area.y;
    assert_eq!(top_y, 0);

    // 下边缘：y = work_area.y + work_area.height - window.height
    let bottom_y = work_area.y + work_area.height - window.height; // 0 + 1080 - 600 = 480
    assert_eq!(bottom_y, 480);
}
