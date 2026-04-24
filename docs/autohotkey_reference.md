# AutoHotkey 参考项目分析

本文档基于 AutoHotkey 2.0.23 源代码，结合 wakem 项目实际代码结构，提取与 wakem 实现相关的功能参考。

---

## 1. wakem 与 AutoHotkey 功能对照

| wakem 模块 | wakem 文件 | AutoHotkey 参考 |
|-----------|-----------|----------------|
| 输入钩子 | `platform/windows/input_device.rs` | `hook.cpp` - 低级别钩子实现 |
| 按键发送 | `platform/windows/output_device.rs` | `keyboard_mouse.cpp` - SendInput/SendEvent |
| 热键映射 | `runtime/mapper.rs` | `hotkey.cpp` - 热键匹配逻辑 |
| 层管理 | `runtime/layer_manager.rs` | `hotkey.cpp` - 前缀键状态管理 |
| 窗口管理 | `platform/windows/window_manager.rs` | `window.cpp` - 窗口操作 API |
| 进程启动 | `platform/windows/launcher.rs` | `lib/process.cpp` - CreateProcess |
| 配置解析 | `config.rs` | `script.cpp` - 配置加载和验证 |

---

## 2. 输入系统参考

### 2.1 输入钩子 (wakem: `input_device.rs`)

**AutoHotkey 参考**: `hook.cpp`

**关键实现**:
- 使用 `SetWindowsHookEx` 安装 `WH_KEYBOARD_LL` 和 `WH_MOUSE_LL`
- 通过 `KBDLLHOOKSTRUCT` 和 `MSLLHOOKSTRUCT` 获取输入事件
- 区分物理输入和脚本生成事件（通过 `dwExtraInfo`）
- 滚轮事件映射为虚拟键码（`VK_WHEEL_UP`/`VK_WHEEL_DOWN`）

**wakem 对应**:
```rust
// wakem: platform/windows/input_device.rs
// 使用 windows crate 的 SetWindowsHookEx
// 处理 WM_INPUT 或低级别钩子
```

### 2.2 按键发送 (wakem: `output_device.rs`)

**AutoHotkey 参考**: `keyboard_mouse.cpp`

**发送模式对比**:

| 模式 | AutoHotkey | wakem |
|-----|-----------|-------|
| SendInput | `SendInput()` API，批量发送 | `SendInput()` via windows crate |
| SendEvent | `keybd_event()`/`mouse_event()` | 备用方案 |
| 修饰键管理 | 跟踪修饰键状态，自动释放 | 需要手动管理 |

**关键要点**:
- 使用 `INPUT` 结构数组批量发送
- `KEYEVENTF_UNICODE` 支持任意字符
- `dwExtraInfo` 标记脚本生成的事件（避免递归）

### 2.3 热键映射 (wakem: `mapper.rs`)

**AutoHotkey 参考**: `hotkey.cpp`

**AutoHotkey 热键系统**:
- **双模式注册**: `RegisterHotKey` API vs 键盘钩子
- **热键变体**: 同一按键在不同条件下的多种触发
- **前缀键**: `prefix & key` 形式的组合热键

**wakem 对应设计**:
```rust
// wakem: types/mapping.rs
pub struct MappingRule {
    pub trigger: Trigger,    // 触发条件
    pub action: Action,      // 执行动作
    pub context: Option<ContextCondition>, // 上下文条件
}

pub enum Trigger {
    Key { scan_code, virtual_key, modifiers },
    MouseButton { button, modifiers },
    Chord(Vec<Trigger>),
}
```

### 2.4 层管理 (wakem: `layer_manager.rs`)

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
    pub activation_key: u16,  // 扫描码
    pub activation_vk: u16,   // 虚拟键码
    pub mode: LayerMode,
    pub mappings: Vec<MappingRule>,
}
```

---

## 3. 窗口管理参考

### 3.1 窗口操作 (wakem: `window_manager.rs`)

**AutoHotkey 参考**: `window.cpp`

**窗口激活策略** (按优先级):
1. `SetForegroundWindow()` - 简单激活
2. `AttachThreadInput()` - 连接线程输入队列
3. Alt 键技巧 - 解除前台锁定

**窗口查找**:
- 支持多种匹配模式（标题、类名、进程名）
- 特殊语法：`ahk_class`, `ahk_exe`, `ahk_id`

**wakem 对应**:
```rust
// wakem: platform/windows/window_api.rs
// 使用 Win32 API: SetForegroundWindow, SetWindowPos, etc.
```

### 3.2 显示器管理

**AutoHotkey 参考**: `lib/env.cpp`

**关键 API**:
- `EnumDisplayMonitors` - 枚举显示器
- `GetMonitorInfo` - 获取显示器信息
- `MONITORINFOF_PRIMARY` - 识别主显示器

**wakem 跨显示器移动**:
```rust
// 需要处理多显示器坐标转换
// 参考 AutoHotkey 的 MonitorGet 实现
```

---

## 4. 进程启动参考

### 4.1 程序启动 (wakem: `launcher.rs`)

**AutoHotkey 参考**: `lib/process.cpp`

**启动方式**:
- `CreateProcess` - 标准启动，支持参数和工作目录
- `CreateProcessWithLogonW` - 以指定用户运行
- `ShellExecuteEx` - 使用系统关联打开

**wakem 对应**:
```rust
// wakem: platform/windows/launcher.rs
// 使用 windows crate 的 CreateProcessW
```

---

## 6. 实现建议

### 6.1 输入钩子设计

参考 `hook.cpp`:
1. **事件过滤**: 通过 `dwExtraInfo` 标记脚本生成的事件
2. **状态跟踪**: 维护物理按键状态数组
3. **滚轮处理**: 将滚轮事件映射为虚拟键码

### 6.2 热键匹配优化

参考 `hotkey.cpp`:
1. **分层匹配**: 先匹配层，再匹配映射规则
2. **上下文条件**: 窗口类名、进程名匹配
3. **修饰键处理**: 区分左右修饰键

### 6.3 窗口激活策略

参考 `window.cpp`:
1. **渐进式激活**: 从简单方法到复杂方法
2. **线程输入**: 使用 `AttachThreadInput` 解决焦点问题
3. **恢复最小化**: 先恢复再激活

### 6.4 按键发送实现

参考 `keyboard_mouse.cpp`:
1. **批量发送**: 使用 `SendInput` 数组
2. **修饰键同步**: 发送前检查并同步修饰键状态
3. **Unicode 支持**: 使用 `KEYEVENTF_UNICODE`

---

## 7. 文件索引

### 7.1 wakem 核心文件

| 文件 | 功能 | AutoHotkey 参考 |
|------|------|----------------|
| `platform/windows/input_device.rs` | 输入钩子 | `hook.cpp` |
| `platform/windows/output_device.rs` | 按键发送 | `keyboard_mouse.cpp` |
| `platform/windows/window_manager.rs` | 窗口管理 | `window.cpp` |
| `platform/windows/launcher.rs` | 进程启动 | `lib/process.cpp` |
| `runtime/mapper.rs` | 热键映射 | `hotkey.cpp` |
| `runtime/layer_manager.rs` | 层管理 | `hotkey.cpp` (前缀键) |
| `config.rs` | 配置解析 | `script.cpp` |
| `types/action.rs` | 动作类型 | `types/action.rs` |
| `types/mapping.rs` | 映射规则 | `hotkey.cpp` |
| `types/layer.rs` | 层定义 | `hotkey.cpp` |

### 7.2 AutoHotkey 参考文件

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
