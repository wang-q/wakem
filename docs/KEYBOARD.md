# wakem 键盘功能文档

## 概述

wakem 的键盘功能包括：

1. **基础重映射** - 简单按键替换
2. **层系统** - 上下文相关的键位映射
3. **修饰键支持** - Ctrl, Alt, Shift, Win 组合

## 基础重映射

最简单的功能，将一个按键映射到另一个按键。

### 配置示例

```toml
[keyboard]
remap = [
    { from = "CapsLock", to = "Backspace" },
    { from = "RightAlt", to = "Ctrl" },
]
```

### 常见用途

- **CapsLock 改为 Backspace**: 更符合人体工程学
- **RightAlt 改为 Ctrl**: 方便单手操作
- **交换 Esc 和 Grave**: Vim 用户常用

## 层系统

层允许你创建多套键位配置，通过特定按键切换。

### 概念

- **激活键**: 触发层切换的按键
- **模式**: Hold（按住）或 Toggle（切换）
- **映射**: 层内的键位映射规则

### Hold 模式

按住激活键时层激活，松开后恢复原始映射。

```toml
[[keyboard.layers]]
name = "navigation"
activation_key = "CapsLock"
mode = "Hold"
mappings = [
    { from = "H", to = "Left" },
    { from = "J", to = "Down" },
    { from = "K", to = "Up" },
    { from = "L", to = "Right" },
]
```

**使用场景**: Vim 风格导航

### Toggle 模式

按一次激活，再按一次关闭。

```toml
[[keyboard.layers]]
name = "numpad"
activation_key = "ScrollLock"
mode = "Toggle"
mappings = [
    { from = "U", to = "Numpad7" },
    { from = "I", to = "Numpad8" },
    { from = "O", to = "Numpad9" },
]
```

**使用场景**: 数字键盘模拟

## 多层配置

可以同时配置多个层：

```toml
# Vim 导航层
[[keyboard.layers]]
name = "vim_navigation"
activation_key = "CapsLock"
mode = "Hold"
mappings = [
    { from = "H", to = "Left" },
    { from = "J", to = "Down" },
    { from = "K", to = "Up" },
    { from = "L", to = "Right" },
]

# 数字层
[[keyboard.layers]]
name = "numpad"
activation_key = "RightAlt"
mode = "Hold"
mappings = [
    { from = "U", to = "Numpad7" },
    { from = "I", to = "Numpad8" },
    { from = "O", to = "Numpad9" },
    { from = "J", to = "Numpad4" },
    { from = "K", to = "Numpad5" },
    { from = "L", to = "Numpad6" },
]
```

## 高级层映射

层内可以映射组合键：

```toml
[[keyboard.layers]]
name = "window_management"
activation_key = "CapsLock"
mode = "Hold"
mappings = [
    # 窗口管理
    { from = "Q", to = "Ctrl+W" },      # 关闭标签
    { from = "T", to = "Ctrl+T" },      # 新建标签
    { from = "Tab", to = "Ctrl+Tab" },  # 切换标签
    
    # 文本编辑
    { from = "W", to = "Ctrl+Right" },  # 下一个单词
    { from = "B", to = "Ctrl+Left" },   # 上一个单词
    { from = "D", to = "Delete" },      # 删除字符
]
```

## 修饰键

### 支持的修饰键

- `Ctrl` / `Control`
- `Alt`
- `Shift`
- `Win` / `Meta` / `Command`

### 左右区分

- `LeftCtrl`, `RightCtrl`
- `LeftAlt`, `RightAlt`
- `LeftShift`, `RightShift`
- `LeftWin`, `RightWin`

### 快捷键语法

```toml
# 单修饰键
"Ctrl+C"
"Alt+F4"
"Win+E"

# 多修饰键
"Ctrl+Alt+Delete"
"Ctrl+Shift+Esc"
"Ctrl+Alt+Shift+S"
```

## 完整示例

### Vim 风格完整配置

```toml
[keyboard]
remap = [
    { from = "CapsLock", to = "Backspace" },
]

[[keyboard.layers]]
name = "vim"
activation_key = "CapsLock"
mode = "Hold"
mappings = [
    # 基本方向
    { from = "H", to = "Left" },
    { from = "J", to = "Down" },
    { from = "K", to = "Up" },
    { from = "L", to = "Right" },
    
    # 快速移动
    { from = "W", to = "Ctrl+Right" },
    { from = "B", to = "Ctrl+Left" },
    { from = "0", to = "Home" },
    { from = "4", to = "End" },
    { from = "G", to = "Ctrl+End" },
    { from = "U", to = "PageUp" },
    { from = "D", to = "PageDown" },
    
    # 文本编辑
    { from = "X", to = "Delete" },
    { from = "I", to = "Home" },
    { from = "A", to = "End" },
    { from = "O", to = "Ctrl+Enter" },
]
```

### 程序员专用配置

```toml
[keyboard]
remap = [
    # 更方便的符号输入
    { from = "RightAlt", to = "Ctrl" },
]

[[keyboard.layers]]
name = "symbols"
activation_key = "RightAlt"
mode = "Hold"
mappings = [
    # 常用符号
    { from = "1", to = "Exclamation" },  # !
    { from = "2", to = "At" },            # @
    { from = "3", to = "Hash" },          # #
    { from = "4", to = "Dollar" },        # $
    { from = "5", to = "Percent" },       # %
    
    # 括号
    { from = "9", to = "LeftParen" },     # (
    { from = "0", to = "RightParen" },    # )
    { from = "Minus", to = "Underscore" }, # _
    { from = "Equal", to = "Plus" },      # +
]
```

## 故障排除

### 层不生效

1. 检查激活键名称是否正确
2. 确认没有其他软件占用该按键
3. 查看日志确认层是否正确加载

### 映射冲突

层映射优先级高于基础重映射。如果冲突，层映射会覆盖基础映射。

### 系统键无法映射

某些系统键（如 Ctrl+Alt+Delete）受 Windows 保护，无法重映射。
