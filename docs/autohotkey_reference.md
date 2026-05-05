# AutoHotkey 参考项目分析

本文档基于 AutoHotkey 2.0.23 源代码，结合 wakem 项目实际代码结构，提取与 wakem 实现相关的功能参考。

---

## 1. wakem 与 AutoHotkey 功能对照

| wakem 模块 | wakem 文件 | AutoHotkey 参考 |
|-----------|-----------|----------------|
| 输入钩子 | `platform/windows/input_device.rs` | `hook.cpp` - 低级别钩子实现 |
| 按键发送 | `platform/windows/output_device.rs` | `keyboard_mouse.cpp` - SendInput |
| 按键发送辅助 | `platform/common/output_helpers.rs` | `keyboard_mouse.cpp` - 字符映射 |
| 热键映射 | `runtime/mapper.rs` | `hotkey.cpp` - 热键匹配逻辑 |
| 层管理 | `runtime/layer_manager.rs` | `hotkey.cpp` - 前缀键状态管理 |
| 宏录制/播放 | `runtime/macro_player.rs` + `types/macros.rs` | — (AutoHotkey 宏为脚本级) |
| 窗口动作 | `runtime/window_actions.rs` | `window.cpp` - 窗口操作 API |
| 窗口 API | `platform/windows/window_api.rs` | `window.cpp` - Win32 API |
| 窗口管理器 | `platform/common/window_manager.rs` | `window.cpp` - 高层窗口操作 |
| 窗口事件钩子 | `platform/windows/window_event_hook.rs` | — (窗口切换上下文) |
| 进程启动 | `platform/common/launcher.rs` | `lib/process.cpp` - CreateProcess |
| 配置解析 | `config.rs` | `script.cpp` - 配置加载和验证 |
| IPC 通信 | `ipc/` | — (AutoHotkey 无守护进程模式) |
| 平台抽象 | `platform/traits.rs` | — (AutoHotkey 仅 Windows) |
| 平台类型 | `platform/types.rs` | — (跨平台类型定义) |

---

## 2. 输入系统参考

### 2.1 输入钩子 (wakem: `platform/windows/input_device.rs`)

**AutoHotkey 参考**: `hook.cpp`

**关键实现**:
- AutoHotkey 使用 `SetWindowsHookEx` 安装 `WH_KEYBOARD_LL` 和 `WH_MOUSE_LL`
- 通过 `KBDLLHOOKSTRUCT` 和 `MSLLHOOKSTRUCT` 获取输入事件
- 区分物理输入和脚本生成事件（通过 `dwExtraInfo`）
- 滚轮事件映射为虚拟键码（`VK_WHEEL_UP`/`VK_WHEEL_DOWN`）

**wakem 对应**:
```rust
// wakem: platform/windows/input_device.rs
// Windows 使用 Raw Input API (WM_INPUT)
// 通过 platform/windows/input.rs 的 RawInputDevice 实现
// 通用逻辑在 platform/common/input_device.rs 的 GenericInputDevice

// 跨平台 trait 定义在 platform/traits.rs
pub trait InputDevice: Send {
    fn register(&mut self) -> Result<()>;
    fn unregister(&mut self);
    fn poll_event(&mut self) -> Option<InputEvent>;
    fn is_running(&self) -> bool;
    fn stop(&mut self);
    fn run_once(&mut self) -> Result<bool>;
}
```

**输入事件类型** (`types/input.rs`):
```rust
pub enum InputEvent {
    Key(KeyEvent),      // 键盘事件
    Mouse(MouseEvent),  // 鼠标事件
}

pub struct KeyEvent {
    pub scan_code: u16,
    pub virtual_key: u16,
    pub state: KeyState,         // Pressed / Released
    pub modifiers: ModifierState,
    pub device_type: DeviceType,
    pub timestamp: Timestamp,
    pub is_injected: bool,       // 区分物理/模拟输入
}

pub struct MouseEvent {
    pub event_type: MouseEventType, // Move/ButtonDown/Up/Wheel/HWheel
    pub x: i32, pub y: i32,
    pub modifiers: ModifierState,
    pub timestamp: Timestamp,
    pub is_injected: bool,
}
```

### 2.2 按键发送 (wakem: `platform/windows/output_device.rs`)

**AutoHotkey 参考**: `keyboard_mouse.cpp`

**发送模式对比**:

| 模式 | AutoHotkey | wakem |
|-----|-----------|-------|
| SendInput | `SendInput()` API，批量发送 | `SendInput()` via windows crate |
| SendEvent | `keybd_event()`/`mouse_event()` | 备用方案 |
| 修饰键管理 | 跟踪修饰键状态，自动释放 | `OutputDevice` trait 的 `send_combo`/`send_key_action` |

**关键要点**:
- 使用 `INPUT` 结构发送键盘/鼠标事件
- `KEYEVENTF_SCANCODE` + `KEYEVENTF_EXTENDEDKEY` 支持扩展键
- `dwExtraInfo` 标记脚本生成的事件（避免递归）
- 通用逻辑（字符映射、文本输入、组合键）在 `platform/common/output_helpers.rs`

**wakem OutputDevice trait** (`platform/traits.rs`):
```rust
pub trait OutputDevice: Send + Sync {
    fn send_key(&self, scan_code: u16, virtual_key: u16, release: bool) -> Result<()>;
    fn send_mouse_move(&self, x: i32, y: i32, relative: bool) -> Result<()>;
    fn send_mouse_button(&self, button: MouseButton, release: bool) -> Result<()>;
    fn send_mouse_wheel(&self, delta: i32, horizontal: bool) -> Result<()>;
    // 默认实现：send_key_action, send_text, send_combo, send_mouse_action
}
```

### 2.3 热键映射 (wakem: `runtime/mapper.rs`)

**AutoHotkey 参考**: `hotkey.cpp`

**AutoHotkey 热键系统**:
- **双模式注册**: `RegisterHotKey` API vs 键盘钩子
- **热键变体**: 同一按键在不同条件下的多种触发
- **前缀键**: `prefix & key` 形式的组合热键

**wakem 对应设计**:
```rust
// wakem: types/mapping.rs
pub struct MappingRule {
    pub name: Option<String>,
    pub trigger: Trigger,
    pub action: Action,
    pub context: Option<ContextCondition>,
    pub enabled: bool,
}

pub enum Trigger {
    Key { scan_code: Option<u16>, virtual_key: Option<u16>, modifiers: ModifierState },
    MouseButton { button: MouseButton, modifiers: ModifierState },
    HotString { trigger: String },       // 文本展开
    Chord(Vec<Trigger>),                 // 序列组合键
    Timer { interval_ms: u64 },          // 定时触发
    Always,                              // 始终触发
}
```

**上下文条件** (`types/context.rs`):
```rust
pub struct ContextCondition {
    pub window_class: Option<String>,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
    pub executable_path: Option<String>,  // 可执行文件路径匹配
}
// 支持通配符匹配 (* 和 ?)，大小写不敏感
// 对应 AutoHotkey 的 ahk_class, ahk_exe, ahk_id 等窗口匹配
```

**KeyMapper** (`runtime/mapper.rs`):
- 先匹配上下文规则 (`context_rules`)，再匹配基础规则 (`rules`)
- `adjust_action_for_key_state`: 将 Click 动作按 Press/Release 拆分
- 支持 Sequence 动作的 Press/Release 分割（通过 `Action::None` 占位符）

### 2.4 层管理 (wakem: `runtime/layer_manager.rs`)

**AutoHotkey 参考**: `hotkey.cpp` 的前缀键机制

**AutoHotkey 前缀键实现**:
- 单独跟踪前缀键状态
- 检查前缀键是否有启用的后缀
- 超时处理

**wakem 层系统**:
```rust
// wakem: types/layer.rs
pub enum LayerMode {
    Hold,    // 按住激活，释放退出
    Toggle,  // 切换模式
}

pub struct Layer {
    pub name: String,
    pub activation_key: u16,   // 扫描码
    pub activation_vk: u16,    // 虚拟键码
    pub mode: LayerMode,
    pub mappings: Vec<MappingRule>,
}

pub struct LayerStack {
    base_layer: Vec<MappingRule>,    // 基础层映射
    active_layers: Vec<Layer>,       // 活动层（按优先级排序）
    hold_layers: Vec<String>,        // Hold 模式激活的层
}
```

**LayerManager** (`runtime/layer_manager.rs`):
- `activation_key_index`: 快速查找激活键对应的层
- Hold 模式：按下激活，释放退出
- Toggle 模式：按一次激活，再按退出
- 层优先级：后激活的层覆盖先激活的层
- 查找顺序：层映射 → 基础映射

### 2.5 宏录制与播放 (wakem: `types/macros.rs` + `runtime/macro_player.rs`)

**AutoHotkey 参考**: AutoHotkey 的宏为脚本级别，无录制/播放功能

**wakem 宏系统**:
```rust
// types/macros.rs
pub struct MacroStep {
    pub delay_ms: u64,
    pub action: Action,
    pub modifiers: ModifierState,   // 录制时的修饰键状态
    pub timestamp: Timestamp,
}

pub struct Macro {
    pub name: String,
    pub steps: Vec<MacroStep>,
    pub created_at: Option<String>,
    pub description: Option<String>,
}

pub struct MacroRecorder { ... }  // 异步录制，自动跟踪修饰键状态
```

**MacroPlayer** (`runtime/macro_player.rs`):
- 支持取消标志 (`cancel_flag: Option<Arc<AtomicBool>>`)
- 自动管理修饰键状态（`ensure_modifiers`：按差异增量同步）
- LIFO 释放修饰键（Meta→Alt→Shift→Ctrl）
- 延迟简化：合并连续动作，仅保留超过阈值的延迟

---

## 3. 窗口管理参考

### 3.1 窗口操作 (wakem: `platform/common/window_manager.rs` + `platform/windows/window_api.rs`)

**AutoHotkey 参考**: `window.cpp`

**窗口激活策略** (按优先级):
1. `SetForegroundWindow()` - 简单激活
2. `AttachThreadInput()` - 连接线程输入队列
3. Alt 键技巧 - 解除前台锁定

**窗口查找**:
- 支持多种匹配模式（标题、类名、进程名、可执行文件路径）
- 通配符匹配（`*` 和 `?`），大小写不敏感
- 对应 AutoHotkey 的 `ahk_class`, `ahk_exe`, `ahk_id` 语法

**wakem 窗口管理 trait 层次** (`platform/traits.rs`):
```rust
pub trait WindowApiBase: Send + Sync {    // 平台底层 API
    type WindowId: Copy + Send + 'static;
    fn get_foreground_window(&self) -> Option<Self::WindowId>;
    fn set_window_pos(...) -> Result<()>;
    fn minimize_window(...) -> Result<()>;
    fn maximize_window(...) -> Result<()>;
    fn restore_window(...) -> Result<()>;
    fn close_window(...) -> Result<()>;
    fn set_topmost(...) -> Result<()>;
    fn get_window_rect(...) -> Result<WindowFrame>;
    fn get_monitors(&self) -> Vec<MonitorInfo>;
    fn get_process_name(...) -> Option<String>;
    fn get_executable_path(...) -> Option<String>;
    // ...
}

pub trait WindowOperations: Send + Sync { ... }
pub trait WindowStateQueries: Send + Sync { ... }
pub trait MonitorOperations: Send + Sync { ... }
pub trait ForegroundWindowOperations: Send + Sync { ... }
pub trait WindowSwitching: Send + Sync { ... }

pub trait WindowManagerExt: WindowOperations + WindowStateQueries + ... {
    fn move_to_center(&self, window: WindowId) -> Result<()>;
    fn move_to_edge(&self, window: WindowId, edge: Edge) -> Result<()>;
    fn set_half_screen(&self, window: WindowId, edge: Edge) -> Result<()>;
    fn loop_width(&self, window: WindowId, align: Alignment) -> Result<()>;
    fn loop_height(&self, window: WindowId, align: Alignment) -> Result<()>;
    fn set_fixed_ratio(&self, window: WindowId, ratio: f32, ...) -> Result<()>;
    fn set_native_ratio(&self, window: WindowId, ...) -> Result<()>;
    fn toggle_topmost(&self, window: WindowId) -> Result<bool>;
}

pub trait WindowManagerTrait:
    WindowOperations + WindowStateQueries + MonitorOperations
    + ForegroundWindowOperations + WindowSwitching + WindowManagerExt
    + Send + Sync { }
```

**WindowAction** (`types/action.rs`):
```rust
pub enum WindowAction {
    Center,                                    // 居中
    MoveToEdge(Edge),                          // 移到屏幕边缘
    HalfScreen(Edge),                          // 半屏
    LoopWidth(Alignment),                      // 循环宽度 (75%→60%→50%→40%→25%)
    LoopHeight(Alignment),                     // 循环高度 (75%→50%→25%)
    FixedRatio { ratio: f32, scale_index: usize }, // 固定比例 (4:3 等)
    NativeRatio { scale_index: usize },        // 屏幕原生比例
    SwitchToNextWindow,                        // 同进程窗口切换 (Alt+`)
    MoveToMonitor(MonitorDirection),           // 跨显示器移动
    Move { x: i32, y: i32 },                  // 绝对移动
    Resize { width: i32, height: i32 },        // 调整大小
    Minimize, Maximize, Restore, Close,        // 基本操作
    ToggleTopmost,                             // 置顶切换
    ShowDebugInfo,                             // 调试信息
    ShowNotification { title, message },       // 通知
    SavePreset { name }, LoadPreset { name },  // 窗口预设
    ApplyPreset,                               // 应用匹配预设
    None,
}
```

**窗口动作执行** (`runtime/window_actions.rs`):
- 通过 `WindowManagerTrait` trait 分发所有窗口操作
- 跨显示器移动：计算相对坐标比例，映射到目标显示器
- 可选 `NotificationService` 和 `WindowPresetManager` 支持

### 3.2 显示器管理

**AutoHotkey 参考**: `lib/env.cpp`

**关键 API**:
- `EnumDisplayMonitors` - 枚举显示器
- `GetMonitorInfo` - 获取显示器信息
- `MONITORINFOF_PRIMARY` - 识别主显示器

**wakem 跨显示器移动**:
```rust
// platform/types.rs
pub struct MonitorInfo {
    pub x: i32, pub y: i32,
    pub width: i32, pub height: i32,
}

pub struct MonitorWorkArea {  // 排除任务栏
    pub x: i32, pub y: i32,
    pub width: i32, pub height: i32,
}

// platform/windows/window_api.rs
// 使用 EnumDisplayMonitors + GetMonitorInfoW 枚举显示器

// runtime/window_actions.rs
// 跨显示器移动：计算相对坐标比例，映射到目标显示器
```

---

## 4. 进程启动参考

### 4.1 程序启动 (wakem: `platform/common/launcher.rs`)

**AutoHotkey 参考**: `lib/process.cpp`

**启动方式**:
- `CreateProcess` - 标准启动，支持参数和工作目录
- `CreateProcessWithLogonW` - 以指定用户运行
- `ShellExecuteEx` - 使用系统关联打开

**wakem 对应**:
```rust
// wakem: platform/common/launcher.rs
// 使用 std::process::Command
// 支持 program, args, working_dir, env_vars

// wakem: types/action.rs
pub struct LaunchAction {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_vars: Vec<(String, String)>,
}
```

---

## 5. IPC 通信参考

### 5.1 进程间通信 (wakem: `ipc/`)

**AutoHotkey 参考**: 无对应（AutoHotkey 无守护进程模式）

wakem 采用客户端-服务端架构，守护进程通过 IPC 接受控制命令。

**IPC 模块结构**:
```rust
ipc/
├── mod.rs           // 模块入口，re-export 常用类型
├── auth.rs          // HMAC-SHA256 挑战-响应认证
├── client.rs        // IPC 客户端
├── discovery.rs     // 实例发现（扫描运行中的实例）
├── io.rs            // IO 操作（端口分配等）
├── messages.rs      // 消息定义
├── rate_limiter.rs  // 连接速率限制（防暴力破解）
├── security.rs      // 安全相关（IP 白名单，仅允许私有 IPv4）
└── server.rs        // IPC 服务端
```

**安全特性**:
- TCP 协议 + JSON 序列化
- HMAC-SHA256 挑战-响应认证
- 仅允许私有 IPv4 地址连接
- 连接速率限制

---

## 6. 平台抽象层

### 6.1 设计概览

wakem 采用 trait-based 平台抽象层设计：

```
platform/
├── mod.rs           // 平台模块入口
├── traits.rs        // 平台 trait 定义
├── types.rs         // 平台类型定义
├── macros.rs        // 平台相关宏
├── common/          // 通用实现
│   ├── input_device.rs    // 通用输入设备
│   ├── output_helpers.rs  // 输出辅助（字符映射等）
│   ├── key_names.rs       // 按键名称映射
│   ├── launcher.rs        // 启动器
│   ├── window_manager.rs  // 通用窗口管理器
│   ├── window_preset.rs   // 窗口预设
│   ├── app_control.rs     // 应用控制
│   └── tray.rs            // 托盘通用逻辑
├── windows/         // Windows 平台实现
│   ├── input.rs           // Raw Input API
│   ├── input_device.rs    // 输入设备
│   ├── output_device.rs   // SendInput 输出
│   ├── window_api.rs      // Win32 窗口 API
│   ├── window_manager.rs  // 窗口管理器
│   ├── window_event_hook.rs // 窗口事件钩子
│   ├── context.rs         // Windows 上下文
│   ├── notification.rs    // Windows 通知
│   ├── platform_utils.rs  // 平台工具
│   ├── tray.rs            // 系统托盘
│   └── app_control.rs     // 应用控制
└── mock/            // 测试用 Mock 实现
    └── mock_window_api.rs
```

### 6.2 PlatformFactory trait

```rust
pub trait PlatformFactory {
    type InputDevice: InputDevice;
    type OutputDevice: OutputDevice;
    type WindowManager: WindowManagerTrait;
    type WindowPresetManager: WindowPresetManager;
    type NotificationService: NotificationService;
    type Launcher: Launcher;
    type WindowEventHook: WindowEventHook;

    fn create_input_device(config, sender) -> Result<Self::InputDevice>;
    fn create_output_device() -> Self::OutputDevice;
    fn create_window_manager() -> Self::WindowManager;
    // ...
}
```

---

## 7. 类型系统

### 7.1 核心类型

```
types/
├── mod.rs         // 类型模块入口
├── action.rs      // 动作类型 (KeyAction, MouseAction, WindowAction, Action)
├── context.rs     // 上下文条件 (ContextCondition, wildcard_match)
├── input.rs       // 输入类型 (KeyEvent, MouseEvent, InputEvent, ModifierState)
├── key_codes.rs   // 虚拟键码 (VirtualKey, VK_* 常量)
├── layer.rs       // 层定义 (Layer, LayerMode, LayerStack)
├── macros.rs      // 宏类型 (Macro, MacroStep, MacroRecorder)
└── mapping.rs     // 映射规则 (MappingRule, Trigger)
```

### 7.2 Action 类型层次

```rust
pub enum Action {
    Key(KeyAction),           // 键盘动作
    Mouse(MouseAction),       // 鼠标动作
    Window(WindowAction),     // 窗口动作
    Launch(LaunchAction),     // 启动程序
    Sequence(Vec<Action>),    // 动作序列
    Delay { milliseconds: u64 }, // 延迟（宏播放用）
    None,
}

pub enum KeyAction {
    Press { scan_code, virtual_key },
    Release { scan_code, virtual_key },
    Click { scan_code, virtual_key },
    TypeText(String),
    Combo { modifiers, key },
    None,
}

pub enum MouseAction {
    Move { x, y, relative },
    ButtonDown { button },
    ButtonUp { button },
    ButtonClick { button },
    Wheel { delta },
    HWheel { delta },
    None,
}
```

### 7.3 VirtualKey 系统

```rust
// types/key_codes.rs
// 使用 Windows VK_* 值作为内部统一键码

pub struct VirtualKey(u16);  // 0 = 无效，非零 = Windows VK 码

// 常量定义: VK_SHIFT(0x10), VK_CONTROL(0x11), VK_ALT(0x12), VK_LMETA(0x5B) 等
// ModifierState: shift, ctrl, alt, meta (支持 from_internal_vk/apply_from_internal_vk)
```

---

## 8. 实现建议

### 8.1 输入钩子设计

参考 `hook.cpp`:
1. **事件过滤**: 通过 `is_injected` 标记脚本生成的事件
2. **状态跟踪**: `ModifierState` 跟踪修饰键状态
3. **滚轮处理**: `MouseEventType::Wheel(delta)` / `HWheel(delta)`

### 8.2 热键匹配优化

参考 `hotkey.cpp`:
1. **分层匹配**: 先匹配上下文规则，再匹配基础规则
2. **上下文条件**: 窗口类名、进程名、可执行文件路径匹配（通配符）
3. **修饰键处理**: 子集匹配（trigger modifiers ⊆ event modifiers），支持 Hyper 键

### 8.3 窗口激活策略

参考 `window.cpp`:
1. **渐进式激活**: 从简单方法到复杂方法
2. **线程输入**: 使用 `AttachThreadInput` 解决焦点问题
3. **恢复最小化**: 先恢复再激活

### 8.4 按键发送实现

参考 `keyboard_mouse.cpp`:
1. **SendInput**: 使用 `INPUT` 结构发送键盘/鼠标事件
2. **修饰键同步**: `MacroPlayer::ensure_modifiers` 按差异增量同步
3. **扩展键**: `KEYEVENTF_EXTENDEDKEY` 处理扩展扫描码

---

## 9. 文件索引

### 9.1 wakem 核心文件

| 文件 | 功能 | AutoHotkey 参考 |
|------|------|----------------|
| `platform/traits.rs` | 平台抽象 trait 定义 | — |
| `platform/types.rs` | 平台类型定义 | — |
| `platform/common/input_device.rs` | 通用输入设备 | `hook.cpp` |
| `platform/common/output_helpers.rs` | 输出辅助 | `keyboard_mouse.cpp` |
| `platform/common/launcher.rs` | 启动器 | `lib/process.cpp` |
| `platform/common/window_manager.rs` | 通用窗口管理器 | `window.cpp` |
| `platform/windows/input_device.rs` | Windows 输入设备 | `hook.cpp` |
| `platform/windows/input.rs` | Raw Input API | `hook.cpp` |
| `platform/windows/output_device.rs` | SendInput 输出 | `keyboard_mouse.cpp` |
| `platform/windows/window_api.rs` | Win32 窗口 API | `window.cpp` |
| `platform/windows/window_manager.rs` | Windows 窗口管理器 | `window.cpp` |
| `platform/windows/window_event_hook.rs` | 窗口事件钩子 | — |
| `runtime/mapper.rs` | 热键映射 | `hotkey.cpp` |
| `runtime/layer_manager.rs` | 层管理 | `hotkey.cpp` (前缀键) |
| `runtime/macro_player.rs` | 宏播放器 | — |
| `runtime/window_actions.rs` | 窗口动作执行 | `window.cpp` |
| `config.rs` | 配置解析 | `script.cpp` |
| `types/action.rs` | 动作类型 | — |
| `types/mapping.rs` | 映射规则 | `hotkey.cpp` |
| `types/layer.rs` | 层定义 | `hotkey.cpp` |
| `types/input.rs` | 输入事件类型 | — |
| `types/context.rs` | 上下文条件 | `window.cpp` |
| `types/key_codes.rs` | 虚拟键码 | `defines.h` |
| `types/macros.rs` | 宏类型/录制器 | — |
| `ipc/auth.rs` | IPC 认证 | — |
| `ipc/server.rs` | IPC 服务端 | — |
| `ipc/client.rs` | IPC 客户端 | — |
| `ipc/messages.rs` | IPC 消息定义 | — |
| `ipc/security.rs` | IPC 安全 | — |
| `ipc/discovery.rs` | 实例发现 | — |
| `ipc/rate_limiter.rs` | 速率限制 | — |
| `daemon.rs` | 守护进程逻辑 | — |
| `cli.rs` | 命令行界面 | — |
| `tray.rs` | 系统托盘 | — |
| `shutdown.rs` | 关闭处理 | — |

### 9.2 AutoHotkey 参考文件

| 文件 | 功能描述 |
|------|---------|
| `hook.cpp/h` | 输入钩子核心 |
| `hotkey.cpp/h` | 热键系统 |
| `keyboard_mouse.cpp/h` | 键盘鼠标发送 |
| `window.cpp/h` | 窗口管理 |
| `WinGroup.cpp/h` | 窗口组 |
| `lib/process.cpp` | 进程管理 |
| `lib/wait.cpp` | 等待机制 |
| `lib/sound.cpp` | 声音控制 |
| `lib/env.cpp` | 显示器管理 |
| `defines.h` | 常量定义 |
