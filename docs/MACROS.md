# wakem 宏系统文档

本文档详细介绍 wakem 的宏录制和回放系统。

## 目录

- [概述](#概述)
- [命令行使用](#命令行使用)
- [配置文件定义宏](#配置文件定义宏)
- [核心组件](#核心组件)
- [智能录制特性](#智能录制特性)
- [获取按键扫描码](#获取按键扫描码)

## 概述

宏系统允许用户录制和回放键盘/鼠标操作序列。你可以：

- 录制任意键盘和鼠标操作
- 通过快捷键触发录制的宏
- 在配置文件中精确定义宏步骤
- 支持所有动作类型（按键、鼠标、窗口管理、启动程序等）

## 命令行使用

### 录制宏

```bash
# 开始录制宏
wakem record my-macro
# 执行要录制的操作...
# 按 Ctrl+Shift+Esc 停止录制

# 停止录制
wakem stop-record
```

### 播放宏

```bash
# 播放宏
wakem play my-macro
```

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

# 宏触发键绑定
[macro_bindings]
"F1" = "open-terminal"
"Ctrl+Shift+V" = "copy-paste"
```

### MacroStep 字段说明

| 字段 | 类型 | 说明 |
|-----|------|------|
| `delay_ms` | u64 | 延迟（毫秒） |
| `action` | Action | 动作（Key/Mouse/Window/Launch/System/Delay/Sequence） |
| `modifiers` | ModifierState | 录制时的修饰键状态（ctrl/shift/alt/meta） |
| `timestamp` | u64 | 事件时间戳（用于分析） |

### 宏动作类型

宏系统复用 `Action` 枚举，支持所有动作类型：

| 动作类型 | 说明 |
|-----|------|
| `Key` | 按键动作（Press/Release/Click/TypeText/Combo） |
| `Mouse` | 鼠标动作（Move/ButtonDown/ButtonUp/ButtonClick/Wheel/HWheel） |
| `Window` | 窗口管理动作（Center/MoveToEdge/HalfScreen 等） |
| `Launch` | 启动程序 |
| `System` | 系统控制（音量/亮度） |
| `Sequence` | 动作序列（嵌套） |
| `Delay` | 延迟等待 |

## 核心组件

| 组件 | 文件 | 说明 |
|------|------|------|
| `MacroRecorder` | `src/types/macros.rs` | 录制输入事件，使用 `Action::from_input_event()` |
| `MacroPlayer` | `src/runtime/macro_player.rs` | 回放宏动作，支持修饰键状态重建 |
| `MacroStep` | `src/types/macros.rs` | 宏步骤结构，包含动作、修饰键、时间戳 |
| `Action` | `src/types/action.rs` | 统一的动作枚举 |

### 架构说明

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

## 智能录制特性

1. **过滤单独修饰键**: 录制时自动跳过单独的 Ctrl/Shift/Alt/Win 键，只记录组合键
2. **跟踪修饰键状态**: 记录每个动作发生时的修饰键状态
3. **简化录制结果**: 组合键（如 Ctrl+C）只记录为 2 个步骤，而不是 4 个

## 获取按键扫描码

如果你需要获取特定按键的扫描码，可以使用 `wakem daemon` 启动守护进程后查看日志，或使用在线工具查询。

常见按键扫描码参考：

| 按键 | 扫描码 | 虚拟键码 |
|------|--------|----------|
| `Ctrl` | 29 | 17 |
| `Shift` | 42 | 16 |
| `Alt` | 56 | 18 |
| `Win` | 91 | 91 |
| `A-Z` | 30-45 | 65-90 |
| `Enter` | 28 | 13 |
| `Space` | 57 | 32 |

---

更多配置信息请参考 [CONFIG.md](CONFIG.md)，开发相关信息请参考 [developer.md](developer.md)。
