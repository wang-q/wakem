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
| `Combo` | 组合键（带修饰键） | `modifiers`, `key` |
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

#### Window 动作 (WindowAction)

支持所有窗口管理动作，参见 [配置指南](config.md#窗口管理动作)。

#### 其他动作

| 动作 | 说明 | 参数 |
|-----|------|------|
| `Launch` | 启动程序 | `program`, `args`, `working_dir`, `env_vars` |
| `System` | 系统控制 | VolumeUp/VolumeDown/VolumeMute/BrightnessUp/BrightnessDown |
| `Sequence` | 动作序列（嵌套多个动作） | `Vec<Action>` |
| `Delay` | 延迟等待 | `milliseconds` |
| `None` | 无操作 | - |

## 核心组件

| 组件 | 文件 | 说明 |
|------|------|------|
| `MacroRecorder` | `src/types/macros.rs` | 录制输入事件，使用 `Action::from_input_event()` |
| `MacroPlayer` | `src/runtime/macro_player.rs` | 回放宏动作，支持修饰键状态重建 |
| `MacroStep` | `src/types/macros.rs` | 宏步骤结构，包含动作、修饰键、时间戳 |
| `Macro` | `src/types/macros.rs` | 宏定义结构，包含名称、步骤列表、元数据 |
| `Action` | `src/types/action.rs` | 统一的动作枚举 |

### 架构概览

```
┌─────────────────────────────────────────┐
│           MacroRecorder                 │
│  - 使用 Action::from_input_event()      │
│  - 使用 is_modifier() 过滤单独修饰键     │
│  - 使用 merge() 跟踪修饰键状态           │
│  - 录制为 Vec<MacroStep>                │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│              MacroStep                  │
│  ├─ delay_ms: u64                       │
│  ├─ action: Action                      │
│  ├─ modifiers: ModifierState            │
│  └─ timestamp: Timestamp                │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│           MacroPlayer                   │
│  - 遍历 MacroStep                       │
│  - 重建修饰键状态                        │
│  - 执行延迟后调用对应处理器              │
└─────────────────────────────────────────┘
```

### 数据结构

```rust
// 宏步骤
pub struct MacroStep {
    pub delay_ms: u64,           // 延迟（毫秒）
    pub action: Action,          // 动作
    pub modifiers: ModifierState, // 录制时的修饰键状态
    pub timestamp: Timestamp,     // 时间戳
}

// 宏定义
pub struct Macro {
    pub name: String,             // 宏名称
    pub steps: Vec<MacroStep>,   // 步骤列表
    pub created_at: Option<String>, // 创建时间
    pub description: Option<String>, // 描述（可选）
}
```

### 架构说明

```
┌─────────────────────────────────────────┐
│           MacroRecorder                 │
│                                         │
│  输入事件 ──► is_modifier()?            │
│              │                         │
│         ┌──┴──┐                        │
│        是    否                       │
│         │     │                        │
│        跳过  from_input_event()        │
│              │                        │
│              ▼                        │
│      更新修饰键状态                     │
│      创建 MacroStep                   │
│      添加到录制缓冲                    │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│           simplify_delays()             │
│                                         │
│  合并连续的短延迟（< 50ms）             │
│  保留必要的延迟（>= 50ms）             │
│  生成最终的 Vec<MacroStep>             │
└─────────────────┬───────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────┐
│           MacroPlayer                  │
│                                         │
│  遍历 MacroStep:                      │
│  1. 执行 delay_ms 延迟                │
│  2. 重建修饰键状态                     │
│  3. 调用对应处理器:                    │
│     - Key → send_key_action()         │
│     - Mouse → send_mouse_action()     │
│     - Window → window_manager         │
│     - Launch → launcher               │
│     - System → system_control         │
│     - Sequence → 递归处理             │
│     - Delay → sleep                   │
└─────────────────────────────────────────┘
```

## 智能录制特性

### 1. 过滤单独修饰键

录制时自动跳过单独的 Ctrl/Shift/Alt/Win 键，只记录组合键。例如：

- 录制 `Ctrl+C`: 只记录 2 个步骤（Ctrl+C 的按下和释放），而不是 4 个
- 单独按 `Ctrl`: 完全跳过，不记录任何内容

实现方式：通过 `KeyEvent::is_modifier()` 方法判断是否为修饰键。

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

这对于回放时正确还原上下文非常重要。

### 3. 延迟优化

自动合并短延迟（< 50ms），只保留有意义的延迟：

```rust
const MIN_DELAY_MS: u64 = 50; // 最小延迟阈值
```

例如：
- 按下 A (0ms) → 释放 A (10ms) → 按下 B (90ms): 只在 A 和 B 之间插入一个 80ms 延迟
- 按下 A (0ms) → 释放 A (10ms) → **Delay(100ms)** → 按下 B: 保留 100ms 延迟

## 获取按键扫描码

如果你需要手动编写宏配置，可能需要知道特定按键的扫描码和虚拟键码。

### 常见按键参考表

| 按键 | 名称 | 扫描码 | 虚拟键码 |
|------|------|--------|----------|
| **修饰键** ||||
| Ctrl 左 | LCtrl / LControl | 0x1D | 0xA2 (VK_LCONTROL) |
| Ctrl 右 | RCtrl / RControl | 0xE01D | 0xA3 (VK_RCONTROL) |
| Shift 左 | LShift | 0x2A | 0xA0 (VK_LSHIFT) |
| Shift 右 | RShift | 0x36 | 0xA1 (VK_RSHIFT) |
| Alt 左 | LAlt | 0x38 | 0xA4 (VK_LMENU) |
| Alt 右 | RAlt | 0xE038 | 0xA5 (VK_RMENU) |
| Win 左 | LWin / LMeta | 0xE05B | 0x5B (VK_LWIN) |
| Win 右 | RWin / RMeta | 0xE05C | 0x5C (VK_RWIN) |
| **字母键** ||||
| A-Z | a-z | 0x1E-0x2C | 0x41-0x5A (65-90) |
| **数字键** ||||
| 0-9 | 0-9 | 0x0B-0x14 | 0x30-0x39 (48-57) |
| **功能键** ||||
| F1-F12 | f1-f12 | 0x3B-0x58 | 0x70-0x7B (112-123) |
| **导航键** ||||
| Enter | Enter / Return | 0x1C | 0x0D (13) |
| Space | Space | 0x39 | 0x20 (32) |
| Tab | Tab | 0x0F | 0x09 (9) |
| Backspace | Backspace / Back | 0x0E | 0x08 (8) |
| Escape | Escape / Esc | 0x01 | 0x1B (27) |
| Up | Up | 0x48 | 0x26 (38) |
| Down | Down | 0x50 | 0x28 (40) |
| Left | Left | 0x4B | 0x25 (37) |
| Right | Right | 0x4D | 0x27 (39) |
| Home | Home | 0x47 | 0x24 (36) |
| End | End | 0x4F | 0x23 (35) |
| PageUp | PageUp | 0x49 | 0x21 (33) |
| PageDown | PageDown | 0x51 | 0x22 (34) |
| Insert | Insert / Ins | 0x52 | 0x2D (45) |
| Delete | Delete / Del | 0x53 | 0x2E (46) |
| **特殊键** ||||
| CapsLock | CapsLock / Caps | 0x3A | 0x14 (20) |
| Grave | Grave / Backtick | 0x29 | 0xC0 (96) |

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
2. 绑定引用的宏必须存在于 `[macros]` 中
3. 配置验证时会检查绑定的有效性
4. 一个宏可以被多个触发键绑定
5. 删除宏时会同时删除相关绑定

### 绑定示例

```toml
[macros]
# 快速打开常用应用
"open-terminal" = [
    { delay_ms = 0, action = { Launch = { program = "wt.exe", args = [], working_dir = null, env_vars = [] } }, ... ],
]
"open-explorer" = [
    { delay_ms = 0, action = { Launch = { program = "explorer.exe", args = [], working_dir = null, env_vars = [] } }, ... ],
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
