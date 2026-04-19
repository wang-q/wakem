# wakem 窗口管理文档

## 概述

wakem 的窗口管理功能借鉴了 [mrw](https://github.com/yourusername/mrw) 项目，提供高效的窗口操作快捷键。

## 功能列表

### 1. 窗口居中 (Center)

将当前窗口移动到屏幕中心。

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+C" = "Center" },
]
```

### 2. 移动到边缘 (MoveToEdge)

将窗口移动到屏幕边缘，保持当前大小。

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+Home" = "MoveToEdge(Left)" },
    { "Ctrl+Alt+End" = "MoveToEdge(Right)" },
    { "Ctrl+Alt+PageUp" = "MoveToEdge(Top)" },
    { "Ctrl+Alt+PageDown" = "MoveToEdge(Bottom)" },
]
```

### 3. 半屏显示 (HalfScreen)

将窗口调整为半屏大小。

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+Shift+Left" = "HalfScreen(Left)" },
    { "Ctrl+Alt+Shift+Right" = "HalfScreen(Right)" },
    { "Ctrl+Alt+Shift+Up" = "HalfScreen(Top)" },
    { "Ctrl+Alt+Shift+Down" = "HalfScreen(Bottom)" },
]
```

### 4. 循环调整尺寸 (LoopWidth/LoopHeight)

连续按键循环切换预设尺寸比例。

**宽度循环** (3/4 → 3/5 → 1/2 → 2/5 → 1/4):

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+Left" = "LoopWidth(Left)" },
    { "Ctrl+Alt+Right" = "LoopWidth(Right)" },
]
```

**高度循环** (3/4 → 1/2 → 1/4):

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+Up" = "LoopHeight(Top)" },
    { "Ctrl+Alt+Down" = "LoopHeight(Bottom)" },
]
```

### 5. 固定比例窗口 (FixedRatio)

保持特定宽高比，循环缩放。

```toml
[window]
shortcuts = [
    # 4:3 比例，从 100% 开始
    { "Ctrl+Alt+M" = "FixedRatio(1.333, 0)" },
]
```

**参数说明**:
- `ratio`: 宽高比（1.333 = 4:3）
- `scale_index`: 初始缩放索引（0 = 100%, 1 = 90%, 2 = 70%, 3 = 50%）

连续按键循环: 100% → 90% → 70% → 50% → 100%

### 6. 原生比例窗口 (NativeRatio)

基于屏幕宽高比计算基础尺寸。

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+Shift+M" = "NativeRatio(0)" },
]
```

### 7. 同进程窗口切换 (SwitchToNextWindow)

类似 Alt+Tab，但只在同一进程的窗口间切换。

```toml
[window]
shortcuts = [
    { "Alt+Grave" = "SwitchToNextWindow" },  # Alt+`
]
```

### 8. 跨显示器移动 (MoveToMonitor)

将窗口移动到另一个显示器。

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+J" = "MoveToMonitor(Next)" },
    { "Ctrl+Alt+K" = "MoveToMonitor(Prev)" },
]
```

**参数**:
- `Next`: 下一个显示器
- `Prev`: 上一个显示器
- `Index(n)`: 指定显示器索引

### 9. 窗口状态控制

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+N" = "Minimize" },   # 最小化
    { "Ctrl+Alt+X" = "Maximize" },   # 最大化
    { "Ctrl+Alt+R" = "Restore" },    # 还原
    { "Ctrl+Alt+Q" = "Close" },      # 关闭
]
```

## 完整配置示例

### mrw 风格配置

```toml
[window]
shortcuts = [
    # 居中
    { "Ctrl+Alt+C" = "Center" },
    
    # 移动到边缘
    { "Ctrl+Alt+Home" = "MoveToEdge(Left)" },
    { "Ctrl+Alt+End" = "MoveToEdge(Right)" },
    { "Ctrl+Alt+PageUp" = "MoveToEdge(Top)" },
    { "Ctrl+Alt+PageDown" = "MoveToEdge(Bottom)" },
    
    # 半屏
    { "Ctrl+Alt+Shift+Left" = "HalfScreen(Left)" },
    { "Ctrl+Alt+Shift+Right" = "HalfScreen(Right)" },
    { "Ctrl+Alt+Shift+Up" = "HalfScreen(Top)" },
    { "Ctrl+Alt+Shift+Down" = "HalfScreen(Bottom)" },
    
    # 循环调整
    { "Ctrl+Alt+Left" = "LoopWidth(Left)" },
    { "Ctrl+Alt+Right" = "LoopWidth(Right)" },
    { "Ctrl+Alt+Up" = "LoopHeight(Top)" },
    { "Ctrl+Alt+Down" = "LoopHeight(Bottom)" },
    
    # 固定比例
    { "Ctrl+Alt+M" = "FixedRatio(1.333, 0)" },
    { "Ctrl+Alt+Shift+M" = "NativeRatio(0)" },
    
    # 窗口切换
    { "Alt+Grave" = "SwitchToNextWindow" },
    
    # 跨显示器
    { "Ctrl+Alt+J" = "MoveToMonitor(Next)" },
    { "Ctrl+Alt+K" = "MoveToMonitor(Prev)" },
]
```

## 多显示器支持

wakem 支持多显示器环境：

1. **自动检测**: 自动识别所有连接的显示器
2. **工作区计算**: 考虑任务栏等占用，计算实际可用区域
3. **相对位置保持**: 跨显示器移动时保持相对位置和大小比例

## 快捷键建议

### 推荐方案 1: Hyper 键风格

使用 `Ctrl+Alt` 作为 Hyper 键：

```toml
[window]
shortcuts = [
    { "Ctrl+Alt+C" = "Center" },
    { "Ctrl+Alt+Left" = "LoopWidth(Left)" },
    { "Ctrl+Alt+Right" = "LoopWidth(Right)" },
]
```

### 推荐方案 2: Win 键风格

使用 `Win+Alt`：

```toml
[window]
shortcuts = [
    { "Win+Alt+C" = "Center" },
    { "Win+Alt+Left" = "HalfScreen(Left)" },
]
```

### 推荐方案 3: 层集成

将窗口管理集成到键盘层：

```toml
[[keyboard.layers]]
name = "window_management"
activation_key = "CapsLock"
mode = "Hold"
mappings = [
    { from = "C", to = "Window(Center)" },
    { from = "H", to = "Window(HalfScreen(Left))" },
    { from = "L", to = "Window(HalfScreen(Right))" },
]
```

## 故障排除

### 窗口不移动

1. 检查窗口是否被其他软件锁定
2. 确认窗口不是系统保护窗口（如任务管理器）
3. 查看日志确认命令是否正确发送

### 多显示器问题

1. 确保显示器已正确识别（查看日志）
2. 检查显示器 DPI 缩放设置
3. 尝试重新启动 wakemd

### 快捷键冲突

1. 检查是否有其他软件占用相同快捷键
2. 尝试更换快捷键组合
3. 使用更复杂的组合（如三键组合）
