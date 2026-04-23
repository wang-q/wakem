# wakem 宏系统文档

本文档详细介绍 wakem 的宏录制和回放系统。

## 目录

- [概述](#概述)
- [命令行使用](#命令行使用)
- [配置文件定义宏](#配置文件定义宏)
- [核心组件](#核心组件)
- [智能录制特性](#智能录制特性)
- [获取按键扫描码](#获取按键扫描码)
- [宏绑定系统](#宏绑定系统)

## 概述

宏系统允许用户录制和回放键盘/鼠标操作序列。你可以：

- 录制任意键盘和鼠标操作（智能过滤单独修饰键）
- 通过快捷键或命令行触发录制的宏
- 在配置文件中精确定义宏步骤（使用 MacroStep 格式）
- 支持所有动作类型（按键、鼠标、窗口管理、启动程序、延迟等）
- 宏数据持久化存储到配置文件

## 命令行使用

### 录制宏

```bash
# 开始录制宏
wakem record my-macro
# 执行要录制的操作...
# 按 Ctrl+Shift+Esc 停止录制

# 停止录制（另一种方式）
wakem stop-record
```

录制完成后，宏会自动保存到配置文件，并显示通知告知录制结果。

### 播放宏

```bash
# 播放宏
wakem play my-macro
```

播放完成后会显示通知。

### 管理宏

```bash
# 列出所有宏
wakem macros

# 绑定宏到快捷键
wakem bind-macro my-macro F1

# 删除宏
wakem delete-macro my-macro
```

## 配置文件定义宏

你也可以直接在配置文件中定义宏（使用 MacroStep 格式）：

```toml
# 宏定义（使用 MacroStep 格式）
[macros]
# 打开终端（Win+R, 输入 wt, 回车）
"open-terminal" = [
    { delay_ms = 0, action = { Key = { Press = { scan_code = 91, virtual_key = 91 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 0 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 91, virtual_key = 91 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 10 },
    { delay_ms = 100, action = { Delay = { milliseconds = 100 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 110 },
    { delay_ms = 0, action = { Key = { Press = { scan_code = 19, virtual_key = 82 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 120 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 19, virtual_key = 82 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 130 },
    { delay_ms = 100, action = { Delay = { milliseconds = 100 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 230 },
    { delay_ms = 0, action = { Key = { Press = { scan_code = 20, virtual_key = 84 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 240 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 20, virtual_key = 84 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 250 },
    { delay_ms = 0, action = { Key = { Press = { scan_code = 28, virtual_key = 13 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 260 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 28, virtual_key = 13 } } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 270 },
]

# 复制粘贴（带 Ctrl 修饰键）
"copy-paste" = [
    { delay_ms = 0, action = { Key = { Press = { scan_code = 46, virtual_key = 67 } } }, modifiers = { ctrl = true, shift = false, alt = false, meta = false }, timestamp = 0 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 46, virtual_key = 67 } } }, modifiers = { ctrl = true, shift = false, alt = false, meta = false }, timestamp = 10 },
    { delay_ms = 100, action = { Delay = { milliseconds = 100 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 110 },
    { delay_ms = 0, action = { Key = { Press = { scan_code = 47, virtual_key = 86 } } }, modifiers = { ctrl = true, shift = false, alt = false, meta = false }, timestamp = 120 },
    { delay_ms = 0, action = { Key = { Release = { scan_code = 47, virtual_key = 86 } } }, modifiers = { ctrl = true, shift = false, alt = false, meta = false }, timestamp = 130 },
]

# 窗口管理宏示例：将当前窗口居中并调整大小
"center-and-resize" = [
    # 先居中窗口
    { delay_ms = 0, action = { Window = "Center" }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 0 },
    { delay_ms = 200, action = { Delay = { milliseconds = 200 } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 200 },
]

# 启动程序宏示例
"launch-browser" = [
    { delay_ms = 0, action = { Launch = { program = "chrome.exe", args = [], working_dir = null, env_vars = [] } }, modifiers = { ctrl = false, shift = false, alt = false, meta = false }, timestamp = 0 },
]

# 宏触发键绑定
[macro_bindings]
"F1" = "open-terminal"
"Ctrl+Shift+V" = "copy-paste"
```

### MacroStep 字段说明

| 字段 | 类型 | 说明 |
|-----|------|------|
| `delay_ms` | u64 | 此步骤前的延迟时间（毫秒） |
| `action` | Action | 要执行的动作 |
| `modifiers` | ModifierState | 录制时的修饰键状态（ctrl/shift/alt/meta） |
| `timestamp` | u64 | 事件时间戳（用于调试分析） |

### 宏动作类型

宏系统复用 `Action` 枚举，支持所有动作类型：

#### Key 动作 (KeyAction)

| 动作 | 说明 | 参数 |
|-----|------|------|
| `Press` | 按下按键 | `scan_code`, `virtual_key` |
| `Release` | 释放按键 | `scan_code`, `virtual_key` |
| `Click` | 点击按键（按下并释放） | `scan_code`, `virtual_key` |
| `TypeText` | 输入文本字符串 | `String` |
| `Combo` | 组合键（带修饰键） | `modifiers` (ModifierState), `key` (scan_code, virtual_key) |
| `None` | 无操作 | - |

#### Mouse 动作 (MouseAction)

| 动作 | 说明 | 参数 |
|-----|------|------|
| `Move` | 移动鼠标 | `x`, `y`, `relative` |
| `ButtonDown` | 按下按钮 | `button` (Left/Right/Middle/X1/X2) |
| `ButtonUp` | 释放按钮 | `button` |
| `ButtonClick` | 点击按钮 | `button` |
| `Wheel` | 垂直滚轮滚动 | `delta` (正值向上) |
| `HWheel` | 水平滚轮滚动 | `delta` (正值向右) |
| `None` | 无操作 | - |

#### Window 动作 (WindowAction)

支持以下窗口管理动作：

**基础操作**

| 动作 | 说明 |
|-----|------|
| `Center` | 居中窗口 |
| `Minimize` | 最小化窗口 |
| `Maximize` | 最大化窗口 |
| `Restore` | 恢复窗口 |
| `Close` | 关闭窗口 |
| `ToggleTopmost` | 切换置顶 |

**位置与大小**

| 动作 | 说明 | 参数 |
|-----|------|------|
| `MoveToEdge(Edge)` | 移动到屏幕边缘 | Left/Right/Top/Bottom |
| `HalfScreen(Edge)` | 半屏显示 | Left/Right/Top/Bottom |
| `LoopWidth(Alignment)` | 循环调整宽度 | Left/Right/Center/Top/Bottom |
| `LoopHeight(Alignment)` | 循环调整高度 | Left/Right/Center/Top/Bottom |
| `FixedRatio { ratio, scale_index }` | 固定比例窗口 | 比例值, 缩放索引 |
| `NativeRatio { scale_index }` | 原生比例窗口 | 缩放索引 |
| `Move { x, y }` | 移动窗口到绝对坐标 | x, y 坐标 |
| `Resize { width, height }` | 调整窗口大小 | 宽度, 高度 |

**高级功能**

| 动作 | 说明 | 参数 |
|-----|------|------|
| `SwitchToNextWindow` | 同进程窗口切换（Alt+\`） | - |
| `MoveToMonitor(MonitorDirection)` | 跨显示器移动 | Next/Prev/Index(n) |
| `ShowDebugInfo` | 显示调试信息 | - |
| `ShowNotification { title, message }` | 显示通知 | 标题, 内容 |
| `SavePreset { name }` | 保存当前窗口为预设 | 预设名称 |
| `LoadPreset { name }` | 加载指定预设到当前窗口 | 预设名称 |
| `ApplyPreset` | 应用匹配的预设到当前窗口 | - |
| `None` | 无操作 | - |

#### Launch 动作 (LaunchAction)

| 字段 | 类型 | 说明 |
|-----|------|------|
| `program` | String | 程序路径或名称 |
| `args` | Vec\<String\> | 命令行参数列表 |
| `working_dir` | Option\<String\> | 工作目录（null 表示不指定） |
| `env_vars` | Vec\<(String, String)\> | 环境变量键值对列表 |

#### System 动作 (SystemAction)

| 动作 | 说明 |
|-----|------|
| `VolumeUp` | 增加音量 |
| `VolumeDown` | 降低音量 |
| `VolumeMute` | 静音切换 |
| `BrightnessUp` | 增加亮度 |
| `BrightnessDown` | 降低亮度 |

#### 其他动作

| 动作 | 说明 | 参数 |
|-----|------|------|
| `Sequence` | 动作序列（嵌套多个动作） | `Vec<Action>` |
| `Delay` | 延迟等待 | `milliseconds` (u64) |
| `None` | 无操作 | - |

## 核心组件

| 组件 | 文件 | 说明 |
|------|------|------|
| `MacroRecorder` | `src/types/macros.rs` | 录制输入事件，使用 `Action::from_input_event()` |
| `MacroPlayer` | `src/runtime/macro_player.rs` | 回放宏动作，支持修饰键状态重建 |
| `MacroManager` | `src/types/macros.rs` | 宏管理器，负责加载、添加、删除、查询宏定义 |
| `MacroStep` | `src/types/macros.rs` | 宏步骤结构，包含动作、修饰键、时间戳 |
| `Macro` | `src/types/macros.rs` | 宏定义结构，包含名称、步骤列表、元数据 |
| `ModifierState` | `src/types/mod.rs` | 修饰键状态结构（ctrl/shift/alt/meta） |
| `Action` | `src/types/action.rs` | 统一的动作枚举 |

### 架构概览

```
┌─────────────────────────────────────────┐
│           MacroRecorder                 │
│  - 使用 Action::from_input_event()      │
│  - 使用 is_modifier() 过滤单独修饰键     │
│  - 使用 from_virtual_key() + merge()    │
│    跟踪修饰键状态                       │
│  - 录制为 Vec<MacroStep>                │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│          simplify_delays()              │
│                                         │
│  合并连续的短延迟（< 50ms）             │
│  保留必要的延迟（>= 50ms）             │
│  生成最终的 Vec<MacroStep>             │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│         MacroManager                    │
│  - load_from_config(): 从配置加载       │
│  - add_macro() / remove_macro()        │
│  - get_macro() / get_macro_names()     │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│           MacroPlayer                   │
│                                         │
│  遍历 MacroStep:                        │
│  1. 执行 delay_ms 延迟                │
│  2. ensure_modifiers() 重建修饰键状态   │
│  3. execute_action() 调用对应处理器:    │
│     - Key -> send_key_action()         │
│     - Mouse -> send_mouse_action()     │
│     - Window -> (通过 ActionMapper)    │
│     - Launch -> launcher               │
│     - System -> system_control         │
│     - Sequence -> 递归处理             │
│     - Delay / None -> sleep 或跳过     │
│  4. release_all_modifiers() 清理       │
└─────────────────────────────────────────┘
```

### 数据结构

```rust
// 修饰键状态
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,  // Win 键 / Command 键
}

impl ModifierState {
    // 从虚拟键码创建修饰键状态
    pub fn from_virtual_key(key: u16, pressed: bool) -> Option<(Self, bool)>;
    // 合并另一个修饰键状态
    pub fn merge(&mut self, other: &ModifierState);
    // 检查是否无修饰键按下
    pub fn is_empty(&self) -> bool;
}

// 宏步骤
pub struct MacroStep {
    pub delay_ms: u64,           // 延迟（毫秒）
    pub action: Action,          // 动作
    pub modifiers: ModifierState, // 录制时的修饰键状态
    pub timestamp: Timestamp,     // 时间戳
}

// 宏定义
pub struct Macro {
    pub name: String,              // 宏名称
    pub steps: Vec<MacroStep>,    // 步骤列表
    pub created_at: Option<String>, // 创建时间
    pub description: Option<String>, // 描述（可选）
}
```

### 录制流程架构

```
┌─────────────────────────────────────────┐
│           MacroRecorder                 │
│                                         │
│  输入事件 ──► is_modifier()?            │
│              │                         │
│         ┌────┴────┐                    │
│        是        否                    │
│         │         │                    │
│        跳过  from_input_event()        │
│              │                        │
│              ▼                        │
│      update_modifiers()                │
│      (from_virtual_key + merge)        │
│              │                        │
│              ▼                        │
│      创建 MacroStep                   │
│      添加到录制缓冲                    │
└─────────────────┬───────────────────────┘
                  │ stop_recording()
                  ▼
┌─────────────────────────────────────────┐
│          simplify_delays()              │
│                                         │
│  MIN_DELAY_MS = 50ms                   │
│  - 相邻步骤间隔 < 50ms: 不插入延迟     │
│  - 相邻步骤间隔 >= 50ms: 插入 Delay    │
│  - 显式 Delay 动作始终保留             │
└─────────────────────────────────────────┘
```

### 回放流程架构

```
┌─────────────────────────────────────────┐
│           MacroPlayer::play_macro()     │
│                                         │
│  for step in macro.steps:               │
│  ├─ 1. sleep(delay_ms)                 │
│  ├─ 2. ensure_modifiers(&step.modifiers)│
│  │   按 Ctrl -> Alt -> Meta -> Shift    │
│  ├─ 3. execute_action(&step.action)     │
│  │   ├─ Key    -> send_key_action()    │
│  │   ├─ Mouse  -> send_mouse_action()  │
│  │   ├─ Window -> (日志记录)           │
│  │   ├─ Launch -> (日志记录)           │
│  │   ├─ System -> (日志记录)           │
│  │   ├─ Sequence -> 递归执行子动作     │
│  │   └─ Delay/None -> 跳过或 sleep     │
│  └─ end for                            │
│                                         │
│  release_all_modifiers()               │
│  释放顺序: Meta -> Alt -> Shift -> Ctrl │
└─────────────────────────────────────────┘
```

## 智能录制特性

### 1. 过滤单独修饰键

录制时自动跳过单独的 Ctrl/Shift/Alt/Win 键，只记录组合键。例如：

- 录制 `Ctrl+C`: 只记录 2 个步骤（C 的按下和释放），而不是 4 个
- 单独按 `Ctrl`: 完全跳过，不记录任何内容

实现方式：通过 `KeyEvent::is_modifier()` 方法判断是否为修饰键。判断依据为虚拟键码匹配：

| 修饰键 | 虚拟键码 (含左右变体) |
|--------|---------------------|
| Shift | 0x10, 0xA0 (左), 0xA1 (右) |
| Ctrl | 0x11, 0xA2 (左), 0xA3 (右) |
| Alt | 0x12, 0xA4 (左), 0xA5 (右) |
| Win/Meta | 0x5B (左), 0x5C (右) |

### 2. 跟踪修饰键状态

记录每个动作发生时的完整修饰键状态：

```rust
pub struct ModifierState {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,  // Win 键
}
```

这对于回放时正确还原上下文非常重要。录制过程中通过 `ModifierState::from_virtual_key()` 解析修饰键事件，并用 `merge()` 方法合并到当前状态。

### 3. 延迟优化

自动合并短延迟（< 50ms），只保留有意义的延迟：

```rust
const MIN_DELAY_MS: u64 = 50; // 最小延迟阈值
```

例如：
- 按下 A (0ms) → 释放 A (10ms) → 按下 B (90ms): 只在 A 和 B 之间插入一个约 80ms 的延迟
- 按下 A (0ms) → 释放 A (10ms) → **Delay(100ms)** → 按下 B: 保留显式的 100ms 延迟

## 获取按键扫描码

如果你需要手动编写宏配置，可能需要知道特定按键的扫描码和虚拟键码。

完整的键名列表和扫描码/虚拟键码对照表请参阅 [keys.md](keys.md)。

### 获取扫描码的方法

1. **使用 wakem 日志**: 设置 `log_level = "debug"` 后启动守护进程，查看日志中的按键信息
2. **在线工具**: 使用 Windows Virtual-Key Codes 在线查询工具
3. **MSDN 文档**: 参考 [Virtual-Key Codes (Winuser.h)](https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes)

## 宏绑定系统

宏可以通过快捷键触发，需要在 `[macro_bindings]` 部分进行配置：

```toml
[macros]
"my-macro" = [ ... ]  # 宏定义

[macro_bindings]
"F1" = "my-macro"          # F1 键触发
"Ctrl+Shift+V" = "my-macro"  # Ctrl+Shift+V 触发
```

### 绑定规则

1. 绑定的触发键必须是有效的按键名称或快捷键格式
2. 绑定引用的宏必须存在于 `[macros]` 中（配置验证时会检查）
3. 一个宏可以被多个触发键绑定
4. 删除宏时会同时删除相关绑定
5. 空步骤的宏不会报错，但会产生警告日志

### 配置验证

配置加载时会执行以下验证（参见 `Config::validate()`）：

- `[macro_bindings]` 中引用的宏名必须在 `[macros]` 中存在
- 步骤为空的宏会输出 warning 日志提示

### 绑定示例

```toml
[macros]
# 快速打开常用应用
"open-terminal" = [
    { delay_ms = 0, action = { Launch = { program = "wt.exe", args = [], working_dir = null, env_vars = [] } }, ... },
]
"open-explorer" = [
    { delay_ms = 0, action = { Launch = { program = "explorer.exe", args = [], working_dir = null, env_vars = [] } }, ... },
]
"open-browser" = [
    { delay_ms = 0, action = { Launch = { program = "chrome.exe", args = [], working_dir = null, env_vars = [] } }, ... ],
]

[macro_bindings]
# F 系列 Function 键绑定
"F1" = "open-terminal"
"F2" = "open-explorer"
"F3" = "open-browser"

# 组合键绑定
"Ctrl+Alt+T" = "open-terminal"
```

---

更多配置信息请参考 [config.md](config.md)，开发相关信息请参考 [developer.md](developer.md)。
