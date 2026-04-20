# wakem 配置文档

## 配置文件位置

wakem 会在以下位置查找配置文件：

1. `%USERPROFILE%\wakem.toml` (Windows)
2. `%USERPROFILE%\.config\wakem\config.toml`
3. 当前工作目录下的 `wakem.toml`

## 配置文件结构

```toml
# 基本设置
log_level = "info"        # 日志级别: trace, debug, info, warn, error
tray_icon = true          # 是否显示系统托盘图标
auto_reload = true        # 是否自动重新加载配置
icon_path = "assets/icon.ico"  # 自定义托盘图标路径（可选）

[keyboard]
# 基础键位重映射
remap = [
    { from = "CapsLock", to = "Backspace" },
    { from = "RightAlt", to = "Ctrl" },
]

# 层配置
[[keyboard.layers]]
name = "navigation"
activation_key = "CapsLock"
mode = "Hold"  # Hold: 按住激活, Toggle: 切换激活
mappings = [
    { from = "H", to = "Left" },
    { from = "J", to = "Down" },
    { from = "K", to = "Up" },
    { from = "L", to = "Right" },
]

[window]
# 窗口管理快捷键
shortcuts = [
    { "Ctrl+Alt+C" = "Center" },
    { "Ctrl+Alt+Left" = "LoopWidth(Left)" },
]

[mouse]
# 鼠标设置（预留）
```

## 配置选项说明

### 全局设置

| 选项 | 类型 | 默认值 | 说明 |
|-----|------|-------|------|
| `log_level` | string | "info" | 日志级别 |
| `tray_icon` | bool | true | 显示系统托盘图标 |
| `auto_reload` | bool | true | 自动重新加载配置 |
| `icon_path` | string | "assets/icon.ico" | 自定义托盘图标路径 |

### 键盘配置 (`[keyboard]`)

#### 基础重映射 (`remap`)

格式: `{ from = "源按键", to = "目标按键" }`

```toml
[keyboard]
remap = [
    { from = "CapsLock", to = "Backspace" },
    { from = "RightAlt", to = "Ctrl" },
]
```

#### 层 (`layers`)

层允许你创建上下文相关的键位映射。

```toml
[[keyboard.layers]]
name = "navigation"           # 层名称
activation_key = "CapsLock"   # 激活键
mode = "Hold"                 # 模式: Hold 或 Toggle
mappings = [                  # 层内的映射
    { from = "H", to = "Left" },
    { from = "J", to = "Down" },
]
```

**模式说明**:
- `Hold`: 按住激活键时层激活，松开后恢复
- `Toggle`: 按一次激活，再按一次关闭

### 窗口管理配置 (`[window]`)

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+C" = "Center" },
    { "Ctrl+Alt+Home" = "MoveToEdge(Left)" },
    { "Alt+Grave" = "SwitchToNextWindow" },
]
```

支持的窗口管理动作:

| 动作 | 参数 | 说明 |
|-----|------|------|
| `Center` | 无 | 窗口居中 |
| `MoveToEdge` | `Left/Right/Top/Bottom` | 移动到屏幕边缘 |
| `HalfScreen` | `Left/Right/Top/Bottom` | 半屏显示 |
| `LoopWidth` | `Left/Right` | 循环调整宽度 |
| `LoopHeight` | `Top/Bottom` | 循环调整高度 |
| `FixedRatio` | `ratio, scale_index` | 固定比例窗口 |
| `NativeRatio` | `scale_index` | 原生比例窗口 |
| `SwitchToNextWindow` | 无 | 同进程窗口切换 |
| `MoveToMonitor` | `Next/Prev/Index` | 跨显示器移动 |
| `Minimize` | 无 | 最小化窗口 |
| `Maximize` | 无 | 最大化窗口 |
| `Close` | 无 | 关闭窗口 |

## 按键名称

### 字母键
`A` - `Z`

### 数字键
`0` - `9`

### 功能键
`F1` - `F24`

### 控制键
- `CapsLock`, `Caps`
- `Shift`, `LeftShift`, `RightShift`
- `Ctrl`, `Control`, `LeftCtrl`, `RightCtrl`
- `Alt`, `LeftAlt`, `RightAlt`
- `Win`, `Meta`, `Command`, `LeftWin`, `RightWin`

### 导航键
- `Up`, `Down`, `Left`, `Right`
- `Home`, `End`
- `PageUp`, `PageDown`
- `Insert`, `Delete`

### 其他键
- `Backspace`, `Back`
- `Enter`, `Return`
- `Tab`
- `Escape`, `Esc`
- `Space`
- `Grave`, `Backtick` (` 键)

## 修饰键语法

在快捷键中使用修饰键:

```toml
# 单修饰键
"Ctrl+C"           # Ctrl + C
"Alt+Tab"          # Alt + Tab
"Win+E"            # Win + E

# 多修饰键
"Ctrl+Alt+C"       # Ctrl + Alt + C
"Ctrl+Shift+Esc"   # Ctrl + Shift + Esc
"Ctrl+Alt+Shift+S" # Ctrl + Alt + Shift + S
```

## 示例配置

### 最小配置

```toml
[keyboard]
remap = [
    { from = "CapsLock", to = "Backspace" },
]
```

### Vim 风格导航

```toml
[[keyboard.layers]]
name = "vim"
activation_key = "CapsLock"
mode = "Hold"
mappings = [
    { from = "H", to = "Left" },
    { from = "J", to = "Down" },
    { from = "K", to = "Up" },
    { from = "L", to = "Right" },
]
```

### 窗口管理

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+C" = "Center" },
    { "Ctrl+Alt+Left" = "LoopWidth(Left)" },
    { "Ctrl+Alt+Right" = "LoopWidth(Right)" },
    { "Alt+Grave" = "SwitchToNextWindow" },
]
```
