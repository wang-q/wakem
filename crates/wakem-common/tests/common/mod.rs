// 测试辅助工具模块
// 提供测试用的辅助函数

use wakem_common::types::*;

/// 创建测试用的 KeyEvent
pub fn create_key_event(scan_code: u16, virtual_key: u16, state: KeyState) -> KeyEvent {
    KeyEvent {
        scan_code,
        virtual_key,
        state,
        modifiers: ModifierState::default(),
        timestamp: 0,
    }
}

/// 创建测试用的 InputEvent::Key
pub fn create_input_key_event(scan_code: u16, virtual_key: u16, state: KeyState) -> InputEvent {
    InputEvent::Key(create_key_event(scan_code, virtual_key, state))
}

/// 创建带修饰键的 KeyEvent
pub fn create_key_event_with_modifiers(
    scan_code: u16,
    virtual_key: u16,
    state: KeyState,
    modifiers: ModifierState,
) -> KeyEvent {
    KeyEvent {
        scan_code,
        virtual_key,
        state,
        modifiers,
        timestamp: 0,
    }
}

/// 创建测试用的 MouseEvent
pub fn create_mouse_event(x: i32, y: i32, button_state: u32) -> MouseEvent {
    MouseEvent {
        x,
        y,
        button_state,
        wheel_delta: 0,
        timestamp: 0,
    }
}

/// 创建测试用的 ContextInfo
pub fn create_context_info(
    window_class: &str,
    process_name: &str,
    window_title: &str,
) -> ContextInfo {
    ContextInfo {
        window_class: window_class.to_string(),
        process_name: process_name.to_string(),
        process_path: format!("C:\\Program Files\\{}", process_name),
        window_title: window_title.to_string(),
        window_handle: 0x123456,
    }
}

/// 创建简单的按键映射规则
pub fn create_key_mapping_rule(
    scan_code: u16,
    virtual_key: u16,
    action: Action,
) -> MappingRule {
    let trigger = Trigger::key(scan_code, virtual_key);
    MappingRule::new(trigger, action)
}

/// 创建带修饰键的映射规则
pub fn create_key_mapping_with_modifiers(
    scan_code: u16,
    virtual_key: u16,
    modifiers: ModifierState,
    action: Action,
) -> MappingRule {
    let trigger = Trigger::key_with_modifiers(scan_code, virtual_key, modifiers);
    MappingRule::new(trigger, action)
}

/// 创建导航层（Vim 风格）
pub fn create_vim_navigation_layer() -> Layer {
    let mut layer = Layer::new("vim_navigation", 0x3A, 0x14); // CapsLock
    layer.add_mapping(
        Trigger::key(0x23, 0x48), // H
        Action::key(KeyAction::click(0x4B, 0x25)), // Left
    );
    layer.add_mapping(
        Trigger::key(0x24, 0x4A), // J
        Action::key(KeyAction::click(0x50, 0x28)), // Down
    );
    layer.add_mapping(
        Trigger::key(0x25, 0x4B), // K
        Action::key(KeyAction::click(0x48, 0x26)), // Up
    );
    layer.add_mapping(
        Trigger::key(0x26, 0x4C), // L
        Action::key(KeyAction::click(0x4D, 0x27)), // Right
    );
    layer
}

/// 创建窗口管理层
pub fn create_window_management_layer() -> Layer {
    let mut layer = Layer::new("window_management", 0x3B, 0x70); // F1
    layer.with_mode(LayerMode::Toggle);
    layer.add_mapping(
        Trigger::key(0x2E, 0x43), // C
        Action::window(WindowAction::Center),
    );
    layer.add_mapping(
        Trigger::key(0x10, 0x51), // Q
        Action::window(WindowAction::Close),
    );
    layer.add_mapping(
        Trigger::key(0x32, 0x4D), // M
        Action::window(WindowAction::Maximize),
    );
    layer.add_mapping(
        Trigger::key(0x31, 0x4E), // N
        Action::window(WindowAction::Minimize),
    );
    layer
}

/// 窗口位置计算辅助函数
pub mod window_calc {
    use wakem_common::types::*;

    /// 计算居中窗口的位置
    pub fn calculate_center(window_width: i32, window_height: i32, screen_width: i32, screen_height: i32) -> (i32, i32) {
        let x = (screen_width - window_width) / 2;
        let y = (screen_height - window_height) / 2;
        (x, y)
    }

    /// 计算半屏窗口的位置和大小
    pub fn calculate_half_screen(edge: Edge, screen_width: i32, screen_height: i32) -> (i32, i32, i32, i32) {
        match edge {
            Edge::Left => (0, 0, screen_width / 2, screen_height),
            Edge::Right => (screen_width / 2, 0, screen_width / 2, screen_height),
            Edge::Top => (0, 0, screen_width, screen_height / 2),
            Edge::Bottom => (0, screen_height / 2, screen_width, screen_height / 2),
        }
    }

    /// 计算循环宽度
    pub fn calculate_loop_width(current_width: i32, screen_width: i32, alignment: Alignment) -> i32 {
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
}
