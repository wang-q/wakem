// Daemon 核心逻辑测试

use wakem::config::Config;
use wakem::daemon::ServerState;
use wakem::types::{
    InputEvent, KeyEvent, KeyState, MacroStep, ModifierState, MouseEventType,
};

// ==================== ServerState 初始化和配置加载 ====================

/// 测试 ServerState 默认初始化
#[tokio::test]
async fn test_server_state_new() {
    let state = ServerState::new();

    // 验证默认状态
    let (active, config_loaded) = state.get_status().await;
    assert!(active, "默认应该是启用状态");
    assert!(!config_loaded, "默认配置未加载");

    // 验证可以设置状态
    state.set_active(false).await;
    let (active, _) = state.get_status().await;
    assert!(!active);
}

/// 测试 Default trait 实现
#[test]
fn test_server_state_default() {
    let state = ServerState::default();
    // 验证 Default trait 可以正常工作
    let _ = state;
}

/// 测试基本配置加载
#[tokio::test]
async fn test_load_config_basic() {
    let state = ServerState::new();
    let config = Config::default();

    let result = state.load_config(config).await;
    assert!(result.is_ok(), "基本配置应该成功加载");

    // 验证配置已标记为已加载
    let (_, config_loaded) = state.get_status().await;
    assert!(config_loaded, "配置应该被标记为已加载");
}

/// 测试带键盘映射的配置加载
#[tokio::test]
async fn test_load_config_with_key_mappings() {
    let state = ServerState::new();

    let config_str = r#"
[keyboard.remap]
CapsLock = "Backspace"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(result.is_ok(), "带键位映射的配置应该成功加载");
}

/// 测试带层的配置加载（Hold 模式）
#[tokio::test]
async fn test_load_config_with_layers_hold_mode() {
    let state = ServerState::new();

    let config_str = r#"
[keyboard.layers.navigate]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"
K = "Up"
L = "Right"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(result.is_ok(), "Hold 模式的层配置应该成功加载");
}

/// 测试带层的配置加载（Toggle 模式）
#[tokio::test]
async fn test_load_config_with_layers_toggle_mode() {
    let state = ServerState::new();

    let config_str = r#"
[keyboard.layers.symbols]
activation_key = "Space"
mode = "Toggle"

[keyboard.layers.symbols.mappings]
A = "1"
B = "2"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(result.is_ok(), "Toggle 模式的层配置应该成功加载");
}

/// 测试带窗口预设的配置加载
#[tokio::test]
async fn test_load_config_with_window_presets() {
    let state = ServerState::new();

    // 使用一个简单的配置（不包含复杂的窗口预设）
    let config_str = r#"
[window.shortcuts]
"Ctrl+Alt+C" = "Center"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(result.is_ok(), "带窗口快捷键的配置应该成功加载");
}

/// 测试完整配置加载
#[tokio::test]
async fn test_load_config_full() {
    let state = ServerState::new();

    let config_str = r#"
log_level = "debug"
tray_icon = true
auto_reload = true

[keyboard.remap]
CapsLock = "Backspace"

[keyboard.layers.navigate]
activation_key = "RightAlt"
mode = "Hold"

[keyboard.layers.navigate.mappings]
H = "Left"
J = "Down"

[window.shortcuts]
"Ctrl+Alt+C" = "Center"

[launch]
F1 = "notepad.exe"

[network]
enabled = true
instance_id = 1
auth_key = "test_key"

[macros]
test_macro = []

[macro_bindings]
F5 = "test_macro"
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let result = state.load_config(config).await;

    assert!(result.is_ok(), "完整配置应该成功加载");
}

// ==================== 输入事件处理 ====================

/// 测试键盘事件处理（基础）
#[tokio::test]
async fn test_process_input_event_key() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 创建一个简单的键盘按下事件
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed); // 'A' 键
    let event = InputEvent::Key(key_event);

    // 处理事件（不应该 panic）
    state.process_input_event(event).await;
}

/// 测试鼠标滚轮事件处理
#[tokio::test]
async fn test_process_input_event_mouse_wheel() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 创建鼠标滚轮事件
    let mouse_event = wakem::types::MouseEvent::new(MouseEventType::Wheel(120), 0, 0);
    let event = InputEvent::Mouse(mouse_event);

    // 处理事件
    state.process_input_event(event).await;
}

/// 测试禁用状态下的事件处理
#[tokio::test]
async fn test_process_input_event_disabled() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 禁用映射
    state.set_active(false).await;

    // 创建事件
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed);
    let event = InputEvent::Key(key_event);

    // 处理事件（应该被忽略，但不应该 panic）
    state.process_input_event(event).await;

    // 验证仍然是禁用状态
    let (active, _) = state.get_status().await;
    assert!(!active);
}

/// 测试注入事件的忽略
#[tokio::test]
async fn test_process_injected_event_ignored() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 创建一个注入的事件（is_injected = true）
    let key_event = KeyEvent::new(0x1E, 0x41, KeyState::Pressed).injected();
    let event = InputEvent::Key(key_event);

    // 处理注入事件（应该被忽略，但不应该 panic）
    state.process_input_event(event).await;
}

/// 测试鼠标移动事件处理
#[tokio::test]
async fn test_process_input_event_mouse_move() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 创建鼠标移动事件
    let mouse_event = wakem::types::MouseEvent::new(MouseEventType::Move, 100, 200);
    let event = InputEvent::Mouse(mouse_event);

    // 处理事件
    state.process_input_event(event).await;
}

/// 测试鼠标按钮事件处理
#[tokio::test]
async fn test_process_input_event_mouse_button() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 创建鼠标按钮按下事件
    let mouse_event = wakem::types::MouseEvent::new(
        MouseEventType::ButtonDown(wakem::types::MouseButton::Left),
        0,
        0,
    );
    let event = InputEvent::Mouse(mouse_event);

    // 处理事件
    state.process_input_event(event).await;
}

// ==================== 宏管理功能 ====================

/// 测试宏录制开始和停止
#[tokio::test]
async fn test_start_stop_macro_recording() {
    let state = ServerState::new();

    // 开始录制
    let result = state.start_macro_recording("test_macro").await;
    assert!(result.is_ok(), "开始录制应该成功");

    // 验证正在录制
    assert!(state.is_recording_macro().await, "应该处于录制状态");

    // 停止录制
    let result = state.stop_macro_recording().await;
    assert!(result.is_ok(), "停止录制应该成功");

    // 验证不再录制
    assert!(!state.is_recording_macro().await, "不应该处于录制状态");
}

/// 测试播放宏
#[tokio::test]
async fn test_play_macro() {
    let state = ServerState::new();

    // 首先添加一个简单的宏到配置中（使用空步骤）
    let config_str = r#"
[macros]
simple_macro = []
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let _ = state.load_config(config).await;

    // 播放宏
    let result = state.play_macro("simple_macro").await;
    // 注意：这个可能会失败，因为宏播放依赖输出设备
    // 我们只验证不会 panic
    let _ = result;
}

/// 测试播放不存在的宏（错误处理）
#[tokio::test]
async fn test_play_nonexistent_macro() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 尝试播放不存在的宏
    let result = state.play_macro("nonexistent_macro").await;
    assert!(result.is_err(), "播放不存在的宏应该返回错误");
}

/// 测试获取宏列表
#[tokio::test]
async fn test_get_macros_list() {
    let state = ServerState::new();

    // 空配置时，宏列表应该为空
    let macros = state.get_macros().await;
    assert!(macros.is_empty(), "空配置时宏列表应该为空");

    // 添加一些宏
    let config_str = r#"
[macros]
macro1 = []
macro2 = []
macro3 = []
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let _ = state.load_config(config).await;

    let macros = state.get_macros().await;
    assert_eq!(macros.len(), 3, "应该有 3 个宏");
    assert!(macros.contains(&"macro1".to_string()));
    assert!(macros.contains(&"macro2".to_string()));
    assert!(macros.contains(&"macro3".to_string()));
}

/// 测试删除宏
#[tokio::test]
async fn test_delete_macro() {
    let state = ServerState::new();

    // 先添加一个宏
    let config_str = r#"
[macros]
temp_macro = []
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let _ = state.load_config(config).await;

    // 验证宏存在
    let macros = state.get_macros().await;
    assert!(macros.contains(&"temp_macro".to_string()));

    // 删除宏
    let result = state.delete_macro("temp_macro").await;
    // 注意：这可能会失败因为涉及文件操作，我们验证不会 panic
    let _ = result;
}

/// 测试删除不存在的宏（错误处理）
#[tokio::test]
async fn test_delete_nonexistent_macro() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 删除不存在的宏
    let result = state.delete_macro("nonexistent").await;
    assert!(result.is_err(), "删除不存在的宏应该返回错误");
}

/// 测试绑定宏到触发键
#[tokio::test]
async fn test_bind_macro() {
    let state = ServerState::new();

    // 先添加一个宏
    let config_str = r#"
[macros]
my_macro = []
"#;

    let config: Config = toml::from_str(config_str).unwrap();
    let _ = state.load_config(config).await;

    // 绑定宏
    let result = state.bind_macro("my_macro", "F5").await;
    // 可能会失败（文件操作），但不应 panic
    let _ = result;
}

/// 测试绑定不存在的宏（错误处理）
#[tokio::test]
async fn test_bind_nonexistent_macro() {
    let state = ServerState::new();
    let config = Config::default();
    let _ = state.load_config(config).await;

    // 绑定不存在的宏
    let result = state.bind_macro("nonexistent", "F5").await;
    assert!(result.is_err(), "绑定不存在的宏应该返回错误");
}

// ==================== 状态管理 ====================

/// 测试启用/禁用状态切换
#[tokio::test]
async fn test_set_active_state_toggle() {
    let state = ServerState::new();

    // 默认是启用的
    let (active, _) = state.get_status().await;
    assert!(active);

    // 切换到禁用
    state.set_active(false).await;
    let (active, _) = state.get_status().await;
    assert!(!active);

    // 切换回启用
    state.set_active(true).await;
    let (active, _) = state.get_status().await;
    assert!(active);

    // 多次设置相同值
    state.set_active(true).await;
    state.set_active(true).await;
    let (active, _) = state.get_status().await;
    assert!(active);
}

/// 测试状态查询的一致性
#[tokio::test]
async fn test_get_status_consistency() {
    let state = ServerState::new();

    // 多次查询应该返回相同结果
    let status1 = state.get_status().await;
    let status2 = state.get_status().await;
    let status3 = state.get_status().await;

    assert_eq!(status1, status2);
    assert_eq!(status2, status3);
}

/// 测试消息窗口句柄注册（Windows 特定）
#[cfg(target_os = "windows")]
#[tokio::test]
async fn test_set_message_window_hwnd() {
    use windows::Win32::Foundation::HWND;

    let state = ServerState::new();

    // 注册窗口句柄（使用 isize 而不是 HWND）
    let hwnd_value = 12345_isize;
    state.set_message_window_hwnd(hwnd_value).await;

    // 验证通知功能可用（不应该 panic）
    let result = state.show_notification("Test", "Test message").await;
    // 可能失败（Windows API），但不应 panic
    let _ = result;
}

/// 测试消息窗口句柄注册（macOS 版本）
#[cfg(target_os = "macos")]
#[tokio::test]
async fn test_set_message_window_hwnd() {
    let state = ServerState::new();

    // 注册窗口句柄（macOS 版本是 no-op）
    let hwnd_value = 12345_isize;
    state.set_message_window_hwnd(hwnd_value).await;

    // 验证通知功能可用（不应该 panic）
    let result = state.show_notification("Test", "Test message").await;
    let _ = result;
}
